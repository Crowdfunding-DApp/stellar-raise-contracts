//! # time_locked_functions.test.rs
//!
//! @notice  Comprehensive test suite for `time_locked_functions.rs`.
//!          Covers configuration validation, schedule/execute/cancel flows,
//!          status derivation, event emission, authorization, expiry windows,
//!          and property-based invariants.
//!
//! @dev     Tests are grouped into seven sections:
//!          1. Pure helper validation
//!          2. Initialization and getters
//!          3. Scheduling
//!          4. Status derivation
//!          5. Cancellation
//!          6. Execution
//!          7. Property-based / fuzz tests
//!
//! @custom:security-note  Every failure-path test asserts the exact rejection
//!          reason or panic message so regressions in timelock enforcement are
//!          caught immediately.  Targets ≥ 95 % line coverage.

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env, String, Symbol, TryFromVal,
};

use crate::time_locked_functions::{
    compute_execute_after, derive_status, validate_action_name, validate_admin_caller,
    validate_cancellation, validate_config, validate_delay, validate_execution,
    validate_payload_hash, validate_schedule_request, SecurityTimeLockedFunctions,
    SecurityTimeLockedFunctionsClient, TimeLockConfig, TimeLockStatus, TimeLockedAction,
    MAX_ACTION_NAME_LEN, MAX_ALLOWED_DELAY, MAX_ALLOWED_GRACE_PERIOD, MIN_ALLOWED_DELAY,
    MIN_ALLOWED_GRACE_PERIOD,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

const START_TIME: u64 = 10_000;
const DEFAULT_MIN_DELAY: u64 = 3_600;
const DEFAULT_MAX_DELAY: u64 = 86_400;
const DEFAULT_GRACE: u64 = 7_200;

fn env() -> Env {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set_timestamp(START_TIME);
    e
}

fn admin(env: &Env) -> Address {
    Address::generate(env)
}

fn outsider(env: &Env) -> Address {
    Address::generate(env)
}

fn hash(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn config(env: &Env) -> TimeLockConfig {
    TimeLockConfig {
        admin: admin(env),
        min_delay: DEFAULT_MIN_DELAY,
        max_delay: DEFAULT_MAX_DELAY,
        grace_period: DEFAULT_GRACE,
    }
}

fn make_action(
    env: &Env,
    execute_after: u64,
    executed_at: u64,
    cancelled_at: u64,
) -> TimeLockedAction {
    TimeLockedAction {
        action_id: 1,
        proposer: admin(env),
        action_name: String::from_str(env, "upgrade"),
        payload_hash: hash(env, 7),
        created_at: START_TIME,
        execute_after,
        delay_seconds: execute_after.saturating_sub(START_TIME),
        executed_at,
        cancelled_at,
    }
}

fn setup_contract() -> (Env, Address, SecurityTimeLockedFunctionsClient<'static>) {
    let env = env();
    let admin = admin(&env);
    let contract_id = env.register_contract(None, SecurityTimeLockedFunctions);
    let client = SecurityTimeLockedFunctionsClient::new(&env, &contract_id);
    (env, admin, client)
}

fn initialize_contract(client: &SecurityTimeLockedFunctionsClient<'_>, admin: &Address) {
    client.initialize(
        &admin.clone(),
        &DEFAULT_MIN_DELAY,
        &DEFAULT_MAX_DELAY,
        &DEFAULT_GRACE,
    );
}

fn schedule_action(
    client: &SecurityTimeLockedFunctionsClient<'_>,
    env: &Env,
    admin: &Address,
    delay: u64,
) -> u64 {
    client.schedule_action(
        &admin.clone(),
        &String::from_str(env, "upgrade"),
        &hash(env, 9),
        &delay,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Pure helper validation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_validate_config_ok() {
    let env = env();
    assert!(validate_config(&config(&env)).is_ok());
}

#[test]
fn test_validate_config_min_delay_too_small() {
    let env = env();
    let mut cfg = config(&env);
    cfg.min_delay = MIN_ALLOWED_DELAY - 1;
    assert_eq!(
        validate_config(&cfg).unwrap_err(),
        "min_delay below MIN_ALLOWED_DELAY"
    );
}

#[test]
fn test_validate_config_max_delay_too_large() {
    let env = env();
    let mut cfg = config(&env);
    cfg.max_delay = MAX_ALLOWED_DELAY + 1;
    assert_eq!(
        validate_config(&cfg).unwrap_err(),
        "max_delay exceeds MAX_ALLOWED_DELAY"
    );
}

#[test]
fn test_validate_config_min_exceeds_max() {
    let env = env();
    let mut cfg = config(&env);
    cfg.min_delay = DEFAULT_MIN_DELAY + 1;
    cfg.max_delay = DEFAULT_MIN_DELAY;
    assert_eq!(
        validate_config(&cfg).unwrap_err(),
        "min_delay exceeds max_delay"
    );
}

#[test]
fn test_validate_config_grace_period_too_small() {
    let env = env();
    let mut cfg = config(&env);
    cfg.grace_period = MIN_ALLOWED_GRACE_PERIOD - 1;
    assert_eq!(
        validate_config(&cfg).unwrap_err(),
        "grace_period below MIN_ALLOWED_GRACE_PERIOD"
    );
}

#[test]
fn test_validate_config_grace_period_too_large() {
    let env = env();
    let mut cfg = config(&env);
    cfg.grace_period = MAX_ALLOWED_GRACE_PERIOD + 1;
    assert_eq!(
        validate_config(&cfg).unwrap_err(),
        "grace_period exceeds MAX_ALLOWED_GRACE_PERIOD"
    );
}

#[test]
fn test_validate_admin_caller_ok() {
    let env = env();
    let cfg = config(&env);
    assert!(validate_admin_caller(&cfg, &cfg.admin).is_ok());
}

#[test]
fn test_validate_admin_caller_fail() {
    let env = env();
    let cfg = config(&env);
    assert_eq!(
        validate_admin_caller(&cfg, &outsider(&env)).unwrap_err(),
        "caller is not the timelock admin"
    );
}

#[test]
fn test_validate_action_name_ok() {
    let env = env();
    assert!(validate_action_name(&String::from_str(&env, "rotate_keys")).is_ok());
}

#[test]
fn test_validate_action_name_empty_fail() {
    let env = env();
    assert_eq!(
        validate_action_name(&String::from_str(&env, "")).unwrap_err(),
        "action_name must not be empty"
    );
}

#[test]
fn test_validate_action_name_too_long_fail() {
    let env = env();
    let long_name = String::from_str(&env, &"a".repeat((MAX_ACTION_NAME_LEN + 1) as usize));
    assert_eq!(
        validate_action_name(&long_name).unwrap_err(),
        "action_name exceeds MAX_ACTION_NAME_LEN"
    );
}

#[test]
fn test_validate_payload_hash_ok() {
    let env = env();
    assert!(validate_payload_hash(&hash(&env, 1)).is_ok());
}

#[test]
fn test_validate_payload_hash_zero_fail() {
    let env = env();
    let zero = BytesN::from_array(&env, &[0u8; 32]);
    assert_eq!(
        validate_payload_hash(&zero).unwrap_err(),
        "payload_hash must not be zero"
    );
}

#[test]
fn test_validate_delay_ok() {
    let env = env();
    let cfg = config(&env);
    assert!(validate_delay(DEFAULT_MIN_DELAY, &cfg).is_ok());
}

#[test]
fn test_validate_delay_too_small_fail() {
    let env = env();
    let cfg = config(&env);
    assert_eq!(
        validate_delay(DEFAULT_MIN_DELAY - 1, &cfg).unwrap_err(),
        "delay below configured minimum"
    );
}

#[test]
fn test_validate_delay_too_large_fail() {
    let env = env();
    let cfg = config(&env);
    assert_eq!(
        validate_delay(DEFAULT_MAX_DELAY + 1, &cfg).unwrap_err(),
        "delay exceeds configured maximum"
    );
}

#[test]
fn test_compute_execute_after_ok() {
    assert_eq!(compute_execute_after(100, 50).unwrap(), 150);
}

#[test]
fn test_compute_execute_after_zero_now_fail() {
    assert_eq!(
        compute_execute_after(0, 50).unwrap_err(),
        "current timestamp must be non-zero"
    );
}

#[test]
fn test_compute_execute_after_overflow_fail() {
    assert_eq!(
        compute_execute_after(u64::MAX, 1).unwrap_err(),
        "execute_after overflow"
    );
}

#[test]
fn test_validate_schedule_request_ok() {
    let env = env();
    let cfg = config(&env);
    let eta = validate_schedule_request(
        &cfg,
        &cfg.admin,
        &String::from_str(&env, "upgrade"),
        &hash(&env, 8),
        DEFAULT_MIN_DELAY,
        START_TIME,
    )
    .unwrap();
    assert_eq!(eta, START_TIME + DEFAULT_MIN_DELAY);
}

#[test]
fn test_validate_schedule_request_admin_fail() {
    let env = env();
    let cfg = config(&env);
    assert_eq!(
        validate_schedule_request(
            &cfg,
            &outsider(&env),
            &String::from_str(&env, "upgrade"),
            &hash(&env, 8),
            DEFAULT_MIN_DELAY,
            START_TIME,
        )
        .unwrap_err(),
        "caller is not the timelock admin"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Initialization and getters
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_sets_config() {
    let (env, admin, client) = setup_contract();
    let cfg = client.initialize(
        &admin,
        &DEFAULT_MIN_DELAY,
        &DEFAULT_MAX_DELAY,
        &DEFAULT_GRACE,
    );
    assert_eq!(cfg.admin, admin);
    assert_eq!(client.get_config().unwrap(), cfg);
    assert_eq!(client.action_count(), 0);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice_blocked() {
    let (_env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    initialize_contract(&client, &admin);
}

#[test]
fn test_get_status_missing_action_returns_none() {
    let (_env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    assert_eq!(client.get_status(&99), None);
}

#[test]
fn test_is_action_ready_missing_action_false() {
    let (_env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    assert!(!client.is_action_ready(&99));
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Scheduling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_schedule_action_persists_action() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);

    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);
    let action = client.get_action(&action_id).unwrap();

    assert_eq!(action.action_id, 1);
    assert_eq!(action.proposer, admin);
    assert_eq!(action.created_at, START_TIME);
    assert_eq!(action.execute_after, START_TIME + DEFAULT_MIN_DELAY);
    assert_eq!(action.delay_seconds, DEFAULT_MIN_DELAY);
    assert_eq!(client.action_count(), 1);
}

#[test]
fn test_schedule_action_emits_event() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);

    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);
    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();

    assert_eq!(
        Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
        Symbol::new(&env, "timelock")
    );
    assert_eq!(
        Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
        Symbol::new(&env, "queued")
    );
    assert_eq!(
        u64::try_from_val(&env, &topics.get(2).unwrap()).unwrap(),
        action_id
    );
    assert_eq!(
        u64::try_from_val(&env, &data).unwrap(),
        START_TIME + DEFAULT_MIN_DELAY
    );
}

#[test]
#[should_panic(expected = "delay below configured minimum")]
fn test_schedule_action_delay_too_small_panics() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY - 1);
}

#[test]
#[should_panic(expected = "caller is not the timelock admin")]
fn test_schedule_action_non_admin_panics() {
    let (env, admin, client) = setup_contract();
    let outsider = outsider(&env);
    let action_name = String::from_str(&env, "upgrade");
    let payload_hash = hash(&env, 9);
    initialize_contract(&client, &admin);
    client.schedule_action(&outsider, &action_name, &payload_hash, &DEFAULT_MIN_DELAY);
}

#[test]
#[should_panic(expected = "current timestamp must be non-zero")]
fn test_schedule_action_zero_timestamp_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = admin(&env);
    let contract_id = env.register_contract(None, SecurityTimeLockedFunctions);
    let client = SecurityTimeLockedFunctionsClient::new(&env, &contract_id);
    let action_name = String::from_str(&env, "upgrade");
    let payload_hash = hash(&env, 9);
    initialize_contract(&client, &admin);
    client.schedule_action(&admin, &action_name, &payload_hash, &DEFAULT_MIN_DELAY);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Status derivation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_derive_status_pending() {
    let env = env();
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert_eq!(
        derive_status(&action, START_TIME + DEFAULT_MIN_DELAY - 1, DEFAULT_GRACE),
        TimeLockStatus::Pending
    );
}

#[test]
fn test_derive_status_ready() {
    let env = env();
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert_eq!(
        derive_status(&action, START_TIME + DEFAULT_MIN_DELAY, DEFAULT_GRACE),
        TimeLockStatus::Ready
    );
}

#[test]
fn test_derive_status_executed() {
    let env = env();
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, START_TIME + 1, 0);
    assert_eq!(
        derive_status(&action, START_TIME + DEFAULT_MIN_DELAY, DEFAULT_GRACE),
        TimeLockStatus::Executed
    );
}

#[test]
fn test_derive_status_cancelled() {
    let env = env();
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, START_TIME + 1);
    assert_eq!(
        derive_status(&action, START_TIME + DEFAULT_MIN_DELAY, DEFAULT_GRACE),
        TimeLockStatus::Cancelled
    );
}

#[test]
fn test_derive_status_expired() {
    let env = env();
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert_eq!(
        derive_status(
            &action,
            START_TIME + DEFAULT_MIN_DELAY + DEFAULT_GRACE + 1,
            DEFAULT_GRACE,
        ),
        TimeLockStatus::Expired
    );
}

#[test]
fn test_get_status_tracks_transitions() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);

    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);
    assert_eq!(
        client.get_status(&action_id).unwrap(),
        TimeLockStatus::Pending
    );

    env.ledger().set_timestamp(START_TIME + DEFAULT_MIN_DELAY);
    assert!(client.is_action_ready(&action_id));
    assert_eq!(
        client.get_status(&action_id).unwrap(),
        TimeLockStatus::Ready
    );

    env.ledger()
        .set_timestamp(START_TIME + DEFAULT_MIN_DELAY + DEFAULT_GRACE + 1);
    assert_eq!(
        client.get_status(&action_id).unwrap(),
        TimeLockStatus::Expired
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Cancellation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_validate_cancellation_ok() {
    let env = env();
    let cfg = config(&env);
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert!(validate_cancellation(&cfg, &cfg.admin, &action).is_ok());
}

#[test]
fn test_validate_cancellation_executed_fail() {
    let env = env();
    let cfg = config(&env);
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, START_TIME + 1, 0);
    assert_eq!(
        validate_cancellation(&cfg, &cfg.admin, &action).unwrap_err(),
        "action already executed"
    );
}

#[test]
fn test_cancel_action_marks_cancelled() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger().set_timestamp(START_TIME + 10);
    let action = client.cancel_action(&admin, &action_id);
    assert_eq!(action.cancelled_at, START_TIME + 10);
    assert_eq!(
        client.get_status(&action_id).unwrap(),
        TimeLockStatus::Cancelled
    );
}

#[test]
fn test_cancel_action_emits_event() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger().set_timestamp(START_TIME + 10);
    client.cancel_action(&admin, &action_id);

    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();
    assert_eq!(
        Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
        Symbol::new(&env, "cancel")
    );
    assert_eq!(u64::try_from_val(&env, &data).unwrap(), START_TIME + 10);
}

#[test]
#[should_panic(expected = "action already cancelled")]
fn test_cancel_action_twice_panics() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);
    client.cancel_action(&admin, &action_id);
    client.cancel_action(&admin, &action_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Execution
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_validate_execution_ready_ok() {
    let env = env();
    let cfg = config(&env);
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert!(validate_execution(&cfg, &cfg.admin, &action, START_TIME + DEFAULT_MIN_DELAY,).is_ok());
}

#[test]
fn test_validate_execution_pending_fail() {
    let env = env();
    let cfg = config(&env);
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert_eq!(
        validate_execution(
            &cfg,
            &cfg.admin,
            &action,
            START_TIME + DEFAULT_MIN_DELAY - 1,
        )
        .unwrap_err(),
        "action is not ready yet"
    );
}

#[test]
fn test_validate_execution_expired_fail() {
    let env = env();
    let cfg = config(&env);
    let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
    assert_eq!(
        validate_execution(
            &cfg,
            &cfg.admin,
            &action,
            START_TIME + DEFAULT_MIN_DELAY + DEFAULT_GRACE + 1,
        )
        .unwrap_err(),
        "action expired"
    );
}

#[test]
fn test_execute_action_marks_executed() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger().set_timestamp(START_TIME + DEFAULT_MIN_DELAY);
    let action = client.execute_action(&admin, &action_id);
    assert_eq!(action.executed_at, START_TIME + DEFAULT_MIN_DELAY);
    assert_eq!(
        client.get_status(&action_id).unwrap(),
        TimeLockStatus::Executed
    );
}

#[test]
fn test_execute_action_emits_event() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger().set_timestamp(START_TIME + DEFAULT_MIN_DELAY);
    client.execute_action(&admin, &action_id);

    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();
    assert_eq!(
        Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
        Symbol::new(&env, "exec")
    );
    assert_eq!(
        u64::try_from_val(&env, &topics.get(2).unwrap()).unwrap(),
        action_id
    );
    assert_eq!(
        u64::try_from_val(&env, &data).unwrap(),
        START_TIME + DEFAULT_MIN_DELAY
    );
}

#[test]
#[should_panic(expected = "action is not ready yet")]
fn test_execute_action_before_eta_panics() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);
    client.execute_action(&admin, &action_id);
}

#[test]
#[should_panic(expected = "action already executed")]
fn test_execute_action_twice_panics() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger().set_timestamp(START_TIME + DEFAULT_MIN_DELAY);
    client.execute_action(&admin, &action_id);
    client.execute_action(&admin, &action_id);
}

#[test]
#[should_panic(expected = "action expired")]
fn test_execute_action_after_expiry_panics() {
    let (env, admin, client) = setup_contract();
    initialize_contract(&client, &admin);
    let action_id = schedule_action(&client, &env, &admin, DEFAULT_MIN_DELAY);

    env.ledger()
        .set_timestamp(START_TIME + DEFAULT_MIN_DELAY + DEFAULT_GRACE + 1);
    client.execute_action(&admin, &action_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Property-based / fuzz tests
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_compute_execute_after_preserves_delay(now in 1u64..1_000_000, delay in 0u64..1_000_000) {
        let eta = compute_execute_after(now, delay).unwrap();
        prop_assert_eq!(eta - now, delay);
        prop_assert!(eta >= now);
    }

    #[test]
    fn prop_derive_status_before_eta_is_pending(offset in 1u64..10_000) {
        let env = env();
        let action = make_action(&env, START_TIME + offset, 0, 0);
        prop_assert_eq!(
            derive_status(&action, START_TIME + offset - 1, DEFAULT_GRACE),
            TimeLockStatus::Pending
        );
    }

    #[test]
    fn prop_derive_status_after_expiry_is_expired(extra in 1u64..10_000) {
        let env = env();
        let action = make_action(&env, START_TIME + DEFAULT_MIN_DELAY, 0, 0);
        prop_assert_eq!(
            derive_status(
                &action,
                START_TIME + DEFAULT_MIN_DELAY + DEFAULT_GRACE + extra,
                DEFAULT_GRACE
            ),
            TimeLockStatus::Expired
        );
    }
}
