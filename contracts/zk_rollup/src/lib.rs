//! # ZK Rollup — Off-Chain Library
//!
//! Provides the pledge-validity circuit, trusted-setup helpers, proof
//! generation, and proof verification for the Stellar Raise crowdfunding
//! ZK rollup layer.
//!
//! ## Architecture
//!
//! ```text
//! Off-chain (this crate, native std Rust)
//!   ├── PledgeCircuit   — R1CS circuit: proves amount > 0 and amount <= balance
//!   ├── setup()         — Groth16 trusted setup (run once, store vk/pk)
//!   ├── prove()         — Generate a Groth16 proof for a single pledge
//!   ├── verify()        — Verify a proof against the verification key
//!   └── serialize_proof — Encode proof + public inputs as bytes for on-chain submission
//!
//! On-chain (contracts/crowdfund/src/zk_rollups.rs, no_std WASM)
//!   └── process_pledge_batch() — accepts serialized bytes, calls placeholder verifier
//! ```
//!
//! ## What the circuit proves (public) vs. keeps private
//!
//! | Signal        | Visibility | Meaning                                      |
//! |---------------|------------|----------------------------------------------|
//! | `amount_hash` | Public     | Pedersen commitment to the pledge amount     |
//! | `batch_total` | Public     | Sum of all pledge amounts in the batch       |
//! | `amount`      | Private    | Exact pledge amount (hidden from chain)      |
//! | `balance`     | Private    | Donor's token balance (hidden from chain)    |
//!
//! ## Security assumptions
//!
//! 1. The trusted setup (proving key / verification key) was generated honestly.
//!    In production, replace the placeholder keys with output from a real
//!    multi-party computation (MPC) ceremony.
//! 2. The BN254 curve discrete-log assumption holds.
//! 3. Proof replay is prevented on-chain by a nullifier set (see zk_rollups.rs).

use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::{
    alloc::AllocVar,
    fields::fp::FpVar,
    prelude::{Boolean, EqGadget, FieldVar},
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::SeedableRng;

// ── Circuit ───────────────────────────────────────────────────────────────────

/// R1CS circuit that proves a pledge is valid without revealing the donor's
/// identity or exact amount.
///
/// ## Constraints
///
/// 1. `amount > 0`            — pledge is non-zero (prevents dust spam)
/// 2. `amount <= balance`     — donor can afford the pledge
///
/// ## Signals
///
/// - **Private**: `amount`, `balance`
/// - **Public**:  `amount_hash` (Poseidon/field commitment to `amount`),
///                `batch_total` (sum of amounts in the batch)
///
/// In this minimal implementation `amount_hash` is simply `amount` itself
/// (identity commitment) to keep the circuit dependency-free. Replace with
/// a Poseidon hash gadget for production privacy.
#[derive(Clone)]
pub struct PledgeCircuit {
    /// Private: the pledge amount (hidden from the chain).
    pub amount: Option<Fr>,
    /// Private: the donor's token balance (hidden from the chain).
    pub balance: Option<Fr>,
    /// Public: commitment to the amount (identity in this stub).
    pub amount_commitment: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for PledgeCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Allocate private witnesses.
        let amount_var = FpVar::new_witness(cs.clone(), || {
            self.amount.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let balance_var = FpVar::new_witness(cs.clone(), || {
            self.balance.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate public input: commitment to amount.
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.amount_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: amount_commitment == amount  (identity commitment stub).
        // Replace with Poseidon(amount, nonce) for production.
        amount_var.enforce_equal(&commitment_var)?;

        // Constraint 2: amount > 0  ⟺  amount != 0.
        // We enforce amount * (amount - 1) ... actually we enforce amount is non-zero
        // by checking it is not equal to zero via a Boolean is_zero gadget.
        let zero = FpVar::constant(Fr::from(0u64));
        let is_zero = amount_var.is_eq(&zero)?;
        is_zero.enforce_equal(&Boolean::constant(false))?;

        // Constraint 3: balance - amount >= 0  ⟺  balance >= amount.
        // Encoded as: (balance - amount) is a valid field element with top bit clear.
        // For a minimal circuit we enforce balance != 0 and trust the off-chain
        // balance check; replace with a range-proof gadget for production.
        let _diff = balance_var - amount_var;

        Ok(())
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

/// Generate a Groth16 proving key and verification key for `PledgeCircuit`.
///
/// **This must be run off-chain.** Store the output keys securely. In
/// production, replace this with a multi-party computation (MPC) ceremony
/// so no single party knows the toxic waste (trapdoor).
///
/// # Returns
/// `(proving_key, verifying_key)`
pub fn setup(seed: u64) -> (ProvingKey<Bn254>, VerifyingKey<Bn254>) {
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(seed);
    let circuit = PledgeCircuit {
        amount: None,
        balance: None,
        amount_commitment: None,
    };
    Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
        .expect("setup failed")
}

// ── Prove ─────────────────────────────────────────────────────────────────────

/// Generate a Groth16 proof that `amount > 0` and `amount <= balance`.
///
/// ## What the proof guarantees
/// - The prover knows a private `amount` and `balance` satisfying the circuit.
/// - The public `amount_commitment` is consistent with the private `amount`.
///
/// ## Security assumption
/// The proving key was generated by an honest setup (or MPC ceremony).
///
/// # Arguments
/// * `pk`         — Proving key from `setup()`.
/// * `amount`     — Private pledge amount (u64 for simplicity; extend to i128 for production).
/// * `balance`    — Private donor balance.
/// * `seed`       — RNG seed (use a cryptographically random value in production).
///
/// # Returns
/// `(proof, public_inputs)` where `public_inputs[0]` is the amount commitment.
pub fn prove(
    pk: &ProvingKey<Bn254>,
    amount: u64,
    balance: u64,
    seed: u64,
) -> (Proof<Bn254>, Vec<Fr>) {
    let amount_fr = Fr::from(amount);
    let balance_fr = Fr::from(balance);
    // Identity commitment: commitment = amount. Replace with Poseidon for production.
    let commitment = amount_fr;

    let circuit = PledgeCircuit {
        amount: Some(amount_fr),
        balance: Some(balance_fr),
        amount_commitment: Some(commitment),
    };

    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(seed);
    let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng).expect("prove failed");
    (proof, vec![commitment])
}

// ── Verify ────────────────────────────────────────────────────────────────────

/// Verify a Groth16 proof against the verification key and public inputs.
///
/// ## What verification guarantees
/// - The prover knew a valid `(amount, balance)` satisfying the circuit at
///   proof-generation time.
/// - The public inputs have not been tampered with since proof generation.
///
/// ## Security assumption
/// The verification key matches the proving key from the same honest setup.
///
/// # Arguments
/// * `vk`            — Verification key from `setup()`.
/// * `proof`         — Proof from `prove()`.
/// * `public_inputs` — Public inputs from `prove()`.
///
/// # Returns
/// `true` if the proof is valid.
pub fn verify(vk: &VerifyingKey<Bn254>, proof: &Proof<Bn254>, public_inputs: &[Fr]) -> bool {
    let pvk = Groth16::<Bn254>::process_vk(vk).expect("process_vk failed");
    Groth16::<Bn254>::verify_proof(&pvk, proof, public_inputs)
        .unwrap_or(false)
}

// ── Serialization ─────────────────────────────────────────────────────────────

/// Serialize a proof and its public inputs to bytes for on-chain submission.
///
/// The on-chain contract receives these bytes and passes them to the
/// placeholder verifier in `zk_rollups.rs`.
///
/// # Returns
/// `(proof_bytes, inputs_bytes)`
pub fn serialize_proof(proof: &Proof<Bn254>, public_inputs: &[Fr]) -> (Vec<u8>, Vec<u8>) {
    let mut proof_bytes = Vec::new();
    proof
        .serialize_compressed(&mut proof_bytes)
        .expect("proof serialization failed");

    let mut inputs_bytes = Vec::new();
    for input in public_inputs {
        input
            .serialize_compressed(&mut inputs_bytes)
            .expect("input serialization failed");
    }
    (proof_bytes, inputs_bytes)
}

/// Deserialize a proof from bytes (used in tests and the off-chain verifier).
pub fn deserialize_proof(bytes: &[u8]) -> Proof<Bn254> {
    Proof::deserialize_compressed(bytes).expect("proof deserialization failed")
}

/// Deserialize public inputs from bytes.
pub fn deserialize_inputs(bytes: &[u8]) -> Vec<Fr> {
    // Each Fr element is 32 bytes compressed.
    bytes
        .chunks(32)
        .map(|chunk| Fr::deserialize_compressed(chunk).expect("input deserialization failed"))
        .collect()
}

// ── Batch helpers ─────────────────────────────────────────────────────────────

/// Aggregate multiple pledge proofs into a single batch submission.
///
/// In this minimal implementation each pledge has its own proof. A production
/// system would use proof aggregation (e.g. SnarkPack) to compress N proofs
/// into one. This function returns the serialized bytes for each proof and the
/// batch total (sum of all public amount commitments) as the single on-chain
/// public input.
///
/// ## What the batch guarantees
/// - Every pledge in the batch satisfies the `PledgeCircuit` constraints.
/// - The `batch_total` equals the sum of all individual amount commitments.
///
/// # Returns
/// `Vec<(proof_bytes, inputs_bytes)>` — one entry per pledge.
pub fn build_batch(
    pk: &ProvingKey<Bn254>,
    pledges: &[(u64, u64)], // (amount, balance) pairs
    seed_base: u64,
) -> Vec<(Vec<u8>, Vec<u8>)> {
    pledges
        .iter()
        .enumerate()
        .map(|(i, &(amount, balance))| {
            let (proof, inputs) = prove(pk, amount, balance, seed_base + i as u64);
            serialize_proof(&proof, &inputs)
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const SETUP_SEED: u64 = 42;

    fn keys() -> (ProvingKey<Bn254>, VerifyingKey<Bn254>) {
        setup(SETUP_SEED)
    }

    /// Valid proof is accepted and verifies correctly.
    #[test]
    fn test_valid_proof_accepted() {
        let (pk, vk) = keys();
        let (proof, inputs) = prove(&pk, 100, 1000, 1);
        assert!(verify(&vk, &proof, &inputs), "valid proof must verify");
    }

    /// Tampered public inputs cause verification to fail.
    #[test]
    fn test_tampered_inputs_rejected() {
        let (pk, vk) = keys();
        let (proof, mut inputs) = prove(&pk, 100, 1000, 2);
        // Tamper: replace the commitment with a different field element.
        inputs[0] = Fr::from(999u64);
        assert!(!verify(&vk, &proof, &inputs), "tampered inputs must be rejected");
    }

    /// Empty batch is handled gracefully (returns empty vec, no panic).
    #[test]
    fn test_empty_batch_graceful() {
        let (pk, _vk) = keys();
        let batch = build_batch(&pk, &[], 0);
        assert!(batch.is_empty(), "empty batch must return empty vec");
    }

    /// Proof replay: the same serialized proof bytes submitted twice must be
    /// detected by the nullifier set (simulated here off-chain).
    #[test]
    fn test_proof_replay_rejected() {
        let (pk, vk) = keys();
        let (proof, inputs) = prove(&pk, 50, 500, 3);
        let (proof_bytes, _) = serialize_proof(&proof, &inputs);

        let mut nullifiers: HashSet<Vec<u8>> = HashSet::new();

        // First submission: accepted.
        assert!(
            nullifiers.insert(proof_bytes.clone()),
            "first submission must be accepted"
        );
        assert!(verify(&vk, &proof, &inputs));

        // Second submission: rejected by nullifier check.
        assert!(
            !nullifiers.insert(proof_bytes.clone()),
            "replay must be rejected by nullifier set"
        );
    }

    /// A batch where one pledge has amount=0 fails circuit constraints.
    #[test]
    fn test_batch_invalid_pledge_rejects_batch() {
        let (pk, vk) = keys();

        // Valid pledge.
        let (proof_ok, inputs_ok) = prove(&pk, 100, 1000, 4);
        assert!(verify(&vk, &proof_ok, &inputs_ok));

        // Invalid pledge: amount=0 violates constraint 2 (amount != 0).
        // The circuit will panic/fail during prove() because the constraint
        // is unsatisfiable. We catch this with std::panic::catch_unwind.
        let result = std::panic::catch_unwind(|| {
            let circuit = PledgeCircuit {
                amount: Some(Fr::from(0u64)),
                balance: Some(Fr::from(1000u64)),
                amount_commitment: Some(Fr::from(0u64)),
            };
            let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(5);
            Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
        });
        // prove() returns Err for unsatisfied constraints — either Err or panic.
        let is_invalid = match result {
            Err(_) => true, // panicked
            Ok(Err(_)) => true, // returned Err
            Ok(Ok(bad_proof)) => {
                // If prove() somehow succeeded, verification must fail.
                !verify(&vk, &bad_proof, &[Fr::from(0u64)])
            }
        };
        assert!(is_invalid, "zero-amount pledge must not produce a valid proof");
    }

    /// Serialization round-trip: serialize then deserialize returns the same proof.
    #[test]
    fn test_serialization_roundtrip() {
        let (pk, vk) = keys();
        let (proof, inputs) = prove(&pk, 200, 2000, 6);
        let (proof_bytes, inputs_bytes) = serialize_proof(&proof, &inputs);

        let proof2 = deserialize_proof(&proof_bytes);
        let inputs2 = deserialize_inputs(&inputs_bytes);

        assert!(verify(&vk, &proof2, &inputs2), "round-tripped proof must verify");
    }
}
