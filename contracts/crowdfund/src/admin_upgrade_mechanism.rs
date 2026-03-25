//! # admin_upgrade_mechanism
//!
//! Validation helpers and logging bounds for the admin upgrade mechanism.
//!
//! ## Overview
//!
//! The `upgrade` entry-point in `lib.rs` allows a designated admin to swap the
//! contract's WASM binary without changing its address or storage.  This module
//! adds the missing pieces that were absent from the original implementation:
//!
//! * **Admin registration** – `set_admin` stores the admin address during
//!   initialisation (or via a privileged one-time call) so `upgrade` has
//!   something to authenticate against.
//! * **Admin query** – `get_admin` lets off-chain tooling verify who the current
//!   admin is without reading raw storage.
//! * **Pre-upgrade validation** – `validate_upgrade` performs all security
//!   checks (admin set, auth, non-zero hash) and emits a structured event
//!   before the WASM swap happens.
//! * **Post-upgrade logging** – `log_upgrade` emits a post-upgrade event with
//!   the new WASM hash so indexers can track every upgrade on-chain.
//! * **Admin rotation** – `rotate_admin` lets the current admin hand off
//!   control to a new address atomically, with an event for auditability.
//!
//! ## Security Assumptions
//!
//! * Only the address stored under `DataKey::Admin` may call `validate_upgrade`
//!   or `rotate_admin`.  The caller is responsible for invoking
//!   `admin.require_auth()` before these helpers.
//! * `set_admin` is a one-time operation: it panics if an admin is already set,
//!   preventing silent privilege escalation.
//! * A zero WASM hash (`[0u8; 32]`) is rejected as a likely mistake.
//! * No new trust assumptions are introduced beyond those in `lib.rs`.

#![allow(dead_code)]

use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

use crate::DataKey;

// ── Error messages (compile-time constants) ───────────────────────────────────

const ERR_ADMIN_ALREADY_SET: &str = "admin already set";
const ERR_ADMIN_NOT_SET: &str = "admin not set";
const ERR_ZERO_WASM_HASH: &str = "wasm hash must not be zero";
const ERR_NOT_ADMIN: &str = "caller is not the admin";

// ── Admin registration ────────────────────────────────────────────────────────

/// Store the admin address for the first (and only) time.
///
/// # Panics
/// * If an admin address is already stored (`ERR_ADMIN_ALREADY_SET`).
///
/// # Security
/// Call this exactly once during contract initialisation.  Subsequent calls
/// are rejected to prevent privilege escalation.
pub fn set_admin(env: &Env, admin: &Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic!("{}", ERR_ADMIN_ALREADY_SET);
    }
    env.storage().instance().set(&DataKey::Admin, admin);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("set")),
        admin.clone(),
    );
}

/// Return the stored admin address.
///
/// # Panics
/// * If no admin has been set yet (`ERR_ADMIN_NOT_SET`).
pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("{}", ERR_ADMIN_NOT_SET))
}

// ── Pre-upgrade validation ────────────────────────────────────────────────────

/// Validate an upgrade request and emit a pre-upgrade event.
///
/// Performs the following checks in order:
/// 1. An admin address must be stored.
/// 2. The `new_wasm_hash` must not be all-zero bytes.
/// 3. The caller must be the stored admin (enforced via `require_auth`).
///
/// Emits `("upgrade", "pre_upgrade", new_wasm_hash)` on success so indexers
/// can detect pending upgrades before the WASM swap occurs.
///
/// # Arguments
/// * `env`           – The contract environment.
/// * `new_wasm_hash` – The SHA-256 hash of the replacement WASM binary.
///
/// # Panics
/// * `ERR_ADMIN_NOT_SET`   – No admin has been registered.
/// * `ERR_ZERO_WASM_HASH`  – The supplied hash is all zeros.
/// * `ERR_NOT_ADMIN`       – The stored admin address did not authorise the call.
pub fn validate_upgrade(env: &Env, new_wasm_hash: &BytesN<32>) {
    // 1. Admin must be registered.
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("{}", ERR_ADMIN_NOT_SET));

    // 2. Reject zero hash.
    if new_wasm_hash.to_array() == [0u8; 32] {
        panic!("{}", ERR_ZERO_WASM_HASH);
    }

    // 3. Require admin authorisation.
    admin.require_auth();

    // Emit pre-upgrade event for indexers.
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("pre")),
        new_wasm_hash.clone(),
    );
}

// ── Post-upgrade logging ──────────────────────────────────────────────────────

/// Emit a post-upgrade event after the WASM swap has completed.
///
/// Call this immediately after `env.deployer().update_current_contract_wasm()`
/// so on-chain logs reflect the completed upgrade.
///
/// # Arguments
/// * `env`           – The contract environment.
/// * `new_wasm_hash` – The hash that was just deployed.
pub fn log_upgrade(env: &Env, new_wasm_hash: &BytesN<32>) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("done")),
        new_wasm_hash.clone(),
    );
}

// ── Admin rotation ────────────────────────────────────────────────────────────

/// Atomically transfer admin rights to `new_admin`.
///
/// The current admin must authorise this call.  After rotation the old admin
/// address loses all upgrade privileges.
///
/// # Arguments
/// * `env`       – The contract environment.
/// * `new_admin` – The address that will become the new admin.
///
/// # Panics
/// * `ERR_ADMIN_NOT_SET` – No admin has been registered.
/// * Auth failure        – The current admin did not authorise the call.
pub fn rotate_admin(env: &Env, new_admin: &Address) {
    let current_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("{}", ERR_ADMIN_NOT_SET));

    current_admin.require_auth();

    env.storage().instance().set(&DataKey::Admin, new_admin);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("rotated")),
        (current_admin, new_admin.clone()),
    );
}

// ── Convenience predicate ─────────────────────────────────────────────────────

/// Returns `true` when an admin address has been registered.
#[inline]
pub fn admin_is_set(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Returns `true` when `candidate` matches the stored admin address.
///
/// Returns `false` (rather than panicking) when no admin is set, so callers
/// can use this as a guard without catching panics.
pub fn is_admin(env: &Env, candidate: &Address) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .map(|a| a == *candidate)
        .unwrap_or(false)
}

// ── Symbol helpers (avoids magic literals at call sites) ──────────────────────

/// Event topic symbol for upgrade events.
#[inline]
pub fn topic_upgrade(env: &Env) -> Symbol {
    Symbol::new(env, "upgrade")
}

/// Event topic symbol for admin events.
#[inline]
pub fn topic_admin(env: &Env) -> Symbol {
    Symbol::new(env, "admin")
}
