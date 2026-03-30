# ZK Rollup Layer

## Why ZK rollups?

Two problems with naive on-chain pledge processing:

1. **Privacy** — every `contribute()` call reveals the donor's address and exact amount on-chain. For large campaigns this leaks competitive intelligence and exposes donors to targeted attacks.
2. **Gas / resource cost** — N pledges = N transactions = N sets of storage reads/writes and token transfers. Soroban charges per-operation; batching reduces cost proportionally.

A ZK rollup solves both: donors generate a proof off-chain that their pledge is valid (amount > 0, balance sufficient) without revealing the amount or identity. The contract verifies one proof and applies the entire batch in a single call.

## Architecture

```
Off-chain (contracts/zk_rollup/ — native std Rust, never compiled to WASM)
  ├── PledgeCircuit   R1CS circuit: proves amount > 0 and amount <= balance
  ├── setup()         Groth16 trusted setup → (proving_key, verifying_key)
  ├── prove()         Generate proof for one pledge
  ├── verify()        Verify proof off-chain (for testing)
  ├── serialize_proof Encode proof + public inputs as bytes
  └── build_batch     Aggregate N pledges into N (proof, inputs) pairs

On-chain (contracts/crowdfund/src/zk_rollups.rs — no_std WASM)
  └── process_pledge_batch(proof_bytes, inputs_bytes, batch_total)
        ├── campaign Active + deadline check
        ├── nullifier check  (replay protection)
        ├── verify_proof()   ← PLACEHOLDER — replace before production
        └── TotalRaised += batch_total
```

### Why two separate crates?

`ark-groth16` and `ark-bn254` depend on `std` (heap allocation, OS randomness, floating-point). Soroban contracts compile to `#![no_std]` WASM. These are fundamentally incompatible. Proof **generation** always happens off-chain; only proof **verification** runs on-chain, and only with a purpose-built `no_std` verifier.

## Circuit description

| Signal | Visibility | Meaning |
|---|---|---|
| `amount` | **Private** | Exact pledge amount — never revealed on-chain |
| `balance` | **Private** | Donor's token balance — never revealed on-chain |
| `amount_commitment` | **Public** | Commitment to `amount` (identity stub; use Poseidon in production) |

### Constraints

1. `amount_commitment == amount` — commitment is consistent with the private amount
2. `amount != 0` — pledge is non-zero (prevents dust spam)
3. `balance >= amount` — donor can afford the pledge (enforced off-chain in this stub; add a range-proof gadget for production)

### What remains private

- Donor address
- Exact pledge amount
- Donor balance

### What is public (on-chain)

- `batch_total` — sum of all pledge amounts in the batch
- Proof bytes (opaque to observers without the verification key)

## How to generate a proof

```bash
# 1. Build the off-chain crate
cargo build --package zk-rollup

# 2. Run the tests (includes proof generation + verification)
cargo test --package zk-rollup

# 3. Generate a proof programmatically (in your off-chain service)
use zk_rollup::{setup, prove, serialize_proof};

let (pk, vk) = setup(/* seed */ 42);
let (proof, inputs) = prove(&pk, amount_u64, balance_u64, rng_seed);
let (proof_bytes, inputs_bytes) = serialize_proof(&proof, &inputs);

// Submit to the on-chain contract:
// client.process_pledge_batch(&proof_bytes, &inputs_bytes, &batch_total);
```

## How to run the tests

```bash
# Off-chain circuit tests (ark-groth16, native Rust)
cargo test --package zk-rollup

# On-chain stub tests (Soroban testutils, no_std)
cargo test --package crowdfund zk_rollups

# All tests
cargo test --workspace
```

## Security assumptions

| Assumption | Risk if violated |
|---|---|
| Honest trusted setup (or MPC ceremony) | Prover can forge proofs for invalid pledges |
| BN254 discrete-log hardness (~128-bit) | Proof forgery |
| `verify_proof` is a real BN254 verifier | **Currently a stub — always returns true. Any proof is accepted.** |
| Nullifier set is never cleared | Replay attacks become possible |
| `batch_total` matches sum of private amounts | Prover could over-report total (mitigated by circuit when verifier is real) |

## Known limitations

- **`verify_proof` is a placeholder** — it always returns `true`. The on-chain verifier must be replaced with a `no_std`-compatible BN254 Groth16 verifier (e.g. `substrate-bn`) before production deployment.
- **Identity commitment** — `amount_commitment = amount` (no hiding). Replace with `Poseidon(amount, nonce)` for real privacy.
- **No proof aggregation** — each pledge has its own proof. Use SnarkPack or similar to compress N proofs into one for maximum gas savings.
- **Trusted setup seed is hardcoded** — the test setup uses `seed=42`. A production deployment requires a real MPC ceremony; the resulting keys must replace the placeholder.
- **No donor address binding** — the circuit does not bind the proof to a specific donor address. Add `address` as a public input to prevent proof theft.

## Gas comparison (placeholder values)

| Operation | Without rollup | With rollup | Notes |
|---|---|---|---|
| Single pledge | ~50 000 ops | ~50 000 ops | No change for N=1 |
| 10-pledge batch | ~500 000 ops | ~55 000 ops | 1 verify + 1 storage write |
| 100-pledge batch | ~5 000 000 ops | ~60 000 ops | Constant verify cost |
| Proof generation | — | Off-chain only | No on-chain cost |

*Values are illustrative. Replace with real Soroban CPU instruction benchmarks once the production verifier is implemented.*

## Placeholders that must be replaced before production

| Location | Placeholder | What to replace with |
|---|---|---|
| `zk_rollups.rs::verify_proof` | Always returns `true` | `no_std` BN254 Groth16 verifier |
| `lib.rs::PledgeCircuit` | Identity commitment | `Poseidon(amount, nonce)` gadget |
| `lib.rs::setup(seed)` | Deterministic seed | Real MPC ceremony output |
| `lib.rs` constraint 3 | Off-chain balance check | On-circuit range proof gadget |
| `zk_rollups.rs` | No donor address binding | Add `address` as public input |
