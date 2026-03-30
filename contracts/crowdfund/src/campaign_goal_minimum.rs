//! # Campaign goal minimum threshold
//!
<<<<<<< HEAD
//! @title   CampaignGoalMinimum — Validation helpers for campaign goal and
//!          contribution thresholds.
//!
//! @notice  This module enforces minimum thresholds for campaign goals,
//!          minimum contributions, deadlines, and platform fees.  All
//!          validation is pure (no storage reads) so it can be used in
//!          off-chain tooling and property-based tests without a full
//!          Soroban environment.
//!
//! ## Security Rationale
//!
//! A campaign goal below `MIN_GOAL_AMOUNT` (1 token unit) would allow a
//! zero-goal campaign to be immediately "successful" after any contribution,
//! letting the creator drain funds with no real commitment.  A zero
//! `min_contribution` allows dust attacks that pollute the contributor list
//! and waste ledger storage.
//!
//! ## Integer-Overflow Safety
//!
//! All comparisons in this module are single signed/unsigned integer
//! comparisons — no arithmetic is performed, so overflow is impossible.
//! `validate_deadline` uses `saturating_add` to guard against `u64` overflow
//! when `now` is near `u64::MAX`.
||||||| a43ed59f
//! @title   CampaignGoalMinimum — Enforces minimum campaign goal thresholds.
//!
//! @notice  This module provides the logic to prevent campaigns from being
//!          created with goals below a defined minimum, ensuring realistic
//!          fundraising targets and improving security.
=======
//! Centralized enforcement of minimum campaign goal, contribution floor, deadline
//! offset, platform fee cap, and progress basis points. Prefer
//! [`validate_goal_amount`] for any on-chain path that must return a typed
//! [`crate::ContractError`]; the string-based helpers remain for legacy call sites
//! and off-chain tooling but are deprecated where a typed error exists.
>>>>>>> origin/main

<<<<<<< HEAD
// ── Constants ─────────────────────────────────────────────────────────────────
||||||| a43ed59f
use soroban_sdk::{Address, Env};
=======
use soroban_sdk::Env;
>>>>>>> origin/main

<<<<<<< HEAD
/// Minimum allowed campaign goal (in token units).
///
/// @notice A goal of 0 would make the campaign immediately successful after
///         any contribution, allowing the creator to drain funds trivially.
||||||| a43ed59f
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
=======
// ── Constants ─────────────────────────────────────────────────────────────────

/// @title Minimum funding goal
/// @notice Any campaign goal must be at least this many token base units.
/// @dev A goal of `0` would allow trivial “successful” campaigns with no funding
///      target — a known drain / logic-bypass pattern. The floor `1` is the
///      smallest positive `i128` goal.
/// @custom:security Baked into WASM; changing requires contract upgrade and
///                  coordinated indexer / client updates.
>>>>>>> origin/main
pub const MIN_GOAL_AMOUNT: i128 = 1;

<<<<<<< HEAD
/// Minimum allowed `min_contribution` value (in token units).
///
/// @notice A zero minimum contribution allows dust attacks that waste ledger
///         storage and inflate the contributor count.
pub const MIN_CONTRIBUTION_AMOUNT: i128 = 1;

/// Minimum number of seconds the deadline must be in the future at
/// initialization time.
///
/// @notice Ensures the campaign is live for at least one ledger close
///         interval (~5 s on Stellar mainnet), giving contributors time to act.
pub const MIN_DEADLINE_OFFSET: u64 = 60;

/// Maximum platform fee in basis points (10 000 bps = 100 %).
///
/// @notice A fee above 100 % would leave the creator with a negative payout,
///         which is economically nonsensical and would cause an underflow.
pub const MAX_PLATFORM_FEE_BPS: u32 = 10_000;

/// Scale factor used when computing progress in basis points.
pub const PROGRESS_BPS_SCALE: i128 = 10_000;

/// Maximum value returned by `compute_progress_bps` (100 % = 10 000 bps).
pub const MAX_PROGRESS_BPS: u32 = 10_000;

// ── Validation helpers ────────────────────────────────────────────────────────
||||||| a43ed59f
// ── Validation helpers ───────────────────────────────────────────────────────
=======
/// @title Minimum per-contribution amount
/// @notice Each contribution must be at least this many token base units.
/// @custom:security Prevents zero-amount transfers that spam storage or events.
pub const MIN_CONTRIBUTION_AMOUNT: i128 = 1;
>>>>>>> origin/main

<<<<<<< HEAD
/// Validates that `goal` meets the minimum threshold.
///
/// @param  goal  The proposed campaign goal in token units.
/// @return       `Ok(())` if `goal >= MIN_GOAL_AMOUNT`, `Err` otherwise.
///
/// @dev    Returns `&'static str` so this helper stays free of the contract's
///         error type and can be used in off-chain tooling.
||||||| a43ed59f
/// Validates that `goal` meets the minimum threshold.
///
/// @param  goal  The proposed campaign goal in token units.
/// @return       `Ok(())` if valid, `Err(&'static str)` with a reason otherwise.
///
/// @dev    Returns a `&'static str` rather than `ContractError` so this module
///         stays free of the contract's error type and can be used in off-chain
///         tooling without pulling in the full contract dependency.
=======
/// @title Minimum deadline horizon
/// @notice Deadline must be at least this many seconds after the current ledger time.
/// @custom:security Uses `saturating_add` on `u64` so `now` near `u64::MAX` cannot wrap.
pub const MIN_DEADLINE_OFFSET: u64 = 60;

/// @title Platform fee ceiling
/// @notice Fee in basis points must not exceed this value (10_000 = 100%).
pub const MAX_PLATFORM_FEE_BPS: u32 = 10_000;

/// @title Progress scale
/// @notice Denominator for converting raised/goal ratio to basis points.
pub const PROGRESS_BPS_SCALE: i128 = 10_000;

/// @title Maximum progress basis points
/// @notice Progress UI is capped so over-funded campaigns do not report &gt; 100%.
pub const MAX_PROGRESS_BPS: u32 = 10_000;

// ── Legacy string-error API (deprecated) ─────────────────────────────────────

/// @notice Returns `Ok(())` if `goal >= MIN_GOAL_AMOUNT`.
/// @dev **Deprecated** — use [`validate_goal_amount`] for Soroban paths that map to
///      [`crate::ContractError`]. Kept for backward compatibility and tests.
/// @param goal Campaign goal in token base units.
#[deprecated(
    note = "use validate_goal_amount(&env, goal) and map ContractError for on-chain initialization"
)]
>>>>>>> origin/main
#[inline]
pub fn validate_goal(goal: i128) -> Result<(), &'static str> {
    if goal < MIN_GOAL_AMOUNT {
        return Err("goal must be at least MIN_GOAL_AMOUNT");
    }
    Ok(())
}

<<<<<<< HEAD
/// Validates that `goal_amount` meets the minimum threshold, returning a typed
/// [`crate::ContractError::InvalidGoal`] on failure.
///
/// @notice  This is the on-chain enforcement entry point.  Call this inside
///          `initialize()` before persisting any campaign state so that a
///          below-threshold goal is rejected atomically with no side-effects.
///
/// @dev     `_env` is accepted for API consistency and to allow future
///          ledger-aware threshold logic without a breaking signature change.
///
/// @param  _env         The Soroban environment (reserved for future use).
/// @param  goal_amount  The proposed campaign goal in token units.
/// @return              `Ok(())` if `goal_amount >= MIN_GOAL_AMOUNT`,
///                      `Err(ContractError::InvalidGoal)` otherwise.
||||||| a43ed59f
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
=======
/// @title Canonical on-chain goal floor check
/// @notice Returns `Ok(())` if `goal_amount >= MIN_GOAL_AMOUNT`.
/// @param _env Soroban environment (reserved for future ledger-aware rules).
/// @param goal_amount Campaign goal in token base units.
/// @return `Err(ContractError::GoalTooLow)` if below floor; otherwise `Ok(())`.
/// @dev Single signed comparison — no arithmetic, so no overflow.
/// @custom:security Must run before persisting campaign state that depends on `goal`.
>>>>>>> origin/main
#[inline]
pub fn validate_goal_amount(
    _env: &Env,
    goal_amount: i128,
) -> Result<(), crate::ContractError> {
    if goal_amount < MIN_GOAL_AMOUNT {
        return Err(crate::ContractError::InvalidGoal);
    }
    Ok(())
}

<<<<<<< HEAD
/// Validates that `min_contribution` meets the minimum floor.
///
/// @param  min_contribution  The proposed minimum contribution in token units.
/// @return                   `Ok(())` if valid, `Err` otherwise.
||||||| a43ed59f
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
=======
/// @notice Returns `Ok(())` if `min_contribution >= MIN_CONTRIBUTION_AMOUNT`.
/// @param min_contribution Minimum contribution in token base units.
>>>>>>> origin/main
#[inline]
pub fn validate_min_contribution(min_contribution: i128) -> Result<(), &'static str> {
    if min_contribution < MIN_CONTRIBUTION_AMOUNT {
        return Err("min_contribution must be at least MIN_CONTRIBUTION_AMOUNT");
    }
    Ok(())
}

<<<<<<< HEAD
/// Validates that `deadline` is sufficiently far in the future.
///
/// @param  now       Current ledger timestamp (seconds since epoch).
/// @param  deadline  Proposed campaign deadline (seconds since epoch).
/// @return           `Ok(())` if `deadline >= now + MIN_DEADLINE_OFFSET`, `Err` otherwise.
///
/// @dev    Uses `saturating_add` to prevent `u64` overflow when `now` is near
///         `u64::MAX`.
#[inline]
pub fn validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str> {
    let min_deadline = now.saturating_add(MIN_DEADLINE_OFFSET);
    if deadline < min_deadline {
        return Err("deadline must be at least MIN_DEADLINE_OFFSET seconds in the future");
||||||| a43ed59f
    if goal == 0 {
        panic!("Campaign goal must be non-zero");
=======
/// @notice Returns `Ok(())` if `deadline >= now + MIN_DEADLINE_OFFSET` (saturating).
/// @param now Current ledger timestamp (seconds).
/// @param deadline Campaign deadline (seconds).
#[inline]
pub fn validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str> {
    let min_deadline = now.saturating_add(MIN_DEADLINE_OFFSET);
    if deadline < min_deadline {
        return Err("deadline must be at least MIN_DEADLINE_OFFSET seconds in the future");
    }
    Ok(())
}

/// @notice Returns `Ok(())` if `fee_bps <= MAX_PLATFORM_FEE_BPS`.
/// @param fee_bps Platform fee in basis points.
#[inline]
pub fn validate_platform_fee(fee_bps: u32) -> Result<(), &'static str> {
    if fee_bps > MAX_PLATFORM_FEE_BPS {
        return Err("fee_bps must not exceed MAX_PLATFORM_FEE_BPS");
    }
    Ok(())
}

// ── Progress computation ─────────────────────────────────────────────────────

/// @title Funding progress in basis points
/// @notice Computes `min(10_000, (total_raised * PROGRESS_BPS_SCALE) / goal)`.
/// @param total_raised Sum of contributions in token base units.
/// @param goal Campaign goal in token base units.
/// @return Basis points from 0 through [`MAX_PROGRESS_BPS`].
/// @dev Returns `0` if `goal <= 0` or `total_raised <= 0`. Uses `saturating_mul` so
///      `total_raised * PROGRESS_BPS_SCALE` never panics in debug builds when raised
///      is huge (e.g. `i128::MAX` with `goal == 1`); the quotient is then capped at
///      [`MAX_PROGRESS_BPS`].
#[inline]
pub fn compute_progress_bps(total_raised: i128, goal: i128) -> u32 {
    if total_raised <= 0 || goal <= 0 {
        return 0;
>>>>>>> origin/main
    }
<<<<<<< HEAD
    Ok(())
}

/// Validates that `fee_bps` does not exceed `MAX_PLATFORM_FEE_BPS`.
///
/// @param  fee_bps  Platform fee in basis points.
/// @return          `Ok(())` if `fee_bps <= MAX_PLATFORM_FEE_BPS`, `Err` otherwise.
#[inline]
pub fn validate_platform_fee(fee_bps: u32) -> Result<(), &'static str> {
    if fee_bps > MAX_PLATFORM_FEE_BPS {
        return Err("fee_bps must not exceed MAX_PLATFORM_FEE_BPS");
    }
    Ok(())
}

/// Computes campaign progress in basis points, capped at `MAX_PROGRESS_BPS`.
///
/// @param  total_raised  Total tokens raised so far.
/// @param  goal          Campaign funding goal.
/// @return               Progress in bps (0–10 000).  Returns 0 if `goal <= 0`.
///
/// @dev    Integer division truncates toward zero, so 1 token raised against a
///         1 000 000-token goal returns 0 bps (< 1 bps).
#[inline]
pub fn compute_progress_bps(total_raised: i128, goal: i128) -> u32 {
    if goal <= 0 {
        return 0;
    }
    let raw = (total_raised * PROGRESS_BPS_SCALE) / goal;
    if raw >= PROGRESS_BPS_SCALE {
        MAX_PROGRESS_BPS
    } else if raw < 0 {
        0
    } else {
        raw as u32
||||||| a43ed59f

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
=======

    let raw_progress = total_raised.saturating_mul(PROGRESS_BPS_SCALE) / goal;
    if raw_progress <= 0 {
        0
    } else if raw_progress >= MAX_PROGRESS_BPS as i128 {
        MAX_PROGRESS_BPS
    } else {
        raw_progress as u32
>>>>>>> origin/main
    }
}
