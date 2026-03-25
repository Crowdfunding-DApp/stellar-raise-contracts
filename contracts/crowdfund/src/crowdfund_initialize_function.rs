//! # crowdfund_initialize_function
//!
//! @title   CrowdfundInitializeFunction — Validated initialization logic.
//!
//! @notice  Extracts `initialize()` logic into a single auditable location.
//!          Validates all parameters before any storage write (atomic CEI).
//!
//! ## Security Assumptions
//!
//! 1. Re-initialization guard uses `DataKey::Creator` as sentinel.
//! 2. `creator.require_auth()` called before any state mutation.
//! 3. All validations complete before the first storage write.
//! 4. `bonus_goal > goal` enforced to prevent trivially-met bonus goals.

#![allow(dead_code)]

use soroban_sdk::{Address, Env, String, Vec};

use crate::campaign_goal_minimum::{
    validate_deadline, validate_goal, validate_min_contribution, validate_platform_fee,
    MIN_GOAL_AMOUNT,
};
use crate::{ContractError, DataKey, PlatformConfig, RoadmapItem, Status};

// ── InitParams ────────────────────────────────────────────────────────────────

/// All parameters required to initialize a crowdfund campaign.
///
/// @dev Named struct prevents silent parameter-order bugs.
#[derive(Clone)]
pub struct InitParams {
    /// Admin address authorized to upgrade the contract.
    pub admin: Address,
    /// Campaign creator's address (must authorize the call).
    pub creator: Address,
    /// SEP-41 token contract address used for contributions.
    pub token: Address,
    /// Funding goal in the token's smallest unit (>= MIN_GOAL_AMOUNT).
    pub goal: i128,
    /// Campaign deadline as a Unix timestamp (>= now + MIN_DEADLINE_OFFSET).
    pub deadline: u64,
    /// Minimum contribution amount (>= MIN_CONTRIBUTION_AMOUNT).
    pub min_contribution: i128,
    /// Optional platform fee configuration (fee_bps <= MAX_PLATFORM_FEE_BPS).
    pub platform_config: Option<PlatformConfig>,
    /// Optional bonus goal threshold (must be > goal when Some).
    pub bonus_goal: Option<i128>,
    /// Optional human-readable description for the bonus goal.
    pub bonus_goal_description: Option<String>,
}

// ── Validation helpers ────────────────────────────────────────────────────────

/// Validates that `bonus_goal`, when present, is strictly greater than `goal`.
///
/// @param bonus_goal The optional bonus goal value.
/// @param goal       The primary campaign goal.
/// @return `Ok(())` if valid or absent; `Err(ContractError::InvalidBonusGoal)` otherwise.
#[inline]
pub fn validate_bonus_goal(bonus_goal: Option<i128>, goal: i128) -> Result<(), ContractError> {
    if let Some(bg) = bonus_goal {
        if bg <= goal {
            return Err(ContractError::InvalidBonusGoal);
        }
    }
    Ok(())
}

/// Validates all `InitParams` fields in a single pass.
///
/// @param env    The Soroban execution environment.
/// @param params The initialization parameters to validate.
/// @return `Ok(())` if all fields are valid; first `ContractError` otherwise.
pub fn validate_init_params(env: &Env, params: &InitParams) -> Result<(), ContractError> {
    validate_goal(params.goal).map_err(|_| ContractError::InvalidGoal)?;
    validate_min_contribution(params.min_contribution)
        .map_err(|_| ContractError::InvalidMinContribution)?;
    validate_deadline(env.ledger().timestamp(), params.deadline)
        .map_err(|_| ContractError::DeadlineTooSoon)?;
    if let Some(ref config) = params.platform_config {
        validate_platform_fee(config.fee_bps).map_err(|_| ContractError::InvalidPlatformFee)?;
    }
    validate_bonus_goal(params.bonus_goal, params.goal)?;
    Ok(())
}

// ── Core initialization logic ─────────────────────────────────────────────────

/// Executes the full campaign initialization flow.
///
/// @notice Single authoritative implementation. `CrowdfundContract::initialize()`
///         delegates here after constructing `InitParams`.
///
/// @param env    The Soroban execution environment.
/// @param params Fully-populated initialization parameters.
/// @return `Ok(())` on success; typed `ContractError` on failure.
///
/// @security Ordering: guard → auth → validate → write → emit.
pub fn execute_initialize(env: &Env, params: InitParams) -> Result<(), ContractError> {
    // 1. Re-initialization guard
    if env.storage().instance().has(&DataKey::Creator) {
        return Err(ContractError::AlreadyInitialized);
    }

    // 2. Creator authentication (before any state mutation)
    params.creator.require_auth();

    // 3. Parameter validation (no storage writes yet)
    validate_init_params(env, &params)?;

    // 4. Storage writes
    env.storage().instance().set(&DataKey::Admin, &params.admin);
    env.storage().instance().set(&DataKey::Creator, &params.creator);
    env.storage().instance().set(&DataKey::Token, &params.token);
    env.storage().instance().set(&DataKey::Goal, &params.goal);
    env.storage().instance().set(&DataKey::Deadline, &params.deadline);
    env.storage().instance().set(&DataKey::MinContribution, &params.min_contribution);
    env.storage().instance().set(&DataKey::TotalRaised, &0i128);
    env.storage().instance().set(&DataKey::BonusGoalReachedEmitted, &false);
    env.storage().instance().set(&DataKey::Status, &Status::Active);

    if let Some(ref config) = params.platform_config {
        env.storage().instance().set(&DataKey::PlatformConfig, config);
    }
    if let Some(bg) = params.bonus_goal {
        env.storage().instance().set(&DataKey::BonusGoal, &bg);
    }
    if let Some(ref bg_desc) = params.bonus_goal_description {
        env.storage().instance().set(&DataKey::BonusGoalDescription, bg_desc);
    }

    let empty_contributors: Vec<Address> = Vec::new(env);
    env.storage().persistent().set(&DataKey::Contributors, &empty_contributors);

    let empty_roadmap: Vec<RoadmapItem> = Vec::new(env);
    env.storage().instance().set(&DataKey::Roadmap, &empty_roadmap);

    // 5. Event emission
    env.events().publish(
        (
            soroban_sdk::Symbol::new(env, "campaign"),
            soroban_sdk::Symbol::new(env, "initialized"),
        ),
        (
            params.creator.clone(),
            params.token.clone(),
            params.goal,
            params.deadline,
            params.min_contribution,
        ),
    );

    Ok(())
}

// ── Error description helpers ─────────────────────────────────────────────────

/// Returns a human-readable description for an `initialize()`-related error code.
///
/// @param code The numeric `ContractError` repr value.
/// @return A static string for display in frontend error messages.
pub fn describe_init_error(code: u32) -> &'static str {
    match code {
        1 => "Contract is already initialized",
        8 => "Campaign goal must be at least 1",
        9 => "Minimum contribution must be at least 1",
        10 => "Deadline must be at least 60 seconds in the future",
        11 => "Platform fee cannot exceed 100% (10,000 bps)",
        12 => "Bonus goal must be strictly greater than the primary goal",
        _ => "Unknown initialization error",
    }
}

/// Returns `true` if the error code is a correctable input error.
pub fn is_init_error_retryable(code: u32) -> bool {
    matches!(code, 8 | 9 | 10 | 11 | 12)
}

pub use crate::campaign_goal_minimum::MIN_GOAL_AMOUNT as INIT_MIN_GOAL_AMOUNT;
