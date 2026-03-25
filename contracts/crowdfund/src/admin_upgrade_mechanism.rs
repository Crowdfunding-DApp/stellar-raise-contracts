//! # admin_upgrade_mechanism
//!
//! @title   AdminUpgradeMechanism — Restricted WASM upgrade logic for the
//!          crowdfund contract.
//!
//! @notice  This module exposes a single entry point, `upgrade()`, that
//!          replaces the contract's on-chain WASM binary with a new version
//!          identified by its SHA-256 hash.  The call is gated behind a strict
//!          `admin.require_auth()` check — only the address stored as `Admin`
//!          during `initialize()` may invoke it.
//!
//! @dev     ## Centralized upgradeability — risks and mitigations
//!
//!          Upgradeable contracts introduce a trust assumption: whoever controls
//!          the admin key controls the contract logic.  Risks include:
//!
//!          1. **Key compromise** — If the admin's private key is stolen, an
//!             attacker can deploy arbitrary WASM, potentially draining all
//!             contributor funds or redirecting withdrawals.
//!             *Mitigation*: use a multisig or governance contract as admin,
//!             never a plain EOA.
//!
//!          2. **Malicious upgrade** — A rogue admin (or compromised multisig
//!             signer) could push WASM that removes refund logic or changes
//!             the withdrawal recipient.
//!             *Mitigation*: time-lock upgrades and require off-chain review
//!             before execution; publish the WASM source and verify the hash.
//!
//!          3. **Irreversibility** — Once `update_current_contract_wasm` is
//!             called the old WASM is gone.  There is no built-in rollback.
//!             *Mitigation*: test the new WASM on testnet, verify storage
//!             compatibility, and keep the old WASM hash for reference.
//!
//!          4. **Storage layout drift** — A new WASM that changes `DataKey`
//!             variants or storage types can corrupt existing state.
//!             *Mitigation*: treat storage layout as a public API; add new
//!             keys rather than changing existing ones.
//!
//! ## Upgrade flow
//!
//! ```text
//! 1. Build new WASM  →  cargo build --release --target wasm32-unknown-unknown
//! 2. Upload WASM     →  stellar contract install --wasm <file> --network testnet
//!                        returns <WASM_HASH> (32-byte hex)
//! 3. Call upgrade()  →  stellar contract invoke --id <CONTRACT> -- upgrade
//!                        --new_wasm_hash <WASM_HASH>
//!                        (must be signed by the admin key)
//! ```

use soroban_sdk::{BytesN, Env};

use crate::DataKey;

/// Upgrades the contract to a new WASM implementation — admin-only.
///
/// @notice  Replaces the running WASM binary in-place.  All contract storage
///          and the contract address are preserved across the upgrade.
///
/// @dev     Reads the admin address from instance storage (set during
///          `initialize()`).  Panics with an `unwrap()` failure if called
///          before `initialize()` — this is intentional: an uninitialized
///          contract has no admin, so no upgrade should be possible.
///
/// @param  env           The Soroban execution environment.
/// @param  new_wasm_hash SHA-256 hash of the new WASM binary, exactly 32 bytes.
///                       The binary must already be uploaded to the ledger via
///                       `stellar contract install` before this call.
///
/// ## Security assumptions
///
/// - `BytesN<32>` is enforced by the Soroban type system — the host rejects
///   any invocation that supplies a hash of the wrong length at the ABI layer,
///   before this function body is reached.
/// - The hash is opaque to this function; validity (i.e. the WASM exists on
///   the ledger) is checked by `update_current_contract_wasm` in the host.
///   An unknown hash causes a host-level trap, not a Rust panic.
/// - No integer arithmetic is performed, so overflow is impossible.
pub fn upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
    let admin: soroban_sdk::Address =
        env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
