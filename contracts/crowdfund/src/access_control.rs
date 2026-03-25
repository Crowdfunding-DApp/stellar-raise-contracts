//! # access_control
//!
//! Role-based access control (RBAC) with Pausable logic for the crowdfund
//! contract.
//!
//! ## Roles
//!
//! | Role constant          | Who holds it          | What it gates                          |
//! |------------------------|-----------------------|----------------------------------------|
//! | `ROLE_DEFAULT_ADMIN`   | Multi-sig / DAO       | Grant/revoke roles, set platform fees  |
//! | `ROLE_CAMPAIGN_CREATOR`| Campaign creator EOA  | Campaign lifecycle (contribute, cancel)|
//! | `ROLE_PAUSER`          | Security key / DAO    | Pause and unpause the contract         |
//!
//! ## Design
//!
//! * Roles are stored as `DataKey::Role(role_id, address) → bool` in persistent
//!   storage so they survive ledger TTL extensions.
//! * The paused flag lives in instance storage for cheap reads on every call.
//! * Platform-fee changes require the `ROLE_DEFAULT_ADMIN` role, which should
//!   be held by a multi-sig or DAO governance address — never a single EOA.
//! * All mutations emit structured events for on-chain auditability.
//!
//! ## Security Assumptions
//!
//! * `ROLE_DEFAULT_ADMIN` must be assigned to a multi-sig or DAO address at
//!   deploy time.  Assigning it to a single EOA defeats the purpose.
//! * Callers are responsible for invoking `require_auth` on the acting address
//!   before calling mutating helpers (enforced inside each function here).
//! * Revoking `ROLE_DEFAULT_ADMIN` from the last holder locks the contract
//!   permanently — callers must ensure at least one admin remains.

#![allow(dead_code)]

use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ── Role identifiers ──────────────────────────────────────────────────────────

/// Governs role management and platform-fee changes.
/// Must be held by a multi-sig or DAO governance address.
pub const ROLE_DEFAULT_ADMIN: u32 = 0;

/// Governs campaign lifecycle operations (contribute, cancel, update_metadata).
pub const ROLE_CAMPAIGN_CREATOR: u32 = 1;

/// Governs emergency pause / unpause.
pub const ROLE_PAUSER: u32 = 2;

// ── Storage key ───────────────────────────────────────────────────────────────

/// Composite storage key: `(role_id, address) → bool`.
///
/// Stored in persistent storage so role assignments survive TTL expiry.
#[contracttype]
#[derive(Clone)]
pub struct RoleKey {
    pub role: u32,
    pub account: Address,
}

/// Instance-storage key for the paused flag.
#[contracttype]
#[derive(Clone)]
pub enum AccessKey {
    /// `bool` — whether the contract is currently paused.
    Paused,
    /// `Address` — the governance address authorised to set platform fees.
    FeeGovernor,
}

// ── Role helpers ──────────────────────────────────────────────────────────────

/// Grant `role` to `account`.
///
/// Only the `ROLE_DEFAULT_ADMIN` holder may call this.
///
/// # Panics
/// * If `granter` does not hold `ROLE_DEFAULT_ADMIN`.
pub fn grant_role(env: &Env, granter: &Address, role: u32, account: &Address) {
    require_role(env, granter, ROLE_DEFAULT_ADMIN);
    granter.require_auth();

    let key = RoleKey { role, account: account.clone() };
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(&key, 100, 100);

    env.events().publish(
        (symbol_short!("role"), symbol_short!("granted")),
        (role, account.clone(), granter.clone()),
    );
}

/// Revoke `role` from `account`.
///
/// Only the `ROLE_DEFAULT_ADMIN` holder may call this.
///
/// # Panics
/// * If `revoker` does not hold `ROLE_DEFAULT_ADMIN`.
pub fn revoke_role(env: &Env, revoker: &Address, role: u32, account: &Address) {
    require_role(env, revoker, ROLE_DEFAULT_ADMIN);
    revoker.require_auth();

    let key = RoleKey { role, account: account.clone() };
    env.storage().persistent().set(&key, &false);

    env.events().publish(
        (symbol_short!("role"), symbol_short!("revoked")),
        (role, account.clone(), revoker.clone()),
    );
}

/// Returns `true` when `account` holds `role`.
pub fn has_role(env: &Env, account: &Address, role: u32) -> bool {
    let key = RoleKey { role, account: account.clone() };
    env.storage()
        .persistent()
        .get::<RoleKey, bool>(&key)
        .unwrap_or(false)
}

/// Panics with `"missing role"` when `account` does not hold `role`.
///
/// Use this as a guard at the top of privileged functions.
pub fn require_role(env: &Env, account: &Address, role: u32) {
    if !has_role(env, account, role) {
        panic!("missing role");
    }
}

/// Bootstrap: assign `ROLE_DEFAULT_ADMIN` to `account` without any prior
/// admin check.
///
/// This is a one-time operation — it panics if a default admin already exists
/// to prevent privilege escalation after deployment.
///
/// # Panics
/// * If a default admin has already been bootstrapped.
pub fn bootstrap_admin(env: &Env, account: &Address) {
    // Use a sentinel key to track whether bootstrap has run.
    #[contracttype]
    #[derive(Clone)]
    enum BootstrapKey { Done }

    if env.storage().instance().has(&BootstrapKey::Done) {
        panic!("admin already bootstrapped");
    }
    env.storage().instance().set(&BootstrapKey::Done, &true);

    let key = RoleKey { role: ROLE_DEFAULT_ADMIN, account: account.clone() };
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(&key, 100, 100);

    env.events().publish(
        (symbol_short!("role"), symbol_short!("bootstrap")),
        account.clone(),
    );
}

// ── Pausable logic ────────────────────────────────────────────────────────────

/// Pause the contract.
///
/// Only an address holding `ROLE_PAUSER` may call this.
///
/// # Panics
/// * If `pauser` does not hold `ROLE_PAUSER`.
/// * If the contract is already paused.
pub fn pause(env: &Env, pauser: &Address) {
    require_role(env, pauser, ROLE_PAUSER);
    pauser.require_auth();

    if is_paused(env) {
        panic!("already paused");
    }

    env.storage().instance().set(&AccessKey::Paused, &true);

    env.events().publish(
        (symbol_short!("contract"), symbol_short!("paused")),
        pauser.clone(),
    );
}

/// Unpause the contract.
///
/// Only an address holding `ROLE_PAUSER` may call this.
///
/// # Panics
/// * If `pauser` does not hold `ROLE_PAUSER`.
/// * If the contract is not currently paused.
pub fn unpause(env: &Env, pauser: &Address) {
    require_role(env, pauser, ROLE_PAUSER);
    pauser.require_auth();

    if !is_paused(env) {
        panic!("not paused");
    }

    env.storage().instance().set(&AccessKey::Paused, &false);

    env.events().publish(
        (symbol_short!("contract"), symbol_short!("unpaused")),
        pauser.clone(),
    );
}

/// Returns `true` when the contract is paused.
#[inline]
pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&AccessKey::Paused)
        .unwrap_or(false)
}

/// Panics with `"contract paused"` when the contract is paused.
///
/// Insert at the top of any state-mutating entry-point.
#[inline]
pub fn require_not_paused(env: &Env) {
    if is_paused(env) {
        panic!("contract paused");
    }
}

// ── Platform-fee governance ───────────────────────────────────────────────────

/// Register the governance address authorised to set platform fees.
///
/// Only the `ROLE_DEFAULT_ADMIN` holder may call this.
/// The `governor` address should be a multi-sig or DAO contract.
///
/// # Panics
/// * If `admin` does not hold `ROLE_DEFAULT_ADMIN`.
pub fn set_fee_governor(env: &Env, admin: &Address, governor: &Address) {
    require_role(env, admin, ROLE_DEFAULT_ADMIN);
    admin.require_auth();

    env.storage().instance().set(&AccessKey::FeeGovernor, governor);

    env.events().publish(
        (symbol_short!("fee"), symbol_short!("governor")),
        (governor.clone(), admin.clone()),
    );
}

/// Returns the registered fee governor address, if any.
pub fn get_fee_governor(env: &Env) -> Option<Address> {
    env.storage().instance().get(&AccessKey::FeeGovernor)
}

/// Validate that `caller` is the registered fee governor before a fee change.
///
/// # Panics
/// * If no fee governor has been set.
/// * If `caller` is not the fee governor.
pub fn require_fee_governor(env: &Env, caller: &Address) {
    let governor: Address = env
        .storage()
        .instance()
        .get(&AccessKey::FeeGovernor)
        .unwrap_or_else(|| panic!("fee governor not set"));

    if *caller != governor {
        panic!("caller is not fee governor");
    }

    caller.require_auth();
}
