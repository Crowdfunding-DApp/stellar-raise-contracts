<<<<<<< HEAD
//! Comprehensive tests for `campaign_goal_minimum` constants and validation helpers.
//!
//! ## Coverage
//!   - All constant values are correct and stable
//!   - `validate_goal`             — happy path, boundary, below minimum
//!   - `validate_goal_amount`      — typed ContractError::InvalidGoal variant
//!   - `validate_min_contribution` — happy path, boundary, below minimum
//!   - `validate_deadline`         — happy path, exact boundary, too soon, overflow safety
//!   - `validate_platform_fee`     — happy path, exact cap, above cap
//!   - `compute_progress_bps`      — zero raised, partial, exact goal, over goal, zero goal guard
//!
//! ## Security Notes
//!   - Constants are part of the public API; accidental changes are caught immediately.
//!   - `validate_goal_amount` returns a typed error so the frontend can map it to a message.
//!   - `validate_deadline` uses `saturating_add` to prevent `u64` overflow.
||||||| a43ed59f
#[cfg(test)]
mod tests {
    use super::campaign_goal_minimum::*;
    use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, IntoVal, Symbol};
=======
#![allow(deprecated)] // `validate_goal` retained for regression coverage until fully removed.
>>>>>>> origin/main

<<<<<<< HEAD
use crate::campaign_goal_minimum::{
    compute_progress_bps, validate_deadline, validate_goal, validate_goal_amount,
    validate_min_contribution, validate_platform_fee, MAX_PLATFORM_FEE_BPS, MAX_PROGRESS_BPS,
    MIN_CONTRIBUTION_AMOUNT, MIN_DEADLINE_OFFSET, MIN_GOAL_AMOUNT, PROGRESS_BPS_SCALE,
};
use crate::ContractError;
use soroban_sdk::Env;
||||||| a43ed59f
    #[test]
    fn test_valid_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 500u64;
=======
//! Comprehensive tests for `campaign_goal_minimum` constants and validation helpers.
//!
//! # Coverage
//!
//! | Area                    | Cases                                                    |
//! |-------------------------|----------------------------------------------------------|
//! | Constants               | Correct values, stability, scale invariant               |
//! | `validate_goal` (deprecated) | Same thresholds as `validate_goal_amount`; kept for regression |
//! | `validate_goal_amount`  | Typed `GoalTooLow`, exact threshold, zero, negative      |
//! | `validate_min_contribution` | Floor, large, zero, negative, i128::MIN              |
//! | `validate_deadline`     | Exact offset, future, one-past, equal-now, past, overflow|
//! | `validate_platform_fee` | Zero, typical, exact cap, one-above, u32::MAX            |
//! | `compute_progress_bps`  | Zero raised, half, quarter, exact, over, massive over,   |
//! |                         | zero goal, negative goal, 1-of-large, min/min, 99%, 1bps |
//!
//! # Security notes
//!
//! - All helpers are pure functions; no state is mutated.
//! - `validate_goal_amount` returns `ContractError::GoalTooLow` (not a panic)
//!   so the frontend can map the error code to a user-facing message.
//! - `validate_deadline` uses `saturating_add` to prevent overflow when `now`
//!   is near `u64::MAX`.
//! - `compute_progress_bps` guards against division by zero and caps at
//!   `MAX_PROGRESS_BPS` for over-funded campaigns.
>>>>>>> origin/main

<<<<<<< HEAD
// ── Constant value assertions ─────────────────────────────────────────────────
||||||| a43ed59f
        create_campaign(env.clone(), creator.clone(), goal);
=======
use crate::campaign_goal_minimum::{
    compute_progress_bps, validate_deadline, validate_goal, validate_goal_amount,
    validate_min_contribution, validate_platform_fee, MAX_PLATFORM_FEE_BPS, MAX_PROGRESS_BPS,
    MIN_CONTRIBUTION_AMOUNT, MIN_DEADLINE_OFFSET, MIN_GOAL_AMOUNT, PROGRESS_BPS_SCALE,
};
use crate::ContractError;
use soroban_sdk::Env;
>>>>>>> origin/main

<<<<<<< HEAD
/// Ensures constants have not been accidentally changed.
/// These values are part of the public API; changing them is a breaking change.
#[test]
fn constants_have_expected_values() {
    assert_eq!(MIN_GOAL_AMOUNT, 1i128);
    assert_eq!(MIN_CONTRIBUTION_AMOUNT, 1i128);
    assert_eq!(MAX_PLATFORM_FEE_BPS, 10_000u32);
    assert_eq!(PROGRESS_BPS_SCALE, 10_000i128);
    assert_eq!(MIN_DEADLINE_OFFSET, 60u64);
    assert_eq!(MAX_PROGRESS_BPS, 10_000u32);
}

/// PROGRESS_BPS_SCALE and MAX_PROGRESS_BPS must be equal so that a fully-met
/// goal produces exactly MAX_PROGRESS_BPS.
#[test]
fn progress_scale_equals_max_progress_bps() {
    assert_eq!(PROGRESS_BPS_SCALE as u32, MAX_PROGRESS_BPS);
}

// ── validate_goal ─────────────────────────────────────────────────────────────

#[test]
fn validate_goal_accepts_minimum() {
    assert!(validate_goal(MIN_GOAL_AMOUNT).is_ok());
}

#[test]
fn validate_goal_accepts_large_value() {
    assert!(validate_goal(1_000_000_000).is_ok());
}

#[test]
fn validate_goal_accepts_one_above_minimum() {
    assert!(validate_goal(MIN_GOAL_AMOUNT + 1).is_ok());
}

#[test]
fn validate_goal_rejects_zero() {
    let err = validate_goal(0).unwrap_err();
    assert!(
        err.contains("MIN_GOAL_AMOUNT"),
        "error should mention MIN_GOAL_AMOUNT: {err}"
    );
}

#[test]
fn validate_goal_rejects_negative() {
    assert!(validate_goal(-1).is_err());
    assert!(validate_goal(i128::MIN).is_err());
}

// ── validate_min_contribution ─────────────────────────────────────────────────

#[test]
fn validate_min_contribution_accepts_minimum_floor() {
    assert!(validate_min_contribution(MIN_CONTRIBUTION_AMOUNT).is_ok());
}

#[test]
fn validate_min_contribution_accepts_large_value() {
    assert!(validate_min_contribution(1_000_000).is_ok());
}

#[test]
fn validate_min_contribution_rejects_zero() {
    let err = validate_min_contribution(0).unwrap_err();
    assert!(
        err.contains("MIN_CONTRIBUTION_AMOUNT"),
        "error should mention MIN_CONTRIBUTION_AMOUNT: {err}"
    );
}

#[test]
fn validate_min_contribution_rejects_negative() {
    assert!(validate_min_contribution(-1).is_err());
    assert!(validate_min_contribution(i128::MIN).is_err());
}

// ── validate_deadline ─────────────────────────────────────────────────────────

#[test]
fn validate_deadline_accepts_exact_offset() {
    let now: u64 = 1_000;
    let deadline = now + MIN_DEADLINE_OFFSET;
    assert!(validate_deadline(now, deadline).is_ok());
}

#[test]
fn validate_deadline_accepts_well_in_future() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + 3_600).is_ok());
}

#[test]
fn validate_deadline_accepts_one_second_past_offset() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + MIN_DEADLINE_OFFSET + 1).is_ok());
}

#[test]
fn validate_deadline_rejects_deadline_equal_to_now() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now).is_err());
}

#[test]
fn validate_deadline_rejects_deadline_in_past() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now - 1).is_err());
}

#[test]
fn validate_deadline_rejects_one_second_before_offset() {
    let now: u64 = 1_000;
    let deadline = now + MIN_DEADLINE_OFFSET - 1;
    assert!(validate_deadline(now, deadline).is_err());
}

/// `saturating_add` must prevent overflow when `now` is near `u64::MAX`.
#[test]
fn validate_deadline_saturating_add_prevents_overflow() {
    let now = u64::MAX - 10;
    // Does not panic — saturating_add prevents overflow.
    let _ = validate_deadline(now, u64::MAX);
}

// ── validate_platform_fee ─────────────────────────────────────────────────────

#[test]
fn validate_platform_fee_accepts_zero() {
    assert!(validate_platform_fee(0).is_ok());
}

#[test]
fn validate_platform_fee_accepts_typical_fee() {
    assert!(validate_platform_fee(250).is_ok()); // 2.5%
}

#[test]
fn validate_platform_fee_accepts_exact_cap() {
    assert!(validate_platform_fee(MAX_PLATFORM_FEE_BPS).is_ok());
}

#[test]
fn validate_platform_fee_rejects_one_above_cap() {
    let err = validate_platform_fee(MAX_PLATFORM_FEE_BPS + 1).unwrap_err();
    assert!(
        err.contains("MAX_PLATFORM_FEE_BPS"),
        "error should mention MAX_PLATFORM_FEE_BPS: {err}"
    );
}

#[test]
fn validate_platform_fee_rejects_max_u32() {
    assert!(validate_platform_fee(u32::MAX).is_err());
}

// ── compute_progress_bps ─────────────────────────────────────────────────────

#[test]
fn compute_progress_bps_zero_raised() {
    assert_eq!(compute_progress_bps(0, 1_000_000), 0);
}

#[test]
fn compute_progress_bps_half_goal() {
    assert_eq!(compute_progress_bps(500_000, 1_000_000), 5_000);
}

#[test]
fn compute_progress_bps_quarter_goal() {
    assert_eq!(compute_progress_bps(250_000, 1_000_000), 2_500);
}

#[test]
fn compute_progress_bps_exact_goal() {
    assert_eq!(compute_progress_bps(1_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_over_goal_capped() {
    assert_eq!(compute_progress_bps(2_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_massively_over_goal_capped() {
    assert_eq!(compute_progress_bps(i128::MAX, 1), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_zero_goal_returns_zero() {
    assert_eq!(compute_progress_bps(1_000, 0), 0);
}

#[test]
fn compute_progress_bps_negative_goal_returns_zero() {
    assert_eq!(compute_progress_bps(1_000, -1), 0);
}

#[test]
fn compute_progress_bps_one_token_of_large_goal() {
    assert_eq!(compute_progress_bps(1, 1_000_000), 0);
}

#[test]
fn compute_progress_bps_minimum_goal_minimum_raised() {
    assert_eq!(compute_progress_bps(1, 1), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_99_percent() {
    assert_eq!(compute_progress_bps(9_900, 10_000), 9_900);
}

#[test]
fn compute_progress_bps_1_bps() {
    assert_eq!(compute_progress_bps(1, 10_000), 1);
}

// ── validate_goal_amount (typed ContractError::InvalidGoal) ──────────────────

/// Goal exactly at the threshold is accepted.
#[test]
fn validate_goal_amount_accepts_exact_threshold() {
    let env = Env::default();
    assert!(validate_goal_amount(&env, MIN_GOAL_AMOUNT).is_ok());
}

/// Goal well above the threshold is accepted.
#[test]
fn validate_goal_amount_accepts_well_above_threshold() {
    let env = Env::default();
    assert!(validate_goal_amount(&env, 1_000_000_000).is_ok());
}

/// Goal below threshold returns `ContractError::InvalidGoal`.
#[test]
fn validate_goal_amount_rejects_below_threshold() {
    let env = Env::default();
    let result = validate_goal_amount(&env, MIN_GOAL_AMOUNT - 1);
    assert_eq!(result, Err(ContractError::InvalidGoal));
}

/// Zero goal returns `ContractError::InvalidGoal`.
#[test]
fn validate_goal_amount_rejects_zero() {
    let env = Env::default();
    assert_eq!(validate_goal_amount(&env, 0), Err(ContractError::InvalidGoal));
}

/// Negative goal returns `ContractError::InvalidGoal`.
#[test]
fn validate_goal_amount_rejects_negative() {
    let env = Env::default();
    assert_eq!(validate_goal_amount(&env, -1), Err(ContractError::InvalidGoal));
    assert_eq!(validate_goal_amount(&env, i128::MIN), Err(ContractError::InvalidGoal));
||||||| a43ed59f
        // Verify event emission
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let event = events.get(0).unwrap();
        assert_eq!(event.0, env.current_contract_address());
        assert_eq!(event.1, (Symbol::new(&env, "campaign"), Symbol::new(&env, "created")).into_val(&env));
        let data = event.2;
        assert_eq!(data, (creator, goal).into_val(&env));
    }

    #[test]
    #[should_panic(expected = "Minimum campaign goal not met")]
    fn test_below_minimum_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 50u64;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    #[should_panic(expected = "Campaign goal must be non-zero")]
    fn test_zero_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = 0u64;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    fn test_exact_minimum() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = MIN_CAMPAIGN_GOAL;

        create_campaign(env.clone(), creator.clone(), goal);
    }

    #[test]
    fn test_large_goal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let goal = u64::MAX;

        create_campaign(env.clone(), creator.clone(), goal);
    }
=======
// ── Constant stability assertions ─────────────────────────────────────────────

/// Ensures constants have not been accidentally changed.
/// These values are part of the public API; changing them is a breaking change.
#[test]
fn constants_have_expected_values() {
    assert_eq!(MIN_GOAL_AMOUNT, 1i128);
    assert_eq!(MIN_CONTRIBUTION_AMOUNT, 1i128);
    assert_eq!(MAX_PLATFORM_FEE_BPS, 10_000u32);
    assert_eq!(PROGRESS_BPS_SCALE, 10_000i128);
    assert_eq!(MIN_DEADLINE_OFFSET, 60u64);
    assert_eq!(MAX_PROGRESS_BPS, 10_000u32);
}

/// `PROGRESS_BPS_SCALE` and `MAX_PROGRESS_BPS` must be equal so that a
/// fully-met goal produces exactly `MAX_PROGRESS_BPS`.
#[test]
fn progress_scale_equals_max_progress_bps() {
    assert_eq!(PROGRESS_BPS_SCALE as u32, MAX_PROGRESS_BPS);
}

// ── validate_goal ─────────────────────────────────────────────────────────────

#[test]
fn validate_goal_accepts_minimum() {
    assert!(validate_goal(MIN_GOAL_AMOUNT).is_ok());
}

#[test]
fn validate_goal_accepts_one_above_minimum() {
    assert!(validate_goal(MIN_GOAL_AMOUNT + 1).is_ok());
}

#[test]
fn validate_goal_accepts_large_value() {
    assert!(validate_goal(1_000_000_000).is_ok());
}

#[test]
fn validate_goal_accepts_i128_max() {
    assert!(validate_goal(i128::MAX).is_ok());
}

#[test]
fn validate_goal_rejects_zero() {
    let err = validate_goal(0).unwrap_err();
    assert!(
        err.contains("MIN_GOAL_AMOUNT"),
        "error should mention MIN_GOAL_AMOUNT, got: {err}"
    );
}

#[test]
fn validate_goal_rejects_negative_one() {
    assert!(validate_goal(-1).is_err());
}

#[test]
fn validate_goal_rejects_i128_min() {
    assert!(validate_goal(i128::MIN).is_err());
}

// ── validate_min_contribution ─────────────────────────────────────────────────

#[test]
fn validate_min_contribution_accepts_floor() {
    assert!(validate_min_contribution(MIN_CONTRIBUTION_AMOUNT).is_ok());
}

#[test]
fn validate_min_contribution_accepts_one_above_floor() {
    assert!(validate_min_contribution(MIN_CONTRIBUTION_AMOUNT + 1).is_ok());
}

#[test]
fn validate_min_contribution_accepts_large_value() {
    assert!(validate_min_contribution(1_000_000).is_ok());
}

#[test]
fn validate_min_contribution_rejects_zero() {
    let err = validate_min_contribution(0).unwrap_err();
    assert!(
        err.contains("MIN_CONTRIBUTION_AMOUNT"),
        "error should mention MIN_CONTRIBUTION_AMOUNT, got: {err}"
    );
}

#[test]
fn validate_min_contribution_rejects_negative_one() {
    assert!(validate_min_contribution(-1).is_err());
}

#[test]
fn validate_min_contribution_rejects_i128_min() {
    assert!(validate_min_contribution(i128::MIN).is_err());
}

// ── validate_deadline ─────────────────────────────────────────────────────────

#[test]
fn validate_deadline_accepts_exact_offset() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + MIN_DEADLINE_OFFSET).is_ok());
}

#[test]
fn validate_deadline_accepts_one_second_past_offset() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + MIN_DEADLINE_OFFSET + 1).is_ok());
}

#[test]
fn validate_deadline_accepts_well_in_future() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + 3_600).is_ok());
}

#[test]
fn validate_deadline_rejects_one_second_before_offset() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now + MIN_DEADLINE_OFFSET - 1).is_err());
}

#[test]
fn validate_deadline_rejects_equal_to_now() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now).is_err());
}

#[test]
fn validate_deadline_rejects_deadline_in_past() {
    let now: u64 = 1_000;
    assert!(validate_deadline(now, now - 1).is_err());
}

/// `saturating_add` must prevent a panic when `now` is near `u64::MAX`.
///
/// @security  Without `saturating_add`, `now + MIN_DEADLINE_OFFSET` would
///            wrap to a small value, making any deadline appear valid.
#[test]
fn validate_deadline_saturating_add_prevents_overflow() {
    let now = u64::MAX - 10;
    // saturating_add clamps to u64::MAX, so deadline == u64::MAX is still
    // rejected (u64::MAX < u64::MAX is false, but u64::MAX - 1 < u64::MAX).
    // The important thing is that this does not panic.
    let _ = validate_deadline(now, u64::MAX);
    let _ = validate_deadline(now, u64::MAX - 5);
}

// ── validate_platform_fee ─────────────────────────────────────────────────────

#[test]
fn validate_platform_fee_accepts_zero() {
    assert!(validate_platform_fee(0).is_ok());
}

#[test]
fn validate_platform_fee_accepts_typical_fee() {
    // 2.5 % — a realistic platform fee
    assert!(validate_platform_fee(250).is_ok());
}

#[test]
fn validate_platform_fee_accepts_exact_cap() {
    assert!(validate_platform_fee(MAX_PLATFORM_FEE_BPS).is_ok());
}

#[test]
fn validate_platform_fee_rejects_one_above_cap() {
    let err = validate_platform_fee(MAX_PLATFORM_FEE_BPS + 1).unwrap_err();
    assert!(
        err.contains("MAX_PLATFORM_FEE_BPS"),
        "error should mention MAX_PLATFORM_FEE_BPS, got: {err}"
    );
}

#[test]
fn validate_platform_fee_rejects_u32_max() {
    assert!(validate_platform_fee(u32::MAX).is_err());
}

// ── compute_progress_bps ─────────────────────────────────────────────────────

#[test]
fn compute_progress_bps_zero_raised() {
    assert_eq!(compute_progress_bps(0, 1_000_000), 0);
}

#[test]
fn compute_progress_bps_half_goal() {
    // 500_000 / 1_000_000 = 50 % = 5_000 bps
    assert_eq!(compute_progress_bps(500_000, 1_000_000), 5_000);
}

#[test]
fn compute_progress_bps_quarter_goal() {
    assert_eq!(compute_progress_bps(250_000, 1_000_000), 2_500);
}

#[test]
fn compute_progress_bps_99_percent() {
    // 9_900 / 10_000 = 99 % = 9_900 bps
    assert_eq!(compute_progress_bps(9_900, 10_000), 9_900);
}

#[test]
fn compute_progress_bps_1_bps() {
    // 1 / 10_000 = 0.01 % = 1 bps
    assert_eq!(compute_progress_bps(1, 10_000), 1);
}

#[test]
fn compute_progress_bps_exact_goal() {
    assert_eq!(compute_progress_bps(1_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_over_goal_capped() {
    // 2× goal must still return MAX_PROGRESS_BPS, not 20_000.
    assert_eq!(compute_progress_bps(2_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_massively_over_goal_capped() {
    assert_eq!(compute_progress_bps(i128::MAX, 1), MAX_PROGRESS_BPS);
}

#[test]
fn compute_progress_bps_zero_goal_returns_zero() {
    // Division-by-zero guard — must not panic.
    assert_eq!(compute_progress_bps(1_000, 0), 0);
}

#[test]
fn compute_progress_bps_negative_goal_returns_zero() {
    assert_eq!(compute_progress_bps(1_000, -1), 0);
}

#[test]
fn compute_progress_bps_one_token_of_large_goal() {
    // 1 / 1_000_000 rounds down to 0 bps — integer division.
    assert_eq!(compute_progress_bps(1, 1_000_000), 0);
}

#[test]
fn compute_progress_bps_minimum_goal_minimum_raised() {
    // 1 / 1 = 100 % = 10_000 bps
    assert_eq!(compute_progress_bps(1, 1), MAX_PROGRESS_BPS);
}

// ── validate_goal_amount (typed ContractError::GoalTooLow) ───────────────────

/// Goal exactly at the threshold is accepted.
#[test]
fn validate_goal_amount_accepts_exact_threshold() {
    let env = Env::default();
    assert!(validate_goal_amount(&env, MIN_GOAL_AMOUNT).is_ok());
}

/// Goal well above the threshold is accepted.
#[test]
fn validate_goal_amount_accepts_well_above_threshold() {
    let env = Env::default();
    assert!(validate_goal_amount(&env, 1_000_000_000).is_ok());
}

/// Goal one below threshold returns `ContractError::GoalTooLow`.
///
/// @security  This is the primary on-chain enforcement test.  If this fails,
///            zero-goal campaigns can be created and immediately drained.
#[test]
fn validate_goal_amount_rejects_below_threshold() {
    let env = Env::default();
    assert_eq!(
        validate_goal_amount(&env, MIN_GOAL_AMOUNT - 1),
        Err(ContractError::GoalTooLow)
    );
}

/// Zero goal returns `ContractError::GoalTooLow`.
#[test]
fn validate_goal_amount_rejects_zero() {
    let env = Env::default();
    assert_eq!(
        validate_goal_amount(&env, 0),
        Err(ContractError::GoalTooLow)
    );
}

/// Negative goal returns `ContractError::GoalTooLow`.
#[test]
fn validate_goal_amount_rejects_negative_one() {
    let env = Env::default();
    assert_eq!(
        validate_goal_amount(&env, -1),
        Err(ContractError::GoalTooLow)
    );
}

/// `i128::MIN` goal returns `ContractError::GoalTooLow` without panicking.
#[test]
fn validate_goal_amount_rejects_i128_min() {
    let env = Env::default();
    assert_eq!(
        validate_goal_amount(&env, i128::MIN),
        Err(ContractError::GoalTooLow)
    );
}

/// Calling `validate_goal_amount` twice with the same env is idempotent.
#[test]
fn validate_goal_amount_is_idempotent() {
    let env = Env::default();
    assert!(validate_goal_amount(&env, 100).is_ok());
    assert!(validate_goal_amount(&env, 100).is_ok());
}

/// `GoalTooLow` discriminant is stable and does not collide with other errors.
///
/// @security  Discriminant stability is required so that off-chain scripts
///            that map numeric codes to messages continue to work after upgrades.
#[test]
fn goal_too_low_discriminant_is_stable() {
    assert_eq!(ContractError::GoalTooLow as u32, 13);
}

/// `initialize` maps sub-minimum goals to `InvalidGoal` (8), not `GoalTooLow`,
/// for backward-compatible client error handling.
#[test]
fn invalid_goal_discriminant_for_initialize_path() {
    assert_eq!(ContractError::InvalidGoal as u32, 8);
>>>>>>> origin/main
}
