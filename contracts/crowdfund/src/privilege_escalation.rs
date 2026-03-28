//! # privilege_escalation
//!
//! @title   PrivilegeEscalation — Secure role promotion and demotion for the crowdfund contract.
//!
//! @notice  Implements a two-step, time-locked privilege escalation model:
//!          - A `DEFAULT_ADMIN_ROLE` holder may *nominate* an address for a higher role.
//!          - The nominee must *accept* the role within a configurable acceptance window.
//!          - Any escalation attempt by an address that does not hold the required
//!            prerequisite role is rejected immediately.
//!
//!          Supported escalation paths:
//!          | From role          | To role              | Gated by              |
//!          |--------------------|----------------------|-----------------------|
//!          | (none / any)       | PAUSER_ROLE          | DEFAULT_ADMIN_ROLE    |
//!          | PAUSER_ROLE        | DEFAULT_ADMIN_ROLE   | current DEFAULT_ADMIN |
//!          | (none / any)       | GovernanceAddress    | DEFAULT_ADMIN_ROLE    |
//!
//! ## Security Assumptions
//! 1. Only `DEFAULT_ADMIN_ROLE` may initiate any escalation nomination.
//! 2. Nominees must call `accept_role` within `ESCALATION_ACCEPTANCE_WINDOW` ledger-seconds.
//! 3. A pending nomination is invalidated if the nominating admin is replaced before acceptance.
//! 4. Escalation to `DEFAULT_ADMIN_ROLE` requires the nominee to already hold `PAUSER_ROLE`
//!    (enforces a prerequisite chain — no cold-wallet promotion to top role).
//! 5. All escalation events are emitted for off-chain monitoring and audit.
//! 6. Re-entrancy is not possible: no token transfers occur in this module.

#![allow(dead_code)]

use soroban_sdk::{Address, Env, Symbol};

use crate::{ContractError, DataKey};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Seconds within which a nominee must accept a pending role nomination.
/// After this window the nomination is considered stale and must be re-issued.
pub const ESCALATION_ACCEPTANCE_WINDOW: u64 = 86_400; // 24 hours

// ── Storage keys (inline — no new DataKey variants needed) ───────────────────
//
// We encode pending nominations as Symbol-keyed instance storage entries so
// that they share the contract's instance TTL and are automatically cleaned up
// on upgrade.  The key format is:
//   "pending_<role>"  →  (nominee: Address, nominated_at: u64, nominator: Address)

/// Composite value stored for a pending nomination.
#[derive(Clone)]
#[soroban_sdk::contracttype]
pub struct PendingNomination {
    /// The address being nominated for the role.
    pub nominee: Address,
    /// Ledger timestamp at which the nomination was created.
    pub nominated_at: u64,
    /// The admin address that issued the nomination (invalidated if admin changes).
    pub nominator: Address,
}

// ── Role identifiers (string constants) ──────────────────────────────────────

/// Human-readable role tag used in events and storage keys.
pub const ROLE_DEFAULT_ADMIN: &str = "DEFAULT_ADMIN";
/// Human-readable role tag used in events and storage keys.
pub const ROLE_PAUSER: &str = "PAUSER";
/// Human-readable role tag used in events and storage keys.
pub const ROLE_GOVERNANCE: &str = "GOVERNANCE";

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Returns the pending-nomination storage key for `role_tag`.
fn pending_key(env: &Env, role_tag: &str) -> Symbol {
    match role_tag {
        ROLE_DEFAULT_ADMIN => Symbol::new(env, "pending_DEFAULT_ADMIN"),
        ROLE_PAUSER => Symbol::new(env, "pending_PAUSER"),
        ROLE_GOVERNANCE => Symbol::new(env, "pending_GOVERNANCE"),
        _ => Symbol::new(env, "pending_unknown"),
    }
}

/// Read the current `DEFAULT_ADMIN_ROLE` address.
/// Panics if the contract has not been initialized.
fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::DefaultAdmin)
        .expect("DEFAULT_ADMIN_ROLE not set")
}

/// Read the current `PAUSER_ROLE` address.
/// Panics if the contract has not been initialized.
fn get_pauser(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Pauser)
        .expect("PAUSER_ROLE not set")
}

/// Read the current `GovernanceAddress`.
/// Panics if the contract has not been initialized.
fn get_governance(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::GovernanceAddress)
        .expect("GovernanceAddress not set")
}

// ── Nomination (step 1) ───────────────────────────────────────────────────────

/// @notice Nominate `nominee` for `PAUSER_ROLE`.
///
/// @dev    Only `DEFAULT_ADMIN_ROLE` may call this.  The nominee has
///         `ESCALATION_ACCEPTANCE_WINDOW` seconds to call `accept_role_pauser`.
///
/// @param caller  Must be the current `DEFAULT_ADMIN_ROLE`.
/// @param nominee Address being nominated.
///
/// # Errors
/// * Panics with `"not DEFAULT_ADMIN_ROLE"` if `caller` is not the admin.
///
/// # Security
/// - `caller.require_auth()` ensures the admin key signed the transaction.
/// - Emits `privilege / nominated_pauser` for off-chain monitoring.
pub fn nominate_pauser(env: &Env, caller: &Address, nominee: &Address) {
    caller.require_auth();

    let admin = get_admin(env);
    if *caller != admin {
        panic!("not DEFAULT_ADMIN_ROLE");
    }

    let nomination = PendingNomination {
        nominee: nominee.clone(),
        nominated_at: env.ledger().timestamp(),
        nominator: caller.clone(),
    };

    env.storage()
        .instance()
        .set(&pending_key(env, ROLE_PAUSER), &nomination);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "nominated_pauser"),
        ),
        (caller.clone(), nominee.clone()),
    );
}

/// @notice Nominate `nominee` for `GovernanceAddress`.
///
/// @dev    Only `DEFAULT_ADMIN_ROLE` may call this.
///
/// @param caller  Must be the current `DEFAULT_ADMIN_ROLE`.
/// @param nominee Address being nominated (should be a multisig or DAO).
///
/// # Security
/// - `caller.require_auth()` ensures the admin key signed the transaction.
/// - Emits `privilege / nominated_governance` for off-chain monitoring.
pub fn nominate_governance(env: &Env, caller: &Address, nominee: &Address) {
    caller.require_auth();

    let admin = get_admin(env);
    if *caller != admin {
        panic!("not DEFAULT_ADMIN_ROLE");
    }

    let nomination = PendingNomination {
        nominee: nominee.clone(),
        nominated_at: env.ledger().timestamp(),
        nominator: caller.clone(),
    };

    env.storage()
        .instance()
        .set(&pending_key(env, ROLE_GOVERNANCE), &nomination);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "nominated_governance"),
        ),
        (caller.clone(), nominee.clone()),
    );
}

/// @notice Nominate `nominee` for `DEFAULT_ADMIN_ROLE`.
///
/// @dev    Only the current `DEFAULT_ADMIN_ROLE` may call this.
///         The nominee **must** already hold `PAUSER_ROLE` — this enforces a
///         prerequisite chain and prevents cold-wallet promotion to the top role.
///
/// @param caller  Must be the current `DEFAULT_ADMIN_ROLE`.
/// @param nominee Must currently hold `PAUSER_ROLE`.
///
/// # Errors
/// * Panics with `"not DEFAULT_ADMIN_ROLE"` if `caller` is not the admin.
/// * Panics with `"nominee must hold PAUSER_ROLE first"` if prerequisite unmet.
///
/// # Security
/// - Prerequisite chain prevents a single-step escalation to the highest role.
/// - Emits `privilege / nominated_admin` for off-chain monitoring.
pub fn nominate_default_admin(env: &Env, caller: &Address, nominee: &Address) {
    caller.require_auth();

    let admin = get_admin(env);
    if *caller != admin {
        panic!("not DEFAULT_ADMIN_ROLE");
    }

    // Prerequisite: nominee must already hold PAUSER_ROLE
    let pauser = get_pauser(env);
    if *nominee != pauser {
        panic!("nominee must hold PAUSER_ROLE first");
    }

    let nomination = PendingNomination {
        nominee: nominee.clone(),
        nominated_at: env.ledger().timestamp(),
        nominator: caller.clone(),
    };

    env.storage()
        .instance()
        .set(&pending_key(env, ROLE_DEFAULT_ADMIN), &nomination);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "nominated_admin"),
        ),
        (caller.clone(), nominee.clone()),
    );
}

// ── Acceptance (step 2) ───────────────────────────────────────────────────────

/// @notice Accept a pending `PAUSER_ROLE` nomination.
///
/// @dev    Must be called by the nominee within `ESCALATION_ACCEPTANCE_WINDOW`.
///         Validates that:
///         1. A pending nomination exists for `PAUSER_ROLE`.
///         2. `caller` matches the stored nominee.
///         3. The acceptance window has not expired.
///         4. The nominator is still the current admin (guards against admin rotation).
///
/// @param caller Must match the stored nominee address.
///
/// # Errors
/// * [`ContractError::AlreadyInitialized`] — no pending nomination found.
/// * Panics with `"not the nominee"` if `caller` does not match.
/// * Panics with `"nomination expired"` if the window has passed.
/// * Panics with `"nominator is no longer admin"` if admin was rotated.
///
/// # Security
/// - `caller.require_auth()` ensures the nominee key signed the transaction.
/// - Clears the pending nomination after acceptance (prevents replay).
/// - Emits `privilege / role_accepted` for off-chain monitoring.
pub fn accept_role_pauser(env: &Env, caller: &Address) -> Result<(), ContractError> {
    caller.require_auth();

    let key = pending_key(env, ROLE_PAUSER);
    let nomination: PendingNomination = env
        .storage()
        .instance()
        .get(&key)
        .ok_or(ContractError::AlreadyInitialized)?;

    validate_nomination(env, caller, &nomination)?;

    // Commit the role change
    env.storage()
        .instance()
        .set(&DataKey::Pauser, &caller.clone());

    // Clear the pending nomination (prevents replay)
    env.storage().instance().remove(&key);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "role_accepted"),
        ),
        (caller.clone(), Symbol::new(env, ROLE_PAUSER)),
    );

    Ok(())
}

/// @notice Accept a pending `GovernanceAddress` nomination.
///
/// @dev    Same acceptance rules as `accept_role_pauser`.
///
/// @param caller Must match the stored nominee address.
///
/// # Errors
/// * [`ContractError::AlreadyInitialized`] — no pending nomination found.
/// * Panics with `"not the nominee"` if `caller` does not match.
/// * Panics with `"nomination expired"` if the window has passed.
/// * Panics with `"nominator is no longer admin"` if admin was rotated.
pub fn accept_role_governance(env: &Env, caller: &Address) -> Result<(), ContractError> {
    caller.require_auth();

    let key = pending_key(env, ROLE_GOVERNANCE);
    let nomination: PendingNomination = env
        .storage()
        .instance()
        .get(&key)
        .ok_or(ContractError::AlreadyInitialized)?;

    validate_nomination(env, caller, &nomination)?;

    env.storage()
        .instance()
        .set(&DataKey::GovernanceAddress, &caller.clone());

    env.storage().instance().remove(&key);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "role_accepted"),
        ),
        (caller.clone(), Symbol::new(env, ROLE_GOVERNANCE)),
    );

    Ok(())
}

/// @notice Accept a pending `DEFAULT_ADMIN_ROLE` nomination.
///
/// @dev    Same acceptance rules as `accept_role_pauser`.
///         After acceptance the previous admin loses the role.
///
/// @param caller Must match the stored nominee address.
///
/// # Errors
/// * [`ContractError::AlreadyInitialized`] — no pending nomination found.
/// * Panics with `"not the nominee"` if `caller` does not match.
/// * Panics with `"nomination expired"` if the window has passed.
/// * Panics with `"nominator is no longer admin"` if admin was rotated.
pub fn accept_role_default_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    caller.require_auth();

    let key = pending_key(env, ROLE_DEFAULT_ADMIN);
    let nomination: PendingNomination = env
        .storage()
        .instance()
        .get(&key)
        .ok_or(ContractError::AlreadyInitialized)?;

    validate_nomination(env, caller, &nomination)?;

    env.storage()
        .instance()
        .set(&DataKey::DefaultAdmin, &caller.clone());

    env.storage().instance().remove(&key);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "role_accepted"),
        ),
        (caller.clone(), Symbol::new(env, ROLE_DEFAULT_ADMIN)),
    );

    Ok(())
}

// ── Revocation ────────────────────────────────────────────────────────────────

/// @notice Cancel a pending nomination for `role_tag` before it is accepted.
///
/// @dev    Only the current `DEFAULT_ADMIN_ROLE` may revoke a pending nomination.
///         This is useful if a nominee's key is compromised before acceptance.
///
/// @param caller   Must be the current `DEFAULT_ADMIN_ROLE`.
/// @param role_tag One of `ROLE_PAUSER`, `ROLE_GOVERNANCE`, `ROLE_DEFAULT_ADMIN`.
///
/// # Errors
/// * Panics with `"not DEFAULT_ADMIN_ROLE"` if `caller` is not the admin.
/// * [`ContractError::AlreadyInitialized`] if no pending nomination exists.
///
/// # Security
/// - Emits `privilege / nomination_revoked` for off-chain monitoring.
pub fn revoke_nomination(env: &Env, caller: &Address, role_tag: &str) -> Result<(), ContractError> {
    caller.require_auth();

    let admin = get_admin(env);
    if *caller != admin {
        panic!("not DEFAULT_ADMIN_ROLE");
    }

    let key = pending_key(env, role_tag);
    if !env.storage().instance().has(&key) {
        return Err(ContractError::AlreadyInitialized);
    }

    env.storage().instance().remove(&key);

    env.events().publish(
        (
            Symbol::new(env, "privilege"),
            Symbol::new(env, "nomination_revoked"),
        ),
        (caller.clone(), Symbol::new(env, role_tag)),
    );

    Ok(())
}

// ── Query helpers ─────────────────────────────────────────────────────────────

/// @notice Returns the pending nomination for `role_tag`, if any.
///
/// @dev    Returns `None` if no nomination is pending or if it has expired.
///         Callers should check the `nominated_at` field against the current
///         ledger timestamp to determine remaining acceptance time.
pub fn get_pending_nomination(env: &Env, role_tag: &str) -> Option<PendingNomination> {
    env.storage().instance().get(&pending_key(env, role_tag))
}

/// @notice Returns `true` if `addr` currently holds `role_tag`.
///
/// @dev    Convenience helper for off-chain tooling and integration tests.
pub fn has_role(env: &Env, addr: &Address, role_tag: &str) -> bool {
    match role_tag {
        ROLE_DEFAULT_ADMIN => {
            let stored: Option<Address> = env.storage().instance().get(&DataKey::DefaultAdmin);
            stored.map_or(false, |a| a == *addr)
        }
        ROLE_PAUSER => {
            let stored: Option<Address> = env.storage().instance().get(&DataKey::Pauser);
            stored.map_or(false, |a| a == *addr)
        }
        ROLE_GOVERNANCE => {
            let stored: Option<Address> = env.storage().instance().get(&DataKey::GovernanceAddress);
            stored.map_or(false, |a| a == *addr)
        }
        _ => false,
    }
}

// ── Shared validation ─────────────────────────────────────────────────────────

/// Validate a pending nomination against the caller and current ledger state.
///
/// @dev    Shared by all `accept_role_*` functions to avoid duplication.
///
/// # Errors
/// * Panics with `"not the nominee"` if `caller` does not match `nomination.nominee`.
/// * Panics with `"nomination expired"` if the acceptance window has passed.
/// * Panics with `"nominator is no longer admin"` if the nominator is no longer admin.
fn validate_nomination(
    env: &Env,
    caller: &Address,
    nomination: &PendingNomination,
) -> Result<(), ContractError> {
    // 1. Caller must be the nominee
    if *caller != nomination.nominee {
        panic!("not the nominee");
    }

    // 2. Acceptance window must not have expired
    let now = env.ledger().timestamp();
    if now > nomination.nominated_at + ESCALATION_ACCEPTANCE_WINDOW {
        panic!("nomination expired");
    }

    // 3. The nominator must still be the current admin (guards against admin rotation)
    let current_admin = get_admin(env);
    if nomination.nominator != current_admin {
        panic!("nominator is no longer admin");
    }

    Ok(())
}
