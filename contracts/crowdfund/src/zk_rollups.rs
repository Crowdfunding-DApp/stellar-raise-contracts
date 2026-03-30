//! # ZK Rollup — On-Chain Interface (Soroban / no_std)
//!
//! This module is the **on-chain half** of the ZK rollup layer. It runs inside
//! the Soroban WASM environment (`#![no_std]`) and therefore cannot use
//! `ark-groth16` or any `std`-dependent crate directly.
//!
//! ## Architecture
//!
//! ```text
//! Off-chain (contracts/zk_rollup/src/lib.rs — native std Rust)
//!   └── PledgeCircuit + Groth16 prove/verify (ark-groth16 + ark-bn254)
//!   └── serialize_proof() → (proof_bytes, inputs_bytes)
//!
//! On-chain (this file — no_std WASM)
//!   └── process_pledge_batch(proof_bytes, inputs_bytes, batch_total)
//!         ├── nullifier check  (replay protection)
//!         ├── verify_proof()   ← PLACEHOLDER: replace with no_std BN254 verifier
//!         └── apply batch_total to TotalRaised
//! ```
//!
//! ## What the ZK proof guarantees (when the verifier is real)
//!
//! - Every pledge in the batch has `amount > 0`.
//! - Every pledger had sufficient balance at proof-generation time.
//! - Donor identities and exact amounts are not revealed on-chain.
//! - `batch_total` equals the sum of all individual pledge amounts.
//!
//! ## Security assumptions
//!
//! 1. **Honest setup** — The verification key embedded here was produced by a
//!    trusted setup or MPC ceremony. A compromised setup breaks soundness.
//! 2. **BN254 hardness** — Security relies on the discrete-log assumption on
//!    the BN254 curve (~128-bit security).
//! 3. **Nullifier set** — Replay protection depends on the nullifier set stored
//!    in persistent contract storage. Clearing storage would allow replays.
//! 4. **Placeholder verifier** — The current `verify_proof` implementation
//!    always returns `true`. It MUST be replaced before production use.
//!    See the `TODO` comment in `verify_proof`.
//!
//! ## Gas efficiency
//!
//! Batching N pledges into one `process_pledge_batch` call reduces the number
//! of on-chain transactions from N to 1. The proof verification cost is
//! constant regardless of batch size (single Groth16 verify).

use soroban_sdk::{contracttype, Bytes, Env, Vec};

use crate::{ContractError, DataKey, Status};

// ── Storage key ───────────────────────────────────────────────────────────────

/// Storage key for the set of spent proof nullifiers (replay protection).
#[contracttype]
#[derive(Clone)]
pub enum ZkDataKey {
    /// Set of proof nullifiers already consumed. Stored as a Vec<Bytes>.
    Nullifiers,
}

// ── Proof verification (PLACEHOLDER) ─────────────────────────────────────────

/// Verify a serialized Groth16 proof against the embedded verification key.
///
/// ## What this function guarantees (when implemented)
/// - The prover knew private witnesses `(amount, balance)` satisfying the
///   `PledgeCircuit` constraints at proof-generation time.
/// - The `public_inputs` have not been tampered with.
///
/// ## Security assumption
/// The verification key bytes are correct and were produced by an honest setup.
///
/// ## ⚠️  PLACEHOLDER
/// This stub always returns `true`. Replace the body with a `no_std`-compatible
/// BN254 pairing check. Candidate libraries:
/// - `substrate-bn` (no_std, BN256 ≈ BN254)
/// - A hand-rolled Miller-loop verifier compiled to WASM
///
/// # Arguments
/// * `_proof_bytes`  — Compressed Groth16 proof (192 bytes for BN254).
/// * `_inputs_bytes` — Compressed public field elements (32 bytes each).
///
/// # Returns
/// `true` if the proof is valid (always `true` in this stub).
#[allow(unused_variables)]
fn verify_proof(proof_bytes: &Bytes, inputs_bytes: &Bytes) -> bool {
    // TODO: replace with a no_std BN254 Groth16 verifier.
    // Example call shape (substrate-bn style):
    //   let proof = Proof::from_bytes(proof_bytes)?;
    //   let vk    = VerifyingKey::from_bytes(VERIFICATION_KEY_BYTES)?;
    //   pairing_check(&proof, &vk, inputs_bytes)
    true
}

// ── Nullifier helpers ─────────────────────────────────────────────────────────

fn load_nullifiers(env: &Env) -> Vec<Bytes> {
    env.storage()
        .persistent()
        .get(&ZkDataKey::Nullifiers)
        .unwrap_or_else(|| Vec::new(env))
}

fn save_nullifiers(env: &Env, nullifiers: &Vec<Bytes>) {
    env.storage()
        .persistent()
        .set(&ZkDataKey::Nullifiers, nullifiers);
    env.storage()
        .persistent()
        .extend_ttl(&ZkDataKey::Nullifiers, 100, 100);
}

fn is_spent(nullifiers: &Vec<Bytes>, proof_bytes: &Bytes) -> bool {
    for n in nullifiers.iter() {
        if n == *proof_bytes {
            return true;
        }
    }
    false
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Process a batch of pledges verified by a single ZK proof.
///
/// ## What this function does
/// 1. Checks the campaign is still `Active` and the deadline has not passed.
/// 2. Checks the proof has not been submitted before (replay protection).
/// 3. Verifies the ZK proof against the public inputs.
/// 4. Adds `batch_total` to `TotalRaised` with overflow protection.
/// 5. Records the proof nullifier to prevent replay.
///
/// ## What the ZK proof guarantees
/// - Every pledge in the batch has `amount > 0`.
/// - Every pledger had sufficient balance at proof-generation time.
/// - Donor identities and exact amounts are not revealed on-chain.
/// - `batch_total` equals the sum of all individual pledge amounts.
///
/// ## Security assumption
/// `verify_proof` is a real BN254 Groth16 verifier (currently a placeholder —
/// see the `verify_proof` doc comment above).
///
/// # Arguments
/// * `proof_bytes`  — Serialized Groth16 proof from the off-chain prover.
/// * `inputs_bytes` — Serialized public inputs (amount commitments).
/// * `batch_total`  — Sum of all pledge amounts in the batch (public input).
///
/// # Errors
/// * `ContractError::CampaignNotActive`  — Campaign is not in `Active` state.
/// * `ContractError::CampaignEnded`      — Deadline has passed.
/// * `ContractError::Overflow`           — `TotalRaised + batch_total` overflows.
/// * Panics with `"invalid zk proof"`    — Proof verification failed.
/// * Panics with `"proof already used"`  — Replay attack detected.
pub fn process_pledge_batch(
    env: &Env,
    proof_bytes: Bytes,
    inputs_bytes: Bytes,
    batch_total: i128,
) -> Result<(), ContractError> {
    // Guard: campaign must be Active.
    let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
    if status != Status::Active {
        return Err(ContractError::CampaignNotActive);
    }

    // Guard: deadline must not have passed.
    let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
    if env.ledger().timestamp() > deadline {
        return Err(ContractError::CampaignEnded);
    }

    // Guard: batch_total must be positive.
    if batch_total <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    // Replay protection: reject if this proof was already submitted.
    let mut nullifiers = load_nullifiers(env);
    if is_spent(&nullifiers, &proof_bytes) {
        panic!("proof already used");
    }

    // Verify the ZK proof.
    if !verify_proof(&proof_bytes, &inputs_bytes) {
        panic!("invalid zk proof");
    }

    // Update TotalRaised with overflow protection.
    let total: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalRaised)
        .unwrap_or(0);
    let new_total = total.checked_add(batch_total).ok_or(ContractError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &new_total);

    // Record nullifier.
    nullifiers.push_back(proof_bytes.clone());
    save_nullifiers(env, &nullifiers);

    // Emit event.
    env.events()
        .publish(("zk_rollup", "batch_applied"), (batch_total, new_total));

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, Bytes, Env,
    };

    use crate::{CrowdfundContract, CrowdfundContractClient};

    fn setup_active_campaign(env: &Env) -> CrowdfundContractClient<'static> {
        env.mock_all_auths();
        let contract_id = env.register(CrowdfundContract, ());
        let client = CrowdfundContractClient::new(env, &contract_id);

        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin);
        let token_address = token_id.address();

        let creator = Address::generate(env);
        let admin = Address::generate(env);
        let deadline = env.ledger().timestamp() + 10_000;

        client.initialize(
            &admin,
            &creator,
            &token_address,
            &1_000i128,
            &deadline,
            &1i128,
            &None,
            &None,
            &None,
            &None,
        );
        client
    }

    fn dummy_bytes(env: &Env, val: u8) -> Bytes {
        Bytes::from_slice(env, &[val; 32])
    }

    /// Valid proof (stub always returns true) is accepted and TotalRaised increases.
    #[test]
    fn test_valid_batch_applied() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_active_campaign(&env);

        let proof = dummy_bytes(&env, 0xAB);
        let inputs = dummy_bytes(&env, 0x01);

        env.as_contract(&client.address, || {
            let result = process_pledge_batch(&env, proof, inputs, 500);
            assert!(result.is_ok());
            let total: i128 = env
                .storage()
                .instance()
                .get(&DataKey::TotalRaised)
                .unwrap_or(0);
            assert_eq!(total, 500);
        });
    }

    /// Replay: submitting the same proof bytes twice panics.
    #[test]
    #[should_panic(expected = "proof already used")]
    fn test_replay_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_active_campaign(&env);

        let proof = dummy_bytes(&env, 0xCC);
        let inputs = dummy_bytes(&env, 0x02);

        env.as_contract(&client.address, || {
            process_pledge_batch(&env, proof.clone(), inputs.clone(), 100).unwrap();
            // Second call with same proof must panic.
            process_pledge_batch(&env, proof, inputs, 100).unwrap();
        });
    }

    /// Zero batch_total is rejected with ZeroAmount error.
    #[test]
    fn test_zero_batch_total_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_active_campaign(&env);

        env.as_contract(&client.address, || {
            let result = process_pledge_batch(
                &env,
                dummy_bytes(&env, 0x01),
                dummy_bytes(&env, 0x02),
                0,
            );
            assert_eq!(result, Err(ContractError::ZeroAmount));
        });
    }

    /// Expired campaign rejects batch.
    #[test]
    fn test_expired_campaign_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_active_campaign(&env);

        // Advance ledger past deadline.
        env.ledger().with_mut(|l| l.timestamp = 999_999_999);

        env.as_contract(&client.address, || {
            let result = process_pledge_batch(
                &env,
                dummy_bytes(&env, 0x03),
                dummy_bytes(&env, 0x04),
                100,
            );
            assert_eq!(result, Err(ContractError::CampaignEnded));
        });
    }
}
