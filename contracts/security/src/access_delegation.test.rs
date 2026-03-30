#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};

use crate::access_delegation::{
    can_delegate_call, count_active_delegations_for, emit_delegation_created_event,
    emit_delegation_revoked_event, is_scope_allowed, validate_delegation_capacity,
    validate_delegation_input, validate_revocation, AccessDelegation, DelegationCheck,
    MAX_ACTIVE_DELEGATIONS_PER_DELEGATOR, MAX_SCOPE_COUNT,
};

fn env() -> Env {
    let e = Env::default();
    e.mock_all_auths();
    e
}

fn scope(env: &Env, s: &str) -> Symbol {
    Symbol::new(env, s)
}

fn scope_vec(env: &Env, items: &[&str]) -> Vec<Symbol> {
    let mut out = Vec::new(env);
    for item in items {
        out.push_back(scope(env, item));
    }
    out
}

fn delegation(env: &Env, now: u64, active: bool, scopes: Vec<Symbol>) -> AccessDelegation {
    AccessDelegation {
        delegator: Address::generate(env),
        delegate: Address::generate(env),
        scopes,
        expires_at: now + 1_000,
        active,
    }
}

#[test]
fn test_validate_delegation_input_ok() {
    let env = env();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);
    let scopes = scope_vec(&env, &["withdraw", "pause"]);
    assert!(validate_delegation_input(&delegator, &delegate, &scopes, 100, 200).is_ok());
}

#[test]
fn test_validate_delegation_input_rejects_self_delegation() {
    let env = env();
    let account = Address::generate(&env);
    let scopes = scope_vec(&env, &["withdraw"]);
    let err = validate_delegation_input(&account, &account, &scopes, 100, 200).unwrap_err();
    assert!(err.contains("self delegation"));
}

#[test]
fn test_validate_delegation_input_rejects_empty_scope_set() {
    let env = env();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);
    let scopes: Vec<Symbol> = Vec::new(&env);
    let err = validate_delegation_input(&delegator, &delegate, &scopes, 100, 200).unwrap_err();
    assert!(err.contains("at least one scope"));
}

#[test]
fn test_validate_delegation_input_rejects_duplicate_scope() {
    let env = env();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);
    let mut scopes = Vec::new(&env);
    let s = scope(&env, "withdraw");
    scopes.push_back(s.clone());
    scopes.push_back(s);
    let err = validate_delegation_input(&delegator, &delegate, &scopes, 100, 200).unwrap_err();
    assert!(err.contains("duplicate scope"));
}

#[test]
fn test_validate_delegation_input_rejects_expired_or_equal_expiry() {
    let env = env();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);
    let scopes = scope_vec(&env, &["withdraw"]);
    let err = validate_delegation_input(&delegator, &delegate, &scopes, 100, 100).unwrap_err();
    assert!(err.contains("future"));
}

#[test]
fn test_is_scope_allowed() {
    let env = env();
    let scopes = scope_vec(&env, &["withdraw", "pause"]);
    assert!(is_scope_allowed(&scopes, &scope(&env, "withdraw")));
    assert!(!is_scope_allowed(&scopes, &scope(&env, "mint")));
}

#[test]
fn test_can_delegate_call_authorized() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
    let result = can_delegate_call(&d, &d.delegate, &scope(&env, "withdraw"), now);
    assert_eq!(result, DelegationCheck::Authorized);
}

#[test]
fn test_can_delegate_call_rejects_inactive() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, false, scope_vec(&env, &["withdraw"]));
    let result = can_delegate_call(&d, &d.delegate, &scope(&env, "withdraw"), now);
    assert!(!result.is_authorized());
    assert!(result.reason().contains("inactive"));
}

#[test]
fn test_can_delegate_call_rejects_wrong_caller() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
    let outsider = Address::generate(&env);
    let result = can_delegate_call(&d, &outsider, &scope(&env, "withdraw"), now);
    assert!(result.reason().contains("not delegated"));
}

#[test]
fn test_can_delegate_call_rejects_expired() {
    let env = env();
    let now = 1_000;
    let mut d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
    d.expires_at = now - 1;
    let result = can_delegate_call(&d, &d.delegate, &scope(&env, "withdraw"), now);
    assert!(result.reason().contains("expired"));
}

#[test]
fn test_can_delegate_call_rejects_scope_not_allowed() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
    let result = can_delegate_call(&d, &d.delegate, &scope(&env, "pause"), now);
    assert!(result.reason().contains("not delegated"));
}

#[test]
fn test_validate_revocation_only_delegator() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
    assert!(validate_revocation(&d, &d.delegator).is_ok());
    let err = validate_revocation(&d, &d.delegate).unwrap_err();
    assert!(err.contains("only delegator"));
}

#[test]
fn test_validate_revocation_rejects_inactive() {
    let env = env();
    let now = 1_000;
    let d = delegation(&env, now, false, scope_vec(&env, &["withdraw"]));
    let err = validate_revocation(&d, &d.delegator).unwrap_err();
    assert!(err.contains("already inactive"));
}

#[test]
fn test_count_active_delegations_for_filters_expired_and_inactive() {
    let env = env();
    let now = 10_000;
    let delegator = Address::generate(&env);
    let mut delegations = Vec::new(&env);

    delegations.push_back(AccessDelegation {
        delegator: delegator.clone(),
        delegate: Address::generate(&env),
        scopes: scope_vec(&env, &["withdraw"]),
        expires_at: now + 100,
        active: true,
    });
    delegations.push_back(AccessDelegation {
        delegator: delegator.clone(),
        delegate: Address::generate(&env),
        scopes: scope_vec(&env, &["pause"]),
        expires_at: now - 1,
        active: true,
    });
    delegations.push_back(AccessDelegation {
        delegator,
        delegate: Address::generate(&env),
        scopes: scope_vec(&env, &["mint"]),
        expires_at: now + 100,
        active: false,
    });

    assert_eq!(count_active_delegations_for(&delegations, &delegations.get(0).unwrap().delegator, now), 1);
}

#[test]
fn test_validate_delegation_capacity_rejects_limit() {
    let env = env();
    let now = 1_000;
    let delegator = Address::generate(&env);
    let mut delegations = Vec::new(&env);
    for _ in 0..MAX_ACTIVE_DELEGATIONS_PER_DELEGATOR {
        delegations.push_back(AccessDelegation {
            delegator: delegator.clone(),
            delegate: Address::generate(&env),
            scopes: scope_vec(&env, &["withdraw"]),
            expires_at: now + 100,
            active: true,
        });
    }
    let err = validate_delegation_capacity(&delegations, &delegator, now).unwrap_err();
    assert!(err.contains("limit"));
}

#[test]
fn test_event_helpers_do_not_panic() {
    let env = env();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);
    emit_delegation_created_event(&env, &delegator, &delegate, 12345);
    emit_delegation_revoked_event(&env, &delegator, &delegate);
}

proptest! {
    #[test]
    fn prop_validate_delegation_input_rejects_non_future_expiry(
        now in 1u64..=1_000_000u64
    ) {
        let env = env();
        let delegator = Address::generate(&env);
        let delegate = Address::generate(&env);
        let scopes = scope_vec(&env, &["withdraw"]);
        prop_assert!(validate_delegation_input(&delegator, &delegate, &scopes, now, now).is_err());
    }

    #[test]
    fn prop_can_delegate_call_authorizes_valid_delegate(
        now in 1u64..=1_000_000u64
    ) {
        let env = env();
        let d = delegation(&env, now, true, scope_vec(&env, &["withdraw"]));
        let result = can_delegate_call(&d, &d.delegate, &scope(&env, "withdraw"), now);
        prop_assert!(result.is_authorized());
    }

    #[test]
    fn prop_scope_limit_enforced(
        over in (MAX_SCOPE_COUNT + 1)..=(MAX_SCOPE_COUNT + 10)
    ) {
        let env = env();
        let delegator = Address::generate(&env);
        let delegate = Address::generate(&env);
        let mut scopes = Vec::new(&env);
        for _ in 0..over {
            scopes.push_back(scope(&env, "withdraw"));
        }
        let result = validate_delegation_input(&delegator, &delegate, &scopes, 100, 200);
        prop_assert!(result.is_err());
    }
}
