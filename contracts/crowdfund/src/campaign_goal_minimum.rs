// campaign_goal_minimum — Minimum threshold enforcement for campaign goals.
//
// Security Assumptions:
// 1. MIN_GOAL_AMOUNT >= 1 closes the zero-goal drain exploit.
// 2. Negative goals are rejected by the < MIN_GOAL_AMOUNT comparison.
// 3. No integer overflow — only comparisons and saturating_add are used.
// 4. validate_goal_amount is called before any env.storage() write.
// 5. Constants are baked into WASM; changes require a contract upgrade.

use soroban_sdk::{Address, Env};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Minimum campaign goal in token units.
/// A goal of zero enables a trivial drain exploit; 1 closes that surface.
pub const MIN_GOAL_AMOUNT: i128 = 1;

/// Minimum contribution amount in token units.
pub const MIN_CONTRIBUTION_AMOUNT: i128 = 1;
/// @notice Minimum allowed `min_contribution` value in token units.
///
/// @dev    Prevents contributions of 0 tokens, which would allow an attacker
///         to register as a contributor without transferring any value.
pub const MIN_CONTRIBUTION_AMOUNT: i128 = 1;

/// @notice Maximum allowed platform fee in basis points (100% = 10_000 bps).
///
/// # Security
/// Ensures goal meets minimum threshold and creator is authenticated.
pub fn create_campaign(env: Env, creator: Address, goal: u64) {
    creator.require_auth();
    if goal < MIN_CAMPAIGN_GOAL {
        panic!("Goal too low");
    }
    env.events()
        .publish(("campaign", "created"), (creator, goal));
}

/// Minimum seconds a deadline must be ahead of the current ledger timestamp.
pub const MIN_DEADLINE_OFFSET: u64 = 60;

/// Maximum platform fee in basis points (10 000 bps = 100 %).
pub const MAX_PLATFORM_FEE_BPS: u32 = 10_000;

/// Denominator used when computing progress in basis points.
pub const PROGRESS_BPS_SCALE: i128 = 10_000;

/// Maximum value returned by compute_progress_bps.
pub const MAX_PROGRESS_BPS: u32 = 10_000;

// ── Off-chain / string-error validators ──────────────────────────────────────

/// Validates that goal meets the minimum threshold.
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
// ── Constants ────────────────────────────────────────────────────────────────

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

    if goal < MIN_CAMPAIGN_GOAL {
        panic!("Minimum campaign goal not met");
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

/// Validates that `goal_amount` meets the minimum threshold, returning a typed
/// [`crate::ContractError::GoalTooLow`] on failure.
///
/// @notice  On-chain enforcement entry point. Call inside `initialize()` before
///          any state is written so a rejected goal leaves no partial storage.
///
/// @dev     The `_env` parameter is reserved for future governance-controlled
///          thresholds stored in contract storage, without a breaking signature change.
///
/// @param  _env         The Soroban environment (reserved for future use).
/// @param  goal_amount  The proposed campaign goal in token units.
/// @return              `Ok(())` if `goal_amount >= MIN_GOAL_AMOUNT`,
///                      `Err(ContractError::GoalTooLow)` otherwise.
///
/// ## Security rationale
///
/// A goal below `MIN_GOAL_AMOUNT` would:
/// - Allow a zero-goal campaign to be immediately "successful" after any
///   contribution, letting the creator drain funds with no real commitment.
/// - Create "dust" campaigns that consume a ledger entry for negligible value,
///   wasting network resources and increasing state bloat.
/// - Undermine platform credibility by permitting economically meaningless
///   campaigns usable for spam or griefing.
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
/// @param  min_contribution  The proposed minimum contribution in token units.
/// @return                   `Ok(())` if valid, `Err` otherwise.
#[inline]
pub fn validate_min_contribution(min_contribution: i128) -> Result<(), &'static str> {
    if min_contribution < MIN_CONTRIBUTION_AMOUNT {
        return Err("min_contribution must be at least MIN_CONTRIBUTION_AMOUNT");
    }
    Ok(())
}

/// Validates that deadline is sufficiently far in the future.
#[inline]
pub fn validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str> {
    let min_deadline = now.saturating_add(MIN_DEADLINE_OFFSET);
    if deadline < min_deadline {
/// Validates that the campaign deadline is sufficiently far in the future.
///
/// @param  now       Current ledger timestamp (seconds since Unix epoch).
/// @param  deadline  Proposed campaign deadline timestamp.
/// @return           `Ok(())` if `deadline >= now + MIN_DEADLINE_OFFSET`.
#[inline]
pub fn validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str> {
    if deadline < now.saturating_add(MIN_DEADLINE_OFFSET) {
        return Err("deadline must be at least MIN_DEADLINE_OFFSET seconds in the future");
    }
    Ok(())
}

/// Validates that fee_bps does not exceed the platform fee cap.
#[inline]
pub fn validate_platform_fee(fee_bps: u32) -> Result<(), &'static str> {
    if fee_bps > MAX_PLATFORM_FEE_BPS {
        return Err("fee_bps must not exceed MAX_PLATFORM_FEE_BPS");
/// Validates that a platform fee does not exceed the maximum allowed.
///
/// @param  fee_bps  Platform fee in basis points.
/// @return          `Ok(())` if `fee_bps <= MAX_PLATFORM_FEE_BPS`.
#[inline]
pub fn validate_platform_fee(fee_bps: u32) -> Result<(), &'static str> {
    if fee_bps > MAX_PLATFORM_FEE_BPS {
        return Err("platform fee cannot exceed MAX_PLATFORM_FEE_BPS (100%)");
    }
    Ok(())
}

// ── On-chain / typed-error validator ─────────────────────────────────────────

/// @notice Computes campaign funding progress in basis points.
///
/// Security: A zero-goal campaign is immediately "successful" after any
/// contribution, letting the creator drain funds with no real commitment.
/// Integer-overflow safety: single signed comparison, no arithmetic.
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

// ── Progress computation ─────────────────────────────────────────────────────

/// Computes campaign progress in basis points (0–10 000).
/// Returns 0 if goal <= 0.
/// Caps at MAX_PROGRESS_BPS even when total_raised > goal (over-funded).
/// Uses integer division; precision loss is acceptable for UI display.
/// @dev    `progress_bps = (total_raised * PROGRESS_BPS_SCALE) / goal`.
///         Result is capped at `MAX_PROGRESS_BPS` for over-funded campaigns.
///         Returns 0 when `goal <= 0` to avoid division by zero.
///
/// @param  total_raised  Total tokens raised so far.
/// @param  goal          Campaign funding goal.
/// @return               Progress in basis points, capped at `MAX_PROGRESS_BPS`.
///
/// @custom:security Uses `saturating_mul` to prevent overflow on very large
///         `total_raised` values. The cap ensures the return value is always
///         in `[0, MAX_PROGRESS_BPS]`.
/// Computes campaign progress in basis points, capped at [`MAX_PROGRESS_BPS`].
///
/// @param  total_raised  Amount raised so far (token units).
/// @param  goal          Campaign goal (token units, must be > 0).
/// @return               Progress in bps in the range `[0, MAX_PROGRESS_BPS]`.
///
/// @dev    Returns 0 when `goal == 0` to avoid division by zero; callers
///         should ensure `goal >= MIN_GOAL_AMOUNT` before calling.
#[inline]
pub fn compute_progress_bps(total_raised: i128, goal: i128) -> u32 {
    if goal <= 0 || total_raised <= 0 {
        return 0;
    }
    let progress = (total_raised * PROGRESS_BPS_SCALE) / goal;
    if progress > MAX_PROGRESS_BPS as i128 {
        MAX_PROGRESS_BPS
    } else {
        progress as u32
    }
}

/// Creates a new campaign with goal validation.
///
/// # Parameters
/// - creator: campaign owner
/// - goal: funding target
///
/// # Security
/// Ensures goal meets minimum threshold and creator is authenticated.
pub fn create_campaign(env: soroban_sdk::Env, creator: soroban_sdk::Address, goal: u64) {
    creator.require_auth();
    if goal < MIN_CAMPAIGN_GOAL {
        panic!("Goal too low");
    }
    env.events().publish(("campaign", "created"), (creator, goal));
    let raw = total_raised.saturating_mul(PROGRESS_BPS_SCALE) / goal;
    if raw >= PROGRESS_BPS_SCALE {
        return MAX_PROGRESS_BPS;
    }
    raw.max(0) as u32
}

const MIN_CAMPAIGN_GOAL: u64 = 1;
    let raw = (total_raised * PROGRESS_BPS_SCALE) / goal;
    let raw = total_raised.saturating_mul(PROGRESS_BPS_SCALE) / goal;
    if raw > PROGRESS_BPS_SCALE {

    let scaled = total_raised
        .checked_mul(PROGRESS_BPS_SCALE)
        .unwrap_or(i128::MAX);
    let raw = scaled / goal;

    if raw >= PROGRESS_BPS_SCALE {
        MAX_PROGRESS_BPS
    } else {
        raw as u32
    }
/// # Parameters
/// - goal: the proposed goal
///
/// # Returns
/// true if the goal is secure and valid.
pub fn validate_goal(goal: u64) -> bool {
    goal >= MIN_CAMPAIGN_GOAL
}
