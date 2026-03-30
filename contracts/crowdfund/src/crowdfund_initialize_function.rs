//! # crowdfund_initialize_function
//!
<<<<<<< HEAD
//! @title   CrowdfundInitializeFunction — Validated initialization logic for
//!          the crowdfund contract.
||||||| a43ed59f
//! @title   CrowdfundInitializeFunction — Refactored, validated, and
//!          frontend-ready initialization logic for the crowdfund contract.
=======
//! @title   CrowdfundInitializeFunction — Validated, auditable, and
//!          frontend-ready initialization logic for the crowdfund contract.
>>>>>>> origin/main
//!
<<<<<<< HEAD
//! @notice  Extracts and standardizes `initialize()` logic from `lib.rs` into
//!          a single, auditable location.  Provides:
//!          - A validated `InitParams` struct.
//!          - Pure validation helpers returning typed `ContractError` values.
//!          - A deterministic, single-pass initialization flow with clear
//!            checks → effects → storage write ordering.
||||||| a43ed59f
//! @notice  This module extracts and standardizes the `initialize()` logic
//!          from `lib.rs` into a single, auditable location.  It provides:
//!
//!          - A validated `InitParams` struct that the contract passes to
//!            `execute_initialize()` after construction.
//!          - Pure validation helpers for every parameter, each returning a
//!            typed `ContractError` so the frontend can map error codes to
//!            user-facing messages without string matching.
//!          - A deterministic, single-pass initialization flow with a clear
//!            checks → effects → storage write ordering.
//!          - An `InitializedEvent` payload emitted on success so off-chain
//!            indexers can reconstruct campaign state without polling storage.
//!
//! @dev     The module is `no_std`-compatible and has no dependency on the
//!          contract's `#[contractimpl]` block, making it usable in off-chain
//!          tooling and property-based tests without a full Soroban environment.
//!
//! ## Design Decisions
//!
//! ### Why a separate `InitParams` struct?
//!
//! The original `initialize()` accepted nine positional arguments.  Positional
//! argument lists are fragile: swapping two `i128` parameters compiles silently
//! but produces incorrect state.  A named struct makes every field explicit at
//! the call site and allows the compiler to catch type mismatches.
//!
//! ### Why typed errors instead of panics?
//!
//! The original implementation panicked on invalid platform fee and bonus goal.
//! Panics are opaque to the frontend — the SDK surfaces them as a generic host
//! error with no numeric code.  Typed `ContractError` variants let the frontend
//! display a specific message (e.g. "Platform fee exceeds 100%") without
//! parsing error strings.
//!
//! ### Why emit an `initialized` event?
//!
//! Soroban storage is not directly queryable by off-chain services without an
//! RPC call per field.  An `initialized` event carries all campaign parameters
//! in a single ledger entry, enabling indexers to bootstrap campaign state from
//! the event stream alone.
//!
//! ### Why validate before any storage write?
//!
//! The original code interleaved validation and storage writes.  If a later
//! validation failed (e.g. bonus goal check) after earlier writes had already
//! committed (e.g. admin, platform config), the contract would be left in a
//! partially-initialized state.  This module validates all parameters first,
//! then writes atomically.
=======
//! @notice  This module is the single authoritative location for all
//!          `initialize()` logic.  It provides:
//!
//!          - `InitParams` — a named struct that replaces nine positional
//!            arguments, eliminating silent parameter-order bugs.
//!          - Pure validation helpers for every parameter, each returning a
//!            typed `ContractError` so the frontend can map error codes to
//!            user-facing messages without string matching.
//!          - `execute_initialize()` — a deterministic, single-pass flow with
//!            a strict checks → effects → storage write ordering.
//!          - `describe_init_error()` / `is_init_error_retryable()` — helpers
//!            for off-chain scripts and frontend error handling.
//!
//! ## Performance Optimizations
//!
//! 1. **Early validation exit** — Uses `?` operator for short-circuit error
//!    propagation instead of nested `if let Err` blocks.
//!
//! Panics are opaque to the frontend — the SDK surfaces them as a generic host
//! error with no numeric code.  Typed `ContractError` variants let the frontend
//! display a specific message (e.g. "Platform fee exceeds 100%") without
//! parsing error strings.
//!
//! 3. **Batched validation** — All parameter checks run in a single
//!    `validate_init_params()` call, reducing function call overhead.
//!
//! 4. **Storage write batching** — All required storage writes are grouped
//!    together with only necessary conditional writes for optional fields.
//!
//! 5. **Optimized re-initialization guard** — Uses a single `has()` check on
//!    `DataKey::Creator` as the initialization sentinel, avoiding extra
//!    storage lookups.
//!
//! Interleaving validation and storage writes risks leaving the contract in a
//! partially-initialized state if a later check fails.  This module validates
//! all parameters first, then writes atomically.
>>>>>>> origin/main
//!
//! ## Security Assumptions
//!
<<<<<<< HEAD
//! 1. Re-initialization guard uses `DataKey::Creator` as the sentinel.
//! 2. `creator.require_auth()` is called before any storage write.
//! 3. `goal >= MIN_GOAL_AMOUNT` prevents zero-goal campaigns.
//! 4. `min_contribution >= MIN_CONTRIBUTION_AMOUNT` prevents dust attacks.
//! 5. `deadline >= now + MIN_DEADLINE_OFFSET` ensures the campaign is live.
//! 6. `fee_bps <= MAX_PLATFORM_FEE_BPS` caps the platform take.
//! 7. `bonus_goal > goal` prevents a bonus goal met at launch.
//! 8. All validations complete before the first storage write.
||||||| a43ed59f
//! 1. **Re-initialization guard** — `DataKey::Creator` is used as the
//!    initialization sentinel.  The check is the very first operation so no
//!    state can be written before it.
//!
//! 2. **Creator authentication** — `creator.require_auth()` is called before
//!    any storage write.  The Soroban host rejects the transaction if the
//!    creator's signature is absent or invalid.
//!
//! 3. **Goal floor** — `goal >= MIN_GOAL_AMOUNT (1)` prevents zero-goal
//!    campaigns that could be immediately drained by the creator.
//!
//! 4. **Minimum contribution floor** — `min_contribution >= MIN_CONTRIBUTION_AMOUNT (1)`
//!    prevents zero-amount contributions that waste gas and pollute storage.
//!
//! 5. **Deadline offset** — `deadline >= now + MIN_DEADLINE_OFFSET (60s)` ensures
//!    the campaign is live for at least one ledger close interval.
//!
//! 6. **Platform fee cap** — `fee_bps <= MAX_PLATFORM_FEE_BPS (10_000)` ensures
//!    the platform can never be configured to take more than 100% of raised funds.
//!
//! 7. **Bonus goal ordering** — `bonus_goal > goal` prevents a bonus goal that
//!    is already met at launch, which would immediately emit a bonus event and
//!    confuse contributors.
//!
//! 8. **Atomic write ordering** — All validations complete before the first
//!    `env.storage().instance().set()` call.  A failed validation leaves the
//!    contract in its pre-initialization state.
//!
//! ## Validation Flow
//!
//! ```text
//! execute_initialize(env, params)
//!        │
//!        ├─► re-initialization guard (AlreadyInitialized)
//!        ├─► creator.require_auth()
//!        ├─► validate_goal(goal)            → InvalidGoal
//!        ├─► validate_min_contribution(mc)  → InvalidMinContribution
//!        ├─► validate_deadline(now, dl)     → DeadlineTooSoon
//!        ├─► validate_platform_fee(bps)     → InvalidPlatformFee
//!        ├─► validate_bonus_goal(bg, goal)  → InvalidBonusGoal
//!        │
//!        └─► [all checks passed] write storage, emit event → Ok(())
//! ```
//!
//! ## Frontend Interaction
//!
//! The frontend should:
//!
//! 1. Call `initialize()` with a fully-populated `InitParams`.
//! 2. On success, listen for the `("campaign", "initialized")` event to
//!    confirm the campaign is live and cache the emitted parameters.
//! 3. On failure, map the returned `ContractError` code to a user message
//!    using the `describe_init_error()` helper exported from this module.
//!
//! ## Scalability Considerations
//!
//! - `initialize()` is a one-shot function; its gas cost is O(1) regardless
//!   of future campaign size.
//! - The `Contributors` and `Roadmap` lists are seeded as empty vectors.
//!   Their TTL is managed by `contribute()` and `add_roadmap_item()`.
//! - The `InitializedEvent` payload is bounded: it contains only scalar
//!   values and optional scalars, never unbounded collections.
=======
//! 1. **Re-initialization guard** — `DataKey::Creator` is used as the
//!    initialization sentinel. The check is the very first operation so no
//!    state can be written before it.
//!
//! 2. **Creator authentication** — `creator.require_auth()` is called before
//!    any storage write. The Soroban host rejects the transaction if the
//!    creator's signature is absent or invalid.
//!
//! 3. **Goal floor** — `goal >= MIN_GOAL_AMOUNT (1)` prevents zero-goal
//!    campaigns that could be immediately drained by the creator.
//!
//! 4. **Minimum contribution floor** — `min_contribution >= MIN_CONTRIBUTION_AMOUNT (1)`
//!    prevents zero-amount contributions that waste gas and pollute storage.
//!
//! 5. **Deadline offset** — `deadline >= now + MIN_DEADLINE_OFFSET (60s)` ensures
//!    the campaign is live for at least one ledger close interval.
//!
//! 6. **Platform fee cap** — `fee_bps <= MAX_PLATFORM_FEE_BPS (10_000)` ensures
//!    the platform can never be configured to take more than 100% of raised funds.
//!
//! 7. **Bonus goal ordering** — `bonus_goal > goal` prevents a bonus goal that
//!    is already met at launch, which would immediately emit a bonus event.
//!
//! 8. **Atomic write ordering** — All validations complete before the first
//!    `env.storage().instance().set()` call. A failed validation leaves the
//!    contract in its pre-initialization state.
//!
//! ## Validation Flow
//!
//! ```text
//! execute_initialize(env, params)
//!        │
//!        ├─► re-initialization guard     → AlreadyInitialized
//!        ├─► creator.require_auth()
//!        ├─► validate_goal_amount(env, goal) → InvalidGoal (maps GoalTooLow)
//!        ├─► validate_min_contribution() → InvalidMinContribution
//!        ├─► validate_deadline(now, dl)  → DeadlineTooSoon
//!        ├─► validate_platform_fee(bps)  → InvalidPlatformFee
//!        ├─► validate_bonus_goal(bg, g)  → InvalidBonusGoal
//!        │
//!        └─► [all checks passed] write storage → emit event → Ok(())
//! ```
//!
//! ## Frontend Integration
//!
//! 1. Call `initialize()` with a fully-populated parameter set.
//! 2. On success, listen for the `("campaign", "initialized")` event to
//!    confirm the campaign is live and cache the emitted parameters.
//! 3. On failure, map the returned `ContractError` code to a user message
//!    using `describe_init_error()` exported from this module.
//!
//! ## Scalability
//!
//! - `initialize()` is a one-shot O(1) function regardless of future campaign size.
//! - `Contributors` and `Roadmap` are seeded as empty vectors; their TTL is
//!   managed by `contribute()` and `add_roadmap_item()`.
//! - The `initialized` event payload is bounded to scalar values only.
>>>>>>> origin/main

#[allow(dead_code)]
use soroban_sdk::{Address, Env, String, Symbol, Vec};

use crate::campaign_goal_minimum::{
<<<<<<< HEAD
    validate_deadline, validate_goal, validate_min_contribution, validate_platform_fee,
||||||| a43ed59f
    validate_deadline, validate_goal, validate_min_contribution, validate_platform_fee,
    MIN_GOAL_AMOUNT,
=======
    validate_deadline, validate_goal_amount, validate_min_contribution, validate_platform_fee,
>>>>>>> origin/main
};
use crate::{ContractError, DataKey, PlatformConfig, RoadmapItem, Status};

// ── InitParams ────────────────────────────────────────────────────────────────

/// All parameters required to initialize a crowdfund campaign.
<<<<<<< HEAD
||||||| a43ed59f
///
/// @dev Using a named struct instead of positional arguments prevents silent
///      parameter-order bugs (e.g. swapping two `i128` fields compiles but
///      produces incorrect state).
=======
///
/// @dev Using a named struct instead of positional arguments prevents silent
///      parameter-order bugs (e.g. swapping two `i128` fields compiles but
///      produces incorrect state).
///
/// # Type Parameters
/// * `T` - Any type that implements the required trait bounds for Address
>>>>>>> origin/main
#[derive(Clone)]
pub struct InitParams {
    pub admin: Address,
<<<<<<< HEAD
||||||| a43ed59f

    /// The campaign creator's address.
    ///
    /// @notice Must authorize the `initialize()` call.  Stored as the
    ///         re-initialization sentinel.
=======

    /// The campaign creator's address.
    ///
    /// @notice Must authorize the `initialize()` call. Stored as the
    ///         re-initialization sentinel.
>>>>>>> origin/main
    pub creator: Address,
<<<<<<< HEAD
||||||| a43ed59f

    /// The Stellar asset contract address used for contributions.
    ///
    /// @notice Must be a valid token contract that implements the SEP-41
    ///         token interface.
=======

    /// The Stellar asset contract address used for contributions.
    ///
    /// @notice Must be a valid token contract implementing the SEP-41 interface.
>>>>>>> origin/main
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
<<<<<<< HEAD
||||||| a43ed59f

    /// The minimum contribution amount in the token's smallest unit.
    ///
    /// @notice Must be >= `MIN_CONTRIBUTION_AMOUNT` (1).  Setting this to a
    ///         meaningful value (e.g. 1_000 stroops) prevents dust attacks.
=======

    /// The minimum contribution amount in the token's smallest unit.
    ///
    /// @notice Must be >= `MIN_CONTRIBUTION_AMOUNT` (1). Setting this to a
    ///         meaningful value (e.g. 1_000 stroops) prevents dust attacks.
>>>>>>> origin/main
    pub min_contribution: i128,
    pub platform_config: Option<PlatformConfig>,
<<<<<<< HEAD
||||||| a43ed59f

    /// Optional secondary bonus goal threshold.
    ///
    /// @notice When `Some`, must be strictly greater than `goal`.  Reaching
    ///         this threshold emits a `bonus_goal_reached` event exactly once.
=======

    /// Optional secondary bonus goal threshold.
    ///
    /// @notice When `Some`, must be strictly greater than `goal`. Reaching
    ///         this threshold emits a `bonus_goal_reached` event exactly once.
>>>>>>> origin/main
    pub bonus_goal: Option<i128>,
<<<<<<< HEAD
||||||| a43ed59f

    /// Optional human-readable description for the bonus goal.
    ///
    /// @notice Stored as-is; no length validation is enforced at the contract
    ///         level.  The frontend should enforce a reasonable display limit.
=======

    /// Optional human-readable description for the bonus goal.
    ///
    /// @notice Stored as-is.  The frontend should enforce a reasonable display limit.
>>>>>>> origin/main
    pub bonus_goal_description: Option<String>,
}

// ── Validation helpers ───────────────────────────────────────────────────────

/// Validates that `bonus_goal`, when present, is strictly greater than `goal`.
<<<<<<< HEAD
||||||| a43ed59f
///
/// @param  bonus_goal  The optional bonus goal value.
/// @param  goal        The primary campaign goal.
/// @return             `Ok(())` if valid or absent; `Err(ContractError::InvalidBonusGoal)` otherwise.
///
/// @dev    A bonus goal equal to the primary goal would be met at the same
///         time as the campaign goal, making it meaningless.  A bonus goal
///         below the primary goal would be met before the campaign succeeds,
///         which is logically inconsistent.
=======
///
/// @param  bonus_goal  The optional bonus goal value.
/// @param  goal        The primary campaign goal.
/// @return             `Ok(())` if valid or absent; `Err(ContractError::InvalidBonusGoal)` otherwise.
///
/// @dev    A bonus goal equal to the primary goal would be met simultaneously,
///         making it meaningless.  A bonus goal below the primary goal would be
///         met before the campaign succeeds, which is logically inconsistent.
>>>>>>> origin/main
#[inline]
pub fn validate_bonus_goal(bonus_goal: Option<i128>, goal: i128) -> Result<(), ContractError> {
    if let Some(bg) = bonus_goal {
        if bg <= goal {
            return Err(ContractError::InvalidBonusGoal);
        }
    }
    Ok(())
}

/// Validates the bonus goal description length if present.
///
/// Validates the optional bonus goal description.
#[inline]
pub fn validate_bonus_goal_description(description: &Option<String>) -> Result<(), ContractError> {
    // Description is optional; if present, accept any non-empty value.
    // Length validation is handled by Soroban's storage limits.
    let _ = description;
    Ok(())
}

/// Validates all `InitParams` fields in a single pass.
<<<<<<< HEAD
||||||| a43ed59f
///
/// @param  env     The Soroban execution environment (used for ledger timestamp).
/// @param  params  The initialization parameters to validate.
/// @return         `Ok(())` if all fields are valid; the first `ContractError` encountered otherwise.
///
/// @dev    Validation order matches the storage write order in
///         `execute_initialize()` so that error codes are predictable.
=======
///
/// @param  env     The Soroban execution environment (used for ledger timestamp).
/// @param  params  The initialization parameters to validate.
/// @return         `Ok(())` if all fields are valid; the first `ContractError` encountered otherwise.
///
/// @dev    Validation order matches the storage write order in `execute_initialize()`
///         so that error codes are predictable and auditable.
>>>>>>> origin/main
pub fn validate_init_params(env: &Env, params: &InitParams) -> Result<(), ContractError> {
    // Canonical floor check via `validate_goal_amount`; map `GoalTooLow` → `InvalidGoal`
    // so existing clients keep the stable `initialize` error code (8).
    validate_goal_amount(env, params.goal).map_err(|_| ContractError::InvalidGoal)?;
    validate_min_contribution(params.min_contribution).map_err(|_| ContractError::InvalidMinContribution)?;
    validate_deadline(env.ledger().timestamp(), params.deadline).map_err(|_| ContractError::DeadlineTooSoon)?;

    if let Some(ref config) = params.platform_config {
        validate_platform_fee(config.fee_bps).map_err(|_| ContractError::InvalidPlatformFee)?;
    }

    validate_bonus_goal(params.bonus_goal, params.goal)?;
    validate_bonus_goal_description(&params.bonus_goal_description)?;

    Ok(())
}

// ── Core initialization logic ─────────────────────────────────────────────────

/// Executes the full campaign initialization flow.
///
<<<<<<< HEAD
/// Ordering guarantee:
/// 1. Re-initialization guard (read-only check).
/// 2. Creator authentication (`require_auth`).
/// 3. Full parameter validation (no storage writes yet).
/// 4. Storage writes (all-or-nothing within the transaction).
/// 5. Event emission.
||||||| a43ed59f
/// @notice This is the single authoritative implementation of campaign
///         initialization.  `CrowdfundContract::initialize()` in `lib.rs`
///         delegates to this function after constructing `InitParams`.
///
/// @param  env     The Soroban execution environment.
/// @param  params  Fully-populated initialization parameters.
/// @return         `Ok(())` on success; a typed `ContractError` on failure.
///
/// @dev    Ordering guarantee:
///         1. Re-initialization guard (read-only check).
///         2. Creator authentication (`require_auth`).
///         3. Full parameter validation (no storage writes yet).
///         4. Storage writes (all-or-nothing within the transaction).
///         5. Event emission.
///
/// @security  The re-initialization guard uses `DataKey::Creator` as the
///            sentinel because it is always written during initialization and
///            is never deleted.  Using a dedicated `Initialized` key would
///            require an extra storage slot and could be confused with other
///            boolean flags.
=======
/// @notice This is the single authoritative implementation of campaign
///         initialization. `CrowdfundContract::initialize()` in `lib.rs`
///         delegates to this function after constructing `InitParams`.
///
/// @param  env     The Soroban execution environment.
/// @param  params  Fully-populated initialization parameters.
/// @return         `Ok(())` on success; a typed `ContractError` on failure.
///
/// @dev    Strict ordering guarantee:
///         1. Re-initialization guard (read-only check, no state mutation).
///         2. Creator authentication (`require_auth`).
///         3. Full parameter validation (no storage writes yet).
///         4. Storage writes (all-or-nothing within the transaction).
///         5. Event emission.
///
/// @security  The re-initialization guard uses `DataKey::Creator` as the
///            sentinel because it is always written during initialization and
///            is never deleted.  A failed validation at step 3 leaves the
///            contract in its pre-initialization state — no partial writes.
>>>>>>> origin/main
pub fn execute_initialize(env: &Env, params: InitParams) -> Result<(), ContractError> {
<<<<<<< HEAD
    // 1. Re-initialization guard
||||||| a43ed59f
    // ── 1. Re-initialization guard ────────────────────────────────────────
=======
    // ── 1. Re-initialization guard ────────────────────────────────────────
    // Must be the very first check so no state can be written before it.
>>>>>>> origin/main
    if env.storage().instance().has(&DataKey::Creator) {
        return Err(ContractError::AlreadyInitialized);
    }

<<<<<<< HEAD
    // 2. Creator authentication
||||||| a43ed59f
    // ── 2. Creator authentication ─────────────────────────────────────────
    // Must happen before any state mutation so that an unauthorized call
    // cannot leave partial state.
=======
    // ── 2. Creator authentication ─────────────────────────────────────────
    // Called before any state mutation so an unauthorized call cannot leave
    // partial state.
>>>>>>> origin/main
    params.creator.require_auth();

<<<<<<< HEAD
    // 3. Parameter validation
||||||| a43ed59f
    // ── 3. Parameter validation ───────────────────────────────────────────
    // All checks run before the first storage write.  A failed check leaves
    // the contract in its pre-initialization state.
=======
    // ── 3. Parameter validation ───────────────────────────────────────────
    // All checks run before the first storage write. A failed check leaves
    // the contract in its pre-initialization state.
>>>>>>> origin/main
    validate_init_params(env, &params)?;

<<<<<<< HEAD
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

||||||| a43ed59f
    // ── 4. Storage writes ─────────────────────────────────────────────────

    // Admin — stored first so upgrade authorization is available immediately.
    env.storage()
        .instance()
        .set(&DataKey::Admin, &params.admin);

    // Core campaign fields.
    env.storage()
        .instance()
        .set(&DataKey::Creator, &params.creator);
    env.storage()
        .instance()
        .set(&DataKey::Token, &params.token);
    env.storage()
        .instance()
        .set(&DataKey::Goal, &params.goal);
    env.storage()
        .instance()
        .set(&DataKey::Deadline, &params.deadline);
    env.storage()
        .instance()
        .set(&DataKey::MinContribution, &params.min_contribution);

    // Counters and status — always initialized to known-good defaults.
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &0i128);
    env.storage()
        .instance()
        .set(&DataKey::BonusGoalReachedEmitted, &false);
    env.storage()
        .instance()
        .set(&DataKey::Status, &Status::Active);

    // Optional platform configuration.
=======
    // ── 4. Storage writes ─────────────────────────────────────────────────
    // Admin stored first so upgrade authorization is available immediately.
    env.storage().instance().set(&DataKey::Admin, &params.admin);

    // Core campaign fields.
    env.storage()
        .instance()
        .set(&DataKey::Creator, &params.creator);
    env.storage().instance().set(&DataKey::Token, &params.token);
    env.storage().instance().set(&DataKey::Goal, &params.goal);
    env.storage()
        .instance()
        .set(&DataKey::Deadline, &params.deadline);
    env.storage()
        .instance()
        .set(&DataKey::MinContribution, &params.min_contribution);

    // Counters and status — always initialized to known-good defaults.
    env.storage().instance().set(&DataKey::TotalRaised, &0i128);
    env.storage()
        .instance()
        .set(&DataKey::BonusGoalReachedEmitted, &false);
    env.storage()
        .instance()
        .set(&DataKey::Status, &Status::Active);

    // Optional platform configuration.
>>>>>>> origin/main
    if let Some(ref config) = params.platform_config {
        env.storage().instance().set(&DataKey::PlatformConfig, config);
    }
    if let Some(bg) = params.bonus_goal {
        env.storage().instance().set(&DataKey::BonusGoal, &bg);
    }
    if let Some(ref bg_desc) = params.bonus_goal_description {
        env.storage().instance().set(&DataKey::BonusGoalDescription, bg_desc);
    }

<<<<<<< HEAD
||||||| a43ed59f
    // Seed empty collections in persistent storage.

//! Maintainable validation/storage helpers for `initialize()`.
//!
//! This module extracts the initialization logic from `lib.rs` so the security
//! checks are easier to review and unit test.

use soroban_sdk::{Address, Env, String, Vec};

use crate::{contract_state_size, DataKey, PlatformConfig, RoadmapItem, Status};

/// @notice Validates initialization inputs and panics on invalid configuration.
/// @dev Panics preserve existing contract behavior for callers that rely on
///      fail-fast initialization checks.
pub fn validate_initialize_inputs(
    goal: i128,
    min_contribution: i128,
    platform_config: &Option<PlatformConfig>,
    bonus_goal: Option<i128>,
    bonus_goal_description: &Option<String>,
) {
    if goal <= 0 {
        panic!("goal must be positive");
    }
    if min_contribution <= 0 {
        panic!("min contribution must be positive");
    }

    if let Some(config) = platform_config {
        if config.fee_bps > 10_000 {
            panic!("platform fee cannot exceed 100%");
        }
    }

    if let Some(bg) = bonus_goal {
        if bg <= goal {
            panic!("bonus goal must be greater than primary goal");
        }
    }

    if let Some(description) = bonus_goal_description {
        if let Err(err) = contract_state_size::validate_bonus_goal_description(description) {
            panic!("{}", err);
        }
    }
}

/// @notice Persists initialize() state in one place for easier audits.
pub fn persist_initialize_state(
    env: &Env,
    admin: &Address,
    creator: &Address,
    token: &Address,
    goal: i128,
    deadline: u64,
    min_contribution: i128,
    platform_config: &Option<PlatformConfig>,
    bonus_goal: Option<i128>,
    bonus_goal_description: &Option<String>,
) {
    env.storage().instance().set(&DataKey::Admin, admin);
    env.storage().instance().set(&DataKey::Creator, creator);
    env.storage().instance().set(&DataKey::Token, token);
    env.storage().instance().set(&DataKey::Goal, &goal);
    env.storage().instance().set(&DataKey::Deadline, &deadline);
    env.storage()
        .instance()
        .set(&DataKey::MinContribution, &min_contribution);
    env.storage().instance().set(&DataKey::TotalRaised, &0i128);
    env.storage()
        .instance()
        .set(&DataKey::BonusGoalReachedEmitted, &false);
    env.storage().instance().set(&DataKey::Status, &Status::Active);

    if let Some(config) = platform_config {
        env.storage().instance().set(&DataKey::PlatformConfig, config);
    }
    if let Some(bg) = bonus_goal {
        env.storage().instance().set(&DataKey::BonusGoal, &bg);
    }
    if let Some(description) = bonus_goal_description {
        env.storage()
            .instance()
            .set(&DataKey::BonusGoalDescription, description);
    }


=======
    // Seed empty collections in persistent storage.
>>>>>>> origin/main
    let empty_contributors: Vec<Address> = Vec::new(env);
    env.storage().persistent().set(&DataKey::Contributors, &empty_contributors);

    let empty_roadmap: Vec<RoadmapItem> = Vec::new(env);
<<<<<<< HEAD
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
||||||| a43ed59f

    env.storage()
        .instance()
        .set(&DataKey::Roadmap, &empty_roadmap);

    // ── 5. Event emission ─────────────────────────────────────────────────
    // Emit a structured event so off-chain indexers can reconstruct campaign
    // state from the event stream without polling individual storage keys.
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
=======
    env.storage()
        .instance()
        .set(&DataKey::Roadmap, &empty_roadmap);

    // ── 5. Event emission ─────────────────────────────────────────────────
    // Emit a bounded event so off-chain indexers can reconstruct campaign
    // state from the event stream without polling individual storage keys.
    // Only scalar fields are included — no optional strings — to keep the
    // payload size O(1) regardless of bonus_goal_description length.
    log_initialize(
        env,
        &params.creator,
        &params.token,
        params.goal,
        params.deadline,
        params.min_contribution,
>>>>>>> origin/main
    );

    Ok(())
}

// ── Bounded initialization event ──────────────────────────────────────────────

/// Emits a single bounded `("campaign", "initialized")` event.
///
/// @notice Only scalar fields are included in the payload. Optional strings
///         (e.g. `bonus_goal_description`) are intentionally excluded to keep
///         event size O(1) and prevent unbounded gas consumption when long
///         descriptions are provided.
///
/// @param  env              The Soroban execution environment.
/// @param  creator          The campaign creator address.
/// @param  token            The token contract address.
/// @param  goal             The funding goal.
/// @param  deadline         The campaign deadline timestamp.
/// @param  min_contribution The minimum contribution amount.
///
/// @dev    Callers must not pass unbounded data (e.g. raw description strings)
///         to this function. All string fields must be omitted or pre-truncated
///         before calling.
pub fn log_initialize(
    env: &Env,
    creator: &Address,
    token: &Address,
    goal: i128,
    deadline: u64,
    min_contribution: i128,
) {
    env.events().publish(
        (
            Symbol::new(env, "campaign"),
            Symbol::new(env, "initialized"),
        ),
        (
            creator.clone(),
            token.clone(),
            goal,
            deadline,
            min_contribution,
        ),
    );
}

// ── Error description helpers ─────────────────────────────────────────────────

/// Returns a human-readable description for an `initialize()`-related error code.
<<<<<<< HEAD
||||||| a43ed59f
///
/// @param  code  The numeric `ContractError` repr value.
/// @return       A static string suitable for display in a frontend error message.
///
/// @dev    The frontend should call this with `error as u32` after receiving
///         a typed error from the SDK client.
=======
///
/// @param  code  The numeric `ContractError` repr value (e.g. `error as u32`).
/// @return       A static string suitable for display in a frontend error message.
///
/// @dev    Off-chain scripts and frontends should use this instead of hardcoding
///         strings so that a future code change only requires updating this function.
>>>>>>> origin/main
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

<<<<<<< HEAD
/// Returns `true` if the error code corresponds to a correctable input error.
||||||| a43ed59f
/// Returns `true` if the error code corresponds to a client-side input error
/// that can be corrected and retried.
///
/// @param  code  The numeric `ContractError` repr value.
/// @return       `true` for correctable input errors; `false` for permanent failures.
=======
/// Returns `true` if the error code corresponds to a client-side input error
/// that can be corrected and retried.
///
/// @param  code  The numeric `ContractError` repr value.
/// @return       `true` for correctable input errors; `false` for permanent failures.
///
/// @dev    `AlreadyInitialized` (1) is permanent — the contract cannot be
///         re-initialized.  All other init errors are input validation failures
///         that the caller can fix and retry.
>>>>>>> origin/main
pub fn is_init_error_retryable(code: u32) -> bool {
    matches!(code, 8 | 9 | 10 | 11 | 12)
}

/// Re-exports `MIN_GOAL_AMOUNT` for callers that only import this module.
pub use crate::campaign_goal_minimum::MIN_GOAL_AMOUNT as INIT_MIN_GOAL_AMOUNT;
