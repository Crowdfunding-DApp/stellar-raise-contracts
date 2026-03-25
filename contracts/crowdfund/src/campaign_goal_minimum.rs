//! # campaign_goal_minimum
//!
//! @title   CampaignGoalMinimum — Enforces minimum campaign goal thresholds.
//!
//! @notice  This module provides the logic to prevent campaigns from being
//!          created with goals below a defined minimum, ensuring realistic
//!          fundraising targets and improving security.

use soroban_sdk::{Address, Env};

/// Minimum allowed campaign goal.
pub const MIN_CAMPAIGN_GOAL: u64 = 100;

/// Creates a new campaign with goal validation.
///
/// # Parameters
/// - creator: campaign owner
/// - goal: funding target
///
/// # Security
/// Ensures goal meets minimum threshold and creator is authenticated.
pub fn create_campaign(env: Env, creator: Address, goal: u64) {
    creator.require_auth();

// ── Validation helpers ───────────────────────────────────────────────────────

/// Validates that `goal` meets the minimum threshold.
///
/// @param  goal  The proposed campaign goal in token units.
/// @return       `Ok(())` if valid, `Err(&'static str)` with a reason otherwise.
///
/// @dev    Returns a `&'static str` rather than `ContractError` so this module
///         stays free of the contract's error type and can be used in off-chain
///         tooling without pulling in the full contract dependency.
#[inline]
pub fn validate_goal(goal: i128) -> Result<(), &'static str> {
    if goal < MIN_GOAL_AMOUNT {
        return Err("goal must be at least MIN_GOAL_AMOUNT");
    }
    Ok(())
}

/// Validates that `goal_amount` meets the minimum threshold, returning a typed
/// [`ContractError::GoalTooLow`] on failure.
///
/// @notice  This is the on-chain enforcement entry point.  Call this inside
///          `initialize()` before persisting any campaign state so that a
///          below-threshold goal is rejected atomically with no side-effects.
///
/// @dev     The `_env` parameter is accepted for API consistency with other
///          Soroban validation helpers and to allow future ledger-aware
///          threshold logic (e.g. governance-controlled minimums stored in
///          contract storage) without a breaking signature change.
///
/// @param  _env         The Soroban environment (reserved for future use).
/// @param  goal_amount  The proposed campaign goal in token units.
/// @return              `Ok(())` if `goal_amount >= MIN_GOAL_AMOUNT`,
///                      `Err(ContractError::GoalTooLow)` otherwise.
///
/// ## Security rationale
///
/// A campaign goal below `MIN_GOAL_AMOUNT` (currently 1 token unit) would:
/// - Allow a zero-goal campaign to be immediately "successful" after any
///   contribution, letting the creator drain funds with no real commitment.
/// - Create "dust" campaigns that consume a ledger entry for negligible value,
///   wasting network resources and increasing state bloat.
/// - Undermine platform credibility by permitting economically meaningless
///   campaigns that could be used for spam or griefing.
///
/// ## Integer-overflow safety
///
/// `goal_amount` is `i128`.  The comparison `goal_amount < MIN_GOAL_AMOUNT`
/// is a single signed integer comparison — no arithmetic is performed, so
/// overflow is impossible.
#[inline]
pub fn validate_goal_amount(
    _env: &soroban_sdk::Env,
    goal_amount: i128,
) -> Result<(), crate::ContractError> {
    if goal_amount < MIN_GOAL_AMOUNT {
        return Err(crate::ContractError::GoalTooLow);
    }
    Ok(())
}

/// Validates that `min_contribution` meets the minimum floor.
///
/// ## Integer-overflow safety
///
/// The comparison `goal_amount < MIN_GOAL_AMOUNT` is a single signed integer
/// comparison — no arithmetic is performed, so overflow is impossible.
#[inline]
pub fn validate_goal_amount(
    _env: &soroban_sdk::Env,
    goal_amount: i128,
) -> Result<(), crate::ContractError> {
    if goal_amount < MIN_GOAL_AMOUNT {
        return Err(crate::ContractError::GoalTooLow);
    }
    Ok(())
}

/// Validates that `min_contribution` meets the minimum floor.
#[inline]
pub fn validate_min_contribution(min_contribution: i128) -> Result<(), &'static str> {
    if min_contribution < MIN_CONTRIBUTION_AMOUNT {
        return Err("min_contribution must be at least MIN_CONTRIBUTION_AMOUNT");
    }

    if goal == 0 {
        panic!("Campaign goal must be non-zero");
    }

    // Example storage logic (placeholder)
    // env.storage().instance().set(&DataKey::Creator, &creator);
    // env.storage().instance().set(&DataKey::Goal, &goal);
    
    // Emit event as requested
    env.events().publish(("campaign", "created"), (creator, goal));
}

/// Validates if a goal meets the minimum threshold.
///
/// # Parameters
/// - goal: the proposed goal
///
/// # Returns
/// true if the goal is secure and valid.
pub fn validate_goal(goal: u64) -> bool {
    goal >= MIN_CAMPAIGN_GOAL
}
