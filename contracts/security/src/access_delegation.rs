//! # access_delegation
//!
//! @notice  Access delegation primitives for security-sensitive contract flows.
//!          Enables a delegator account to authorize a delegate account to
//!          execute specific scoped actions until an expiry timestamp.
//!
//! @dev     This module is storage-agnostic and validates snapshots of
//!          delegation state. The integrating contract is responsible for
//!          persistence and auth checks on mutating operations.
//!
//! @custom:security-note  Delegation introduces authority transfer. Strict
//!          validation, bounded scope lists, expiry enforcement, and revocation
//!          checks are mandatory to avoid privilege escalation.

#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

/// @notice  Maximum number of scopes allowed per delegation.
/// @dev     Keeps validation and authorization checks bounded and predictable.
pub const MAX_SCOPE_COUNT: u32 = 16;

/// @notice  Maximum number of active delegations per delegator.
/// @dev     Helps protect against storage and iteration abuse.
pub const MAX_ACTIVE_DELEGATIONS_PER_DELEGATOR: u32 = 50;

/// @notice  Authorization outcome for delegated calls.
#[derive(Clone, Debug, PartialEq)]
pub enum DelegationCheck {
    /// Delegate is authorized for the requested scope.
    Authorized,
    /// Delegated call denied; reason explains the failure.
    Denied { reason: &'static str },
}

impl DelegationCheck {
    /// @notice  Returns `true` if delegation check authorized execution.
    pub fn is_authorized(&self) -> bool {
        matches!(self, DelegationCheck::Authorized)
    }

    /// @notice  Returns denial reason or empty string.
    pub fn reason(&self) -> &'static str {
        match self {
            DelegationCheck::Denied { reason } => reason,
            _ => "",
        }
    }
}

/// @notice  A delegation record for scoped, time-limited access.
///
/// @param delegator  Account that grants delegated authority.
/// @param delegate   Account allowed to act on delegator's behalf.
/// @param scopes     Allowed action scopes for this delegation.
/// @param expires_at Ledger timestamp after which delegation is invalid.
/// @param active     Whether delegation is currently active.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AccessDelegation {
    pub delegator: Address,
    pub delegate: Address,
    pub scopes: Vec<Symbol>,
    pub expires_at: u64,
    pub active: bool,
}

/// @notice  Validates input for creating a delegation.
/// @dev     Requires:
///          - distinct delegator and delegate
///          - non-empty scope list
///          - scope count <= MAX_SCOPE_COUNT
///          - no duplicate scopes
///          - `expires_at > now`
pub fn validate_delegation_input(
    delegator: &Address,
    delegate: &Address,
    scopes: &Vec<Symbol>,
    now: u64,
    expires_at: u64,
) -> Result<(), &'static str> {
    if *delegator == *delegate {
        return Err("self delegation is not allowed");
    }
    if scopes.is_empty() {
        return Err("delegation must include at least one scope");
    }
    if scopes.len() > MAX_SCOPE_COUNT {
        return Err("scope count exceeds MAX_SCOPE_COUNT");
    }
    if has_duplicate_scopes(scopes) {
        return Err("duplicate scope detected");
    }
    if now == 0 {
        return Err("current timestamp must be non-zero");
    }
    if expires_at <= now {
        return Err("delegation expiry must be in the future");
    }
    Ok(())
}

/// @notice  Returns true when a duplicate scope exists.
fn has_duplicate_scopes(scopes: &Vec<Symbol>) -> bool {
    for i in 0..scopes.len() {
        for j in (i + 1)..scopes.len() {
            if scopes.get(i) == scopes.get(j) {
                return true;
            }
        }
    }
    false
}

/// @notice  Returns true when `requested_scope` is in delegation scopes.
pub fn is_scope_allowed(scopes: &Vec<Symbol>, requested_scope: &Symbol) -> bool {
    for i in 0..scopes.len() {
        if let Some(scope) = scopes.get(i) {
            if scope == *requested_scope {
                return true;
            }
        }
    }
    false
}

/// @notice  Validates delegated execution for a specific caller and scope.
/// @dev     Denies when delegation is inactive, expired, caller mismatch, or
///          scope is not allowed.
pub fn can_delegate_call(
    delegation: &AccessDelegation,
    caller: &Address,
    requested_scope: &Symbol,
    now: u64,
) -> DelegationCheck {
    if !delegation.active {
        return DelegationCheck::Denied {
            reason: "delegation is inactive",
        };
    }
    if now == 0 {
        return DelegationCheck::Denied {
            reason: "current timestamp must be non-zero",
        };
    }
    if *caller != delegation.delegate {
        return DelegationCheck::Denied {
            reason: "caller is not delegated account",
        };
    }
    if now > delegation.expires_at {
        return DelegationCheck::Denied {
            reason: "delegation has expired",
        };
    }
    if !is_scope_allowed(&delegation.scopes, requested_scope) {
        return DelegationCheck::Denied {
            reason: "requested scope is not delegated",
        };
    }
    DelegationCheck::Authorized
}

/// @notice  Validates that a caller can revoke this delegation.
/// @dev     Only the delegator may revoke.
pub fn validate_revocation(delegation: &AccessDelegation, caller: &Address) -> Result<(), &'static str> {
    if *caller != delegation.delegator {
        return Err("only delegator may revoke delegation");
    }
    if !delegation.active {
        return Err("delegation already inactive");
    }
    Ok(())
}

/// @notice  Counts active, non-expired delegations for one delegator.
pub fn count_active_delegations_for(
    delegations: &Vec<AccessDelegation>,
    delegator: &Address,
    now: u64,
) -> u32 {
    let mut count: u32 = 0;
    for i in 0..delegations.len() {
        if let Some(d) = delegations.get(i) {
            if d.delegator == *delegator && d.active && now <= d.expires_at {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

/// @notice  Checks whether delegator can create another active delegation.
pub fn validate_delegation_capacity(
    delegations: &Vec<AccessDelegation>,
    delegator: &Address,
    now: u64,
) -> Result<(), &'static str> {
    let active = count_active_delegations_for(delegations, delegator, now);
    if active >= MAX_ACTIVE_DELEGATIONS_PER_DELEGATOR {
        return Err("active delegation limit reached for delegator");
    }
    Ok(())
}

/// @notice  Emits event for new delegation.
pub fn emit_delegation_created_event(
    env: &Env,
    delegator: &Address,
    delegate: &Address,
    expires_at: u64,
) {
    env.events().publish(
        (
            Symbol::new(env, "delegation"),
            Symbol::new(env, "created"),
        ),
        (delegator.clone(), delegate.clone(), expires_at),
    );
}

/// @notice  Emits event for delegation revocation.
pub fn emit_delegation_revoked_event(env: &Env, delegator: &Address, delegate: &Address) {
    env.events().publish(
        (
            Symbol::new(env, "delegation"),
            Symbol::new(env, "revoked"),
        ),
        (delegator.clone(), delegate.clone()),
    );
}
