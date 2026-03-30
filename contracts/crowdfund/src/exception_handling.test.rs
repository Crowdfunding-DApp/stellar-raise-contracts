#![cfg(test)]

soroban_sdk::contract_test!();

use crate::exception_handling::{self, Error, ensure_auth, invalid_input, invalid_state, state_limit_exceeded, validate_batch_size};
use soroban_sdk::{testutils::Address as _, Env, Symbol};

// Test all Error variants can be constructed and match.
#[test]
fn test_error_variants() {
    assert_eq!(Error::Unauthorized as u32, 1);
    assert_eq!(Error::InvalidInput as u32, 2);
    assert_eq!(Error::NotFound as u32, 3);
    assert_eq!(Error::Overflow as u32, 4);
    assert_eq!(Error::AlreadyInitialized as u32, 5);
    assert_eq!(Error::StateLimitExceeded as u32, 6);
    assert_eq!(Error::InvalidState as u32, 7);
    assert_eq!(Error::ZeroValue as u32, 8);
    assert_eq!(Error::BelowMinimum as u32, 9);
    assert_eq!(Error::CampaignInactive as u32, 10);
    assert_eq!(Error::BatchInvalid as u32, 11);
    assert_eq!(Error::InvalidWasmHash as u32, 12);
    assert_eq!(Error::AlreadyHalted as u32, 13);
    assert_eq!(Error::InvalidFee as u32, 14);
}

#[test]
fn test_ensure_auth_success() {
    let env = Env::default();
    let addr = Address::random(&env);
    addr.require_auth_for_args(&env);
    ensure_auth(&env, &addr).unwrap();
}

#[test]
#[should_panic(expected = "HostError(Unauthorized)")]
fn test_ensure_auth_failure() {
    let env = Env::default();
    let addr = Address::random(&env);
    let _ = ensure_auth(&env, &addr);
}

#[test]
fn test_invalid_input() {
    let env = Env::default();
    let result = invalid_input(&env, "bad amount");
    assert_eq!(result, Err(exception_handling::Error::InvalidInput));
    // Check event emitted
    let events = env.events().all();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].1, (Symbol::new(&env, "error"), Symbol::new(&env, "InvalidInput")));
}

#[test]
fn test_invalid_state() {
    let env = Env::default();
    let result = invalid_state(&env, "CampaignInactive");
    assert_eq!(result, Err(exception_handling::Error::InvalidState));
}

#[test]
fn test_state_limit_exceeded() {
    let env = Env::default();
    let result = state_limit_exceeded(&env);
    assert_eq!(result, Err(exception_handling::Error::StateLimitExceeded));
}

#[test]
fn test_validate_batch_size_valid() {
    assert!(validate_batch_size(1, 10).is_ok());
    assert!(validate_batch_size(10, 10).is_ok());
}

#[test]
fn test_validate_batch_size_empty() {
    let result = validate_batch_size(0, 10);
    assert_eq!(result, Err(Error::BatchInvalid));
}

#[test]
fn test_validate_batch_size_too_large() {
    let result = validate_batch_size(11, 10);
    assert_eq!(result, Err(Error::BatchInvalid));
}

