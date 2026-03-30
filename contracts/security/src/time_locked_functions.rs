//! # time_locked_functions
//!
//! @notice  Time-lock utilities for security-sensitive contract operations in
//!          Stellar Raise.  This module enforces delayed execution for
//!          privileged actions such as upgrades, emergency-mode changes, and
//!          key-rotation workflows so operators and monitors have time to
//!          review queued changes before they execute.
//!
//! @dev     The contract stores a single `TimeLockConfig` and a monotonically
//!          increasing set of queued `TimeLockedAction` records in Soroban
//!          instance storage.  Only the configured admin may queue, cancel, or
//!          execute actions.  Actions become executable after `execute_after`
//!          and expire after a bounded grace period.
//!
//! @custom:security-note
//!   - Delayed execution reduces the blast radius of compromised admin keys.
//!   - Zero payload hashes are rejected to prevent ambiguous queue entries.
//!   - Expiry windows prevent stale queued actions from executing indefinitely.
//!   - Single-use execution and cancellation timestamps prevent replay.

#![allow(dead_code)]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, String,
};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// @notice  Minimum timelock delay accepted by this module.
/// @dev     One hour is long enough for off-chain monitors to react while still
///          being practical for operational changes.
pub const MIN_ALLOWED_DELAY: u64 = 3_600;

/// @notice  Maximum timelock delay accepted by this module.
/// @dev     Delays longer than 30 days are rejected to avoid effectively
///          orphaning queued actions.
pub const MAX_ALLOWED_DELAY: u64 = 2_592_000; // 30 days

/// @notice  Minimum grace period during which a ready action may be executed.
pub const MIN_ALLOWED_GRACE_PERIOD: u64 = 3_600;

/// @notice  Maximum grace period during which a ready action may be executed.
pub const MAX_ALLOWED_GRACE_PERIOD: u64 = 604_800; // 7 days

/// @notice  Maximum allowed action name length in bytes.
/// @dev     Bounds event/storage payload size for reviewability and indexers.
pub const MAX_ACTION_NAME_LEN: u32 = 64;

const ZERO_HASH: [u8; 32] = [0u8; 32];

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// @notice  Runtime status of a queued time-locked action.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum TimeLockStatus {
    /// Action is queued and still waiting for its delay to elapse.
    Pending,
    /// Action is within its execution window and may be executed.
    Ready,
    /// Action has already been executed and is no longer usable.
    Executed,
    /// Action was explicitly cancelled before execution.
    Cancelled,
    /// Action exceeded its execution grace period and is no longer usable.
    Expired,
}

/// @notice  Immutable configuration for the timelock module.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct TimeLockConfig {
    /// Address allowed to queue, cancel, and execute actions.
    pub admin: Address,
    /// Minimum permitted queue delay in seconds.
    pub min_delay: u64,
    /// Maximum permitted queue delay in seconds.
    pub max_delay: u64,
    /// Execution grace period in seconds after `execute_after`.
    pub grace_period: u64,
}

/// @notice  A queued privileged action subject to delayed execution.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct TimeLockedAction {
    /// Monotonic unique ID.
    pub action_id: u64,
    /// Address that queued the action.
    pub proposer: Address,
    /// Human-readable action label for reviewers.
    pub action_name: String,
    /// Hash of the intended call data / payload.
    pub payload_hash: BytesN<32>,
    /// Timestamp when the action was queued.
    pub created_at: u64,
    /// Timestamp after which the action may be executed.
    pub execute_after: u64,
    /// Original requested delay in seconds.
    pub delay_seconds: u64,
    /// Timestamp when the action executed, or `0` if not executed.
    pub executed_at: u64,
    /// Timestamp when the action was cancelled, or `0` if not cancelled.
    pub cancelled_at: u64,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Config,
    ActionCounter,
    Action(u64),
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure helpers
// ─────────────────────────────────────────────────────────────────────────────

/// @notice  Validates a timelock configuration before it is stored.
/// @custom:security-note  Rejects misconfigurations that would remove the
///          security value of the timelock or make actions permanently inert.
pub fn validate_config(config: &TimeLockConfig) -> Result<(), &'static str> {
    if config.min_delay < MIN_ALLOWED_DELAY {
        return Err("min_delay below MIN_ALLOWED_DELAY");
    }
    if config.max_delay > MAX_ALLOWED_DELAY {
        return Err("max_delay exceeds MAX_ALLOWED_DELAY");
    }
    if config.min_delay > config.max_delay {
        return Err("min_delay exceeds max_delay");
    }
    if config.grace_period < MIN_ALLOWED_GRACE_PERIOD {
        return Err("grace_period below MIN_ALLOWED_GRACE_PERIOD");
    }
    if config.grace_period > MAX_ALLOWED_GRACE_PERIOD {
        return Err("grace_period exceeds MAX_ALLOWED_GRACE_PERIOD");
    }
    Ok(())
}

/// @notice  Verifies that `caller` is the configured timelock admin.
pub fn validate_admin_caller(
    config: &TimeLockConfig,
    caller: &Address,
) -> Result<(), &'static str> {
    if config.admin != *caller {
        Err("caller is not the timelock admin")
    } else {
        Ok(())
    }
}

/// @notice  Ensures an action name is reviewable and non-empty.
pub fn validate_action_name(action_name: &String) -> Result<(), &'static str> {
    if action_name.len() == 0 {
        return Err("action_name must not be empty");
    }
    if action_name.len() > MAX_ACTION_NAME_LEN {
        return Err("action_name exceeds MAX_ACTION_NAME_LEN");
    }
    Ok(())
}

/// @notice  Rejects all-zero payload hashes.
/// @custom:security-note  A zero hash is typically an uninitialized placeholder
///          and would make audit trails ambiguous.
pub fn validate_payload_hash(payload_hash: &BytesN<32>) -> Result<(), &'static str> {
    if payload_hash.to_array() == ZERO_HASH {
        Err("payload_hash must not be zero")
    } else {
        Ok(())
    }
}

/// @notice  Validates a requested delay against the active configuration.
pub fn validate_delay(delay: u64, config: &TimeLockConfig) -> Result<(), &'static str> {
    if delay < config.min_delay {
        return Err("delay below configured minimum");
    }
    if delay > config.max_delay {
        return Err("delay exceeds configured maximum");
    }
    Ok(())
}

/// @notice  Computes the first execution timestamp for a queued action.
/// @custom:security-note  Uses checked addition to prevent overflow.
pub fn compute_execute_after(now: u64, delay: u64) -> Result<u64, &'static str> {
    if now == 0 {
        return Err("current timestamp must be non-zero");
    }
    now.checked_add(delay).ok_or("execute_after overflow")
}

/// @notice  Validates the full schedule request and returns the action ETA.
pub fn validate_schedule_request(
    config: &TimeLockConfig,
    caller: &Address,
    action_name: &String,
    payload_hash: &BytesN<32>,
    delay: u64,
    now: u64,
) -> Result<u64, &'static str> {
    validate_admin_caller(config, caller)?;
    validate_action_name(action_name)?;
    validate_payload_hash(payload_hash)?;
    validate_delay(delay, config)?;
    compute_execute_after(now, delay)
}

/// @notice  Derives the current status of a time-locked action.
pub fn derive_status(action: &TimeLockedAction, now: u64, grace_period: u64) -> TimeLockStatus {
    if action.cancelled_at != 0 {
        return TimeLockStatus::Cancelled;
    }
    if action.executed_at != 0 {
        return TimeLockStatus::Executed;
    }
    if now < action.execute_after {
        return TimeLockStatus::Pending;
    }

    let expiry = action.execute_after.saturating_add(grace_period);
    if now <= expiry {
        TimeLockStatus::Ready
    } else {
        TimeLockStatus::Expired
    }
}

/// @notice  Verifies that an action may still be cancelled.
pub fn validate_cancellation(
    config: &TimeLockConfig,
    caller: &Address,
    action: &TimeLockedAction,
) -> Result<(), &'static str> {
    validate_admin_caller(config, caller)?;
    if action.executed_at != 0 {
        return Err("action already executed");
    }
    if action.cancelled_at != 0 {
        return Err("action already cancelled");
    }
    Ok(())
}

/// @notice  Verifies that an action may be executed now.
pub fn validate_execution(
    config: &TimeLockConfig,
    caller: &Address,
    action: &TimeLockedAction,
    now: u64,
) -> Result<(), &'static str> {
    validate_admin_caller(config, caller)?;
    match derive_status(action, now, config.grace_period) {
        TimeLockStatus::Ready => Ok(()),
        TimeLockStatus::Pending => Err("action is not ready yet"),
        TimeLockStatus::Executed => Err("action already executed"),
        TimeLockStatus::Cancelled => Err("action already cancelled"),
        TimeLockStatus::Expired => Err("action expired"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Storage helpers
// ─────────────────────────────────────────────────────────────────────────────

fn load_config(env: &Env) -> TimeLockConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .expect("timelock not initialized")
}

fn load_action(env: &Env, action_id: u64) -> TimeLockedAction {
    env.storage()
        .instance()
        .get(&DataKey::Action(action_id))
        .expect("action not found")
}

fn maybe_load_action(env: &Env, action_id: u64) -> Option<TimeLockedAction> {
    env.storage().instance().get(&DataKey::Action(action_id))
}

fn save_action(env: &Env, action: &TimeLockedAction) {
    env.storage()
        .instance()
        .set(&DataKey::Action(action.action_id), action);
}

fn next_action_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ActionCounter)
        .unwrap_or(0);
    let next = current.checked_add(1).expect("action counter overflow");
    env.storage().instance().set(&DataKey::ActionCounter, &next);
    next
}

fn require_non_zero_timestamp(env: &Env) -> u64 {
    let now = env.ledger().timestamp();
    if now == 0 {
        panic!("current timestamp must be non-zero");
    }
    now
}

fn emit_scheduled_event(env: &Env, action_id: u64, execute_after: u64) {
    env.events().publish(
        (
            symbol_short!("timelock"),
            symbol_short!("queued"),
            action_id,
        ),
        execute_after,
    );
}

fn emit_cancelled_event(env: &Env, action_id: u64, cancelled_at: u64) {
    env.events().publish(
        (
            symbol_short!("timelock"),
            symbol_short!("cancel"),
            action_id,
        ),
        cancelled_at,
    );
}

fn emit_executed_event(env: &Env, action_id: u64, executed_at: u64) {
    env.events().publish(
        (symbol_short!("timelock"), symbol_short!("exec"), action_id),
        executed_at,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────────────────────────────────────

/// @notice  On-chain timelock for privileged security operations.
#[contract]
pub struct SecurityTimeLockedFunctions;

#[contractimpl]
impl SecurityTimeLockedFunctions {
    /// @notice  Initializes the timelock configuration.
    /// @custom:security-note  Initialization is single-use and requires admin
    ///          authorization to prevent hostile takeover during setup.
    pub fn initialize(
        env: Env,
        admin: Address,
        min_delay: u64,
        max_delay: u64,
        grace_period: u64,
    ) -> TimeLockConfig {
        if env.storage().instance().has(&DataKey::Config) {
            panic!("already initialized");
        }

        admin.require_auth();

        let config = TimeLockConfig {
            admin,
            min_delay,
            max_delay,
            grace_period,
        };
        if let Err(reason) = validate_config(&config) {
            panic!("{}", reason);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::ActionCounter, &0u64);
        config
    }

    /// @notice  Returns the active configuration, if initialized.
    pub fn get_config(env: Env) -> Option<TimeLockConfig> {
        env.storage().instance().get(&DataKey::Config)
    }

    /// @notice  Returns the number of queued actions ever created.
    pub fn action_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ActionCounter)
            .unwrap_or(0)
    }

    /// @notice  Queues a new privileged action subject to the configured delay.
    /// @param   caller        The admin address queuing the action.
    /// @param   action_name   Human-readable label for reviewers.
    /// @param   payload_hash  Hash of the call payload to be executed later.
    /// @param   delay         Requested delay in seconds.
    /// @return  Monotonic queued action ID.
    pub fn schedule_action(
        env: Env,
        caller: Address,
        action_name: String,
        payload_hash: BytesN<32>,
        delay: u64,
    ) -> u64 {
        caller.require_auth();

        let config = load_config(&env);
        let now = require_non_zero_timestamp(&env);
        let execute_after = match validate_schedule_request(
            &config,
            &caller,
            &action_name,
            &payload_hash,
            delay,
            now,
        ) {
            Ok(execute_after) => execute_after,
            Err(reason) => panic!("{}", reason),
        };

        let action_id = next_action_id(&env);
        let action = TimeLockedAction {
            action_id,
            proposer: caller,
            action_name,
            payload_hash,
            created_at: now,
            execute_after,
            delay_seconds: delay,
            executed_at: 0,
            cancelled_at: 0,
        };

        save_action(&env, &action);
        emit_scheduled_event(&env, action_id, execute_after);
        action_id
    }

    /// @notice  Cancels a queued action before it is executed.
    pub fn cancel_action(env: Env, caller: Address, action_id: u64) -> TimeLockedAction {
        caller.require_auth();

        let config = load_config(&env);
        let mut action = load_action(&env, action_id);
        if let Err(reason) = validate_cancellation(&config, &caller, &action) {
            panic!("{}", reason);
        }

        let now = require_non_zero_timestamp(&env);
        action.cancelled_at = now;
        save_action(&env, &action);
        emit_cancelled_event(&env, action_id, now);
        action
    }

    /// @notice  Executes a queued action after the delay and before expiry.
    pub fn execute_action(env: Env, caller: Address, action_id: u64) -> TimeLockedAction {
        caller.require_auth();

        let config = load_config(&env);
        let mut action = load_action(&env, action_id);
        let now = require_non_zero_timestamp(&env);
        if let Err(reason) = validate_execution(&config, &caller, &action, now) {
            panic!("{}", reason);
        }

        action.executed_at = now;
        save_action(&env, &action);
        emit_executed_event(&env, action_id, now);
        action
    }

    /// @notice  Returns a queued action by ID.
    pub fn get_action(env: Env, action_id: u64) -> Option<TimeLockedAction> {
        maybe_load_action(&env, action_id)
    }

    /// @notice  Returns the derived status for a queued action.
    pub fn get_status(env: Env, action_id: u64) -> Option<TimeLockStatus> {
        let config = load_config(&env);
        let action = maybe_load_action(&env, action_id)?;
        Some(derive_status(
            &action,
            env.ledger().timestamp(),
            config.grace_period,
        ))
    }

    /// @notice  Returns `true` when the action is currently executable.
    pub fn is_action_ready(env: Env, action_id: u64) -> bool {
        matches!(
            Self::get_status(env, action_id),
            Some(TimeLockStatus::Ready)
        )
    }
}
