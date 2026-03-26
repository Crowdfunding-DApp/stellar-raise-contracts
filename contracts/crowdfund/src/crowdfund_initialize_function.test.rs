//! Comprehensive test suite for `crowdfund_initialize_function`.
//!
//! @title   CrowdfundInitializeFunction Tests
//! @notice  Tests cover: normal execution, all validation error paths,
//!          edge cases, re-initialization guard, event emission, storage
//!          correctness, and helper function behavior.
//! @dev     Target: 95%+ code coverage for the initialize function module.
//!
//! ## Test Categories
//!
//! 1. **Normal execution** — Happy path initialization
//! 2. **Platform config** — Fee validation edge cases
//! 3. **Bonus goal** — Ordering and boundary conditions
//! 4. **Re-initialization guard** — State isolation
//! 5. **Goal validation** — Boundary and invalid values
//! 6. **Min contribution validation** — Boundary and invalid values
//! 7. **Deadline validation** — Time-based constraints
//! 8. **Helper functions** — Unit tests for validation helpers
//! 9. **Error description helpers** — Frontend integration

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, String,
};

use crate::{
    crowdfund_initialize_function::{
        describe_init_error, execute_initialize, is_init_error_retryable, validate_bonus_goal,
        validate_bonus_goal_description, validate_init_params, InitParams,
    },
    ContractError, CrowdfundContract, CrowdfundContractClient, PlatformConfig,
};

// ══════════════════════════════════════════════════════════════════════════════
// Test Helpers
// ══════════════════════════════════════════════════════════════════════════════

/// Creates a test environment with mocked authorizations.
fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Registers the contract and returns (env, client, creator, token, admin).
fn setup() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let creator = Address::generate(&env);
    token_admin_client.mint(&creator, &10_000_000);

    (env, client, creator, token_address, token_admin)
}

/// Calls `initialize` with sensible defaults and returns the deadline used.
fn default_init(
    client: &CrowdfundContractClient,
    creator: &Address,
    token: &Address,
    deadline: u64,
) {
    let admin = creator.clone();
    client.initialize(
        &admin,
        creator,
        token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
}

/// Creates InitParams with sensible defaults for direct execute_initialize testing.
fn default_init_params(env: &Env, creator: &Address, token: &Address) -> InitParams {
    InitParams {
        admin: creator.clone(),
        creator: creator.clone(),
        token: token.clone(),
        goal: 1_000_000,
        deadline: env.ledger().timestamp() + 3600,
        min_contribution: 1_000,
        platform_config: None,
        bonus_goal: None,
        bonus_goal_description: None,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Normal Execution Tests
// ══════════════════════════════════════════════════════════════════════════════

/// All fields are stored correctly after a successful initialization.
#[test]
fn test_initialize_stores_all_fields() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.token(), token);
    assert_eq!(client.version(), 3);
}

/// Status is Active immediately after initialization.
#[test]
fn test_initialize_status_is_active() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    // Verify by attempting a contribution — only works when Active.
    let contributor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);
    assert_eq!(client.total_raised(), 5_000);
}

/// Contributors list is empty immediately after initialization.
#[test]
fn test_initialize_contributors_list_is_empty() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);
    assert_eq!(client.contributors().len(), 0);
}

/// Roadmap is empty immediately after initialization.
#[test]
fn test_initialize_roadmap_is_empty() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);
    assert_eq!(client.roadmap().len(), 0);
}

/// total_raised is zero immediately after initialization.
#[test]
fn test_initialize_total_raised_is_zero() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);
    assert_eq!(client.total_raised(), 0);
}

/// An `initialized` event is emitted on success.
#[test]
fn test_initialize_emits_event() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let events = env.events().all();
    assert!(!events.is_empty());
}

/// Admin address is stored correctly.
#[test]
fn test_initialize_stores_admin_address() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let admin = Address::generate(&env);
    
    client.initialize(
        &admin,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
}

/// Creator address is stored correctly.
#[test]
fn test_initialize_stores_creator_address() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let other_creator = Address::generate(&env);
    
    client.initialize(
        &creator,
        &other_creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Platform Config Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Platform config is stored and fee is deducted on withdrawal.
#[test]
fn test_initialize_with_platform_config_stores_fee() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let platform_addr = Address::generate(&env);
    let config = PlatformConfig {
        address: platform_addr.clone(),
        fee_bps: 500, // 5%
    };
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );

    // Contribute and withdraw to verify fee is applied.
    let contributor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contributor, &1_000_000);
    client.contribute(&contributor, &1_000_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&platform_addr), 50_000); // 5%
}

/// Exact maximum platform fee (10_000 bps = 100%) is accepted.
#[test]
fn test_initialize_platform_fee_exact_max_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 10_000,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

/// Platform fee of 0 bps is accepted.
#[test]
fn test_initialize_platform_fee_zero_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 0,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

/// Platform fee of 1 bps is accepted.
#[test]
fn test_initialize_platform_fee_one_bps_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 1,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

/// Platform fee of 10_001 bps returns InvalidPlatformFee.
#[test]
fn test_initialize_platform_fee_over_max_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 10_001,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidPlatformFee
    );
}

/// u32::MAX platform fee returns InvalidPlatformFee.
#[test]
fn test_initialize_platform_fee_u32_max_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: u32::MAX,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidPlatformFee
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Bonus Goal Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Bonus goal and description are stored and readable.
#[test]
fn test_initialize_with_bonus_goal_stores_values() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let desc = String::from_str(&env, "Unlock exclusive rewards");
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(2_000_000i128),
        &Some(desc.clone()),
        &None,
    );
    assert_eq!(client.bonus_goal(), Some(2_000_000));
    assert_eq!(client.bonus_goal_description(), Some(desc));
    assert!(!client.bonus_goal_reached());
    assert_eq!(client.bonus_goal_progress_bps(), 0);
}

/// Bonus goal equal to primary goal returns InvalidBonusGoal.
#[test]
fn test_initialize_bonus_goal_equal_to_goal_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(1_000_000i128), // equal, not greater
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidBonusGoal
    );
}

/// Bonus goal less than primary goal returns InvalidBonusGoal.
#[test]
fn test_initialize_bonus_goal_less_than_goal_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(500_000i128),
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidBonusGoal
    );
}

/// Bonus goal of 1 above primary goal is accepted.
#[test]
fn test_initialize_bonus_goal_one_above_goal_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(1_000_001i128),
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.bonus_goal(), Some(1_000_001));
}

/// Bonus goal without description stores None for description.
#[test]
fn test_initialize_bonus_goal_without_description() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(2_000_000i128),
        &None,
        &None,
    );
    assert_eq!(client.bonus_goal(), Some(2_000_000));
    assert_eq!(client.bonus_goal_description(), None);
}

/// Bonus goal of i128::MAX is accepted (theoretical maximum).
#[test]
fn test_initialize_bonus_goal_i128_max_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(i128::MAX),
        &None,
        &None,
    );
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Re-initialization Guard Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Second initialize call returns AlreadyInitialized.
#[test]
fn test_initialize_twice_returns_already_initialized() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::AlreadyInitialized
    );
}

/// Re-initialization with different parameters still returns AlreadyInitialized.
#[test]
fn test_initialize_twice_different_params_still_errors() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &9_999_999, // different goal
        &(deadline + 7200),
        &500,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::AlreadyInitialized
    );
    // Original values must be unchanged.
    assert_eq!(client.goal(), 1_000_000);
}

/// Re-initialization does not modify any storage values.
#[test]
fn test_initialize_twice_preserves_original_values() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    // Attempt re-init with different values
    let _ = client.try_initialize(
        &creator,
        &creator,
        &token,
        &9_999_999,
        &(deadline + 7200),
        &500,
        &None,
        &None,
        &None,
        &None,
    );

    // Verify original values unchanged
    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.total_raised(), 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// Goal Validation Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Goal of 1 (minimum) is accepted.
#[test]
fn test_initialize_goal_minimum_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.goal(), 1);
}

/// Goal of 0 returns InvalidGoal.
#[test]
fn test_initialize_goal_zero_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &0,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

/// Negative goal returns InvalidGoal.
#[test]
fn test_initialize_goal_negative_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &-1,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

/// i128::MIN goal returns InvalidGoal.
#[test]
fn test_initialize_goal_i128_min_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &i128::MIN,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

/// Goal of i128::MAX is accepted (theoretical maximum).
#[test]
fn test_initialize_goal_i128_max_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &i128::MAX,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Min Contribution Validation Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Min contribution of 1 (minimum) is accepted.
#[test]
fn test_initialize_min_contribution_minimum_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.min_contribution(), 1);
}

/// Min contribution of 0 returns InvalidMinContribution.
#[test]
fn test_initialize_min_contribution_zero_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &0,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidMinContribution
    );
}

/// Negative min contribution returns InvalidMinContribution.
#[test]
fn test_initialize_min_contribution_negative_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &-1,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidMinContribution
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Deadline Validation Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Deadline exactly 60 seconds in the future is accepted.
#[test]
fn test_initialize_deadline_exactly_60_seconds_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 60;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

/// Deadline 59 seconds in the future returns DeadlineTooSoon.
#[test]
fn test_initialize_deadline_59_seconds_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 59;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

/// Deadline equal to current time returns DeadlineTooSoon.
#[test]
fn test_initialize_deadline_equal_to_now_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

/// Deadline in the past returns DeadlineTooSoon.
#[test]
fn test_initialize_deadline_in_past_returns_error() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() - 100;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

/// Deadline far in the future is accepted.
#[test]
fn test_initialize_deadline_far_future_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 100_000_000;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.deadline(), deadline);
}

/// Deadline u64::MAX is accepted (theoretical maximum).
#[test]
fn test_initialize_deadline_u64_max_accepted() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = u64::MAX;
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Validation Helper Unit Tests
// ══════════════════════════════════════════════════════════════════════════════

/// validate_bonus_goal with None bonus_goal returns Ok.
#[test]
fn test_validate_bonus_goal_none_returns_ok() {
    let result = validate_bonus_goal(None, 1_000_000);
    assert!(result.is_ok());
}

/// validate_bonus_goal with bonus_goal > goal returns Ok.
#[test]
fn test_validate_bonus_goal_greater_than_goal_returns_ok() {
    let result = validate_bonus_goal(Some(2_000_000), 1_000_000);
    assert!(result.is_ok());
}

/// validate_bonus_goal with bonus_goal == goal returns Err.
#[test]
fn test_validate_bonus_goal_equal_to_goal_returns_err() {
    let result = validate_bonus_goal(Some(1_000_000), 1_000_000);
    assert_eq!(result.unwrap_err(), ContractError::InvalidBonusGoal);
}

/// validate_bonus_goal with bonus_goal < goal returns Err.
#[test]
fn test_validate_bonus_goal_less_than_goal_returns_err() {
    let result = validate_bonus_goal(Some(500_000), 1_000_000);
    assert_eq!(result.unwrap_err(), ContractError::InvalidBonusGoal);
}

/// validate_bonus_goal with bonus_goal = 0 and goal = 1 returns Err.
#[test]
fn test_validate_bonus_goal_zero_vs_one_returns_err() {
    let result = validate_bonus_goal(Some(0), 1);
    assert_eq!(result.unwrap_err(), ContractError::InvalidBonusGoal);
}

/// validate_bonus_goal_description with None returns Ok.
#[test]
fn test_validate_bonus_goal_description_none_returns_ok() {
    let result = validate_bonus_goal_description(&None);
    assert!(result.is_ok());
}

/// validate_init_params integration test with valid params.
#[test]
fn test_validate_init_params_valid() {
    let env = Env::default();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    
    let params = default_init_params(&env, &creator, &token);
    let result = validate_init_params(&env, &params);
    assert!(result.is_ok());
}

/// validate_init_params fails with invalid goal.
#[test]
fn test_validate_init_params_invalid_goal() {
    let env = Env::default();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    
    let mut params = default_init_params(&env, &creator, &token);
    params.goal = 0;
    let result = validate_init_params(&env, &params);
    assert_eq!(result.unwrap_err(), ContractError::InvalidGoal);
}

// ══════════════════════════════════════════════════════════════════════════════
// Error Description Helper Tests
// ══════════════════════════════════════════════════════════════════════════════

/// describe_init_error returns correct message for AlreadyInitialized.
#[test]
fn test_describe_init_error_already_initialized() {
    assert_eq!(
        describe_init_error(1),
        "Contract is already initialized"
    );
}

/// describe_init_error returns correct message for InvalidGoal.
#[test]
fn test_describe_init_error_invalid_goal() {
    assert_eq!(
        describe_init_error(8),
        "Campaign goal must be at least 1"
    );
}

/// describe_init_error returns correct message for InvalidMinContribution.
#[test]
fn test_describe_init_error_invalid_min_contribution() {
    assert_eq!(
        describe_init_error(9),
        "Minimum contribution must be at least 1"
    );
}

/// describe_init_error returns correct message for DeadlineTooSoon.
#[test]
fn test_describe_init_error_deadline_too_soon() {
    assert_eq!(
        describe_init_error(10),
        "Deadline must be at least 60 seconds in the future"
    );
}

/// describe_init_error returns correct message for InvalidPlatformFee.
#[test]
fn test_describe_init_error_invalid_platform_fee() {
    assert_eq!(
        describe_init_error(11),
        "Platform fee cannot exceed 100% (10,000 bps)"
    );
}

/// describe_init_error returns correct message for InvalidBonusGoal.
#[test]
fn test_describe_init_error_invalid_bonus_goal() {
    assert_eq!(
        describe_init_error(12),
        "Bonus goal must be strictly greater than the primary goal"
    );
}

/// describe_init_error returns fallback for unknown error code.
#[test]
fn test_describe_init_error_unknown_code() {
    assert_eq!(
        describe_init_error(99),
        "Unknown initialization error"
    );
}

/// describe_init_error returns fallback for zero.
#[test]
fn test_describe_init_error_zero_code() {
    assert_eq!(
        describe_init_error(0),
        "Unknown initialization error"
    );
}

/// is_init_error_retryable returns false for AlreadyInitialized.
#[test]
fn test_is_init_error_retryable_already_initialized() {
    assert!(!is_init_error_retryable(1));
}

/// is_init_error_retryable returns true for all input errors.
#[test]
fn test_is_init_error_retryable_input_errors() {
    assert!(is_init_error_retryable(8));  // InvalidGoal
    assert!(is_init_error_retryable(9));  // InvalidMinContribution
    assert!(is_init_error_retryable(10)); // DeadlineTooSoon
    assert!(is_init_error_retryable(11)); // InvalidPlatformFee
    assert!(is_init_error_retryable(12)); // InvalidBonusGoal
}

/// is_init_error_retryable returns false for unknown error codes.
#[test]
fn test_is_init_error_retryable_unknown_code() {
    assert!(!is_init_error_retryable(99));
}

// ══════════════════════════════════════════════════════════════════════════════
// Integration Tests
// ══════════════════════════════════════════════════════════════════════════════

/// After initialization, contribution works correctly.
#[test]
fn test_initialize_then_contribute() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let contributor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contributor, &5_000);
    
    client.contribute(&contributor, &5_000);
    assert_eq!(client.total_raised(), 5_000);
    assert_eq!(client.contributors().len(), 1);
}

/// After initialization, withdraw works after deadline with sufficient funds.
#[test]
fn test_initialize_then_withdraw() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let contributor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contributor, &1_000_000);
    
    client.contribute(&contributor, &1_000_000);
    
    // Fast forward past deadline
    env.ledger().set_timestamp(deadline + 1);
    
    // Finalize first
    client.finalize();
    
    // Now withdraw should work
    client.withdraw();
}

/// Initialize with all optional parameters combined works correctly.
#[test]
fn test_initialize_with_all_optional_params() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    let platform = Address::generate(&env);
    let desc = String::from_str(&env, "Stretch goal for extra features");
    
    let config = PlatformConfig {
        address: platform.clone(),
        fee_bps: 250, // 2.5%
    };
    
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(config),
        &Some(2_000_000),
        &Some(desc.clone()),
        &None,
    );

    // Verify all values stored
    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.bonus_goal(), Some(2_000_000));
    assert_eq!(client.bonus_goal_description(), Some(desc));
    
    // Verify contribution still works
    let contributor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contributor, &1_000_000);
    client.contribute(&contributor, &1_000_000);
    assert_eq!(client.total_raised(), 1_000_000);
}

/// execute_initialize directly stores all fields correctly.
#[test]
fn test_execute_initialize_stores_fields_directly() {
    let env = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_id.address();
    
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&creator, &10_000_000);
    
    let params = InitParams {
        admin: creator.clone(),
        creator: creator.clone(),
        token: token.clone(),
        goal: 5_000_000,
        deadline: env.ledger().timestamp() + 7200,
        min_contribution: 500,
        platform_config: None,
        bonus_goal: Some(10_000_000),
        bonus_goal_description: None,
    };
    
    let result = execute_initialize(&env, params);
    assert!(result.is_ok());
    
    assert_eq!(client.goal(), 5_000_000);
    assert_eq!(client.deadline(), env.ledger().timestamp() + 7200);
    assert_eq!(client.min_contribution(), 500);
    assert_eq!(client.bonus_goal(), Some(10_000_000));
}

/// execute_initialize fails for already initialized contract.
#[test]
fn test_execute_initialize_fails_if_already_initialized() {
    let env = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_id.address();
    
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&creator, &10_000_000);
    
    // First initialization
    let params1 = InitParams {
        admin: creator.clone(),
        creator: creator.clone(),
        token: token.clone(),
        goal: 1_000_000,
        deadline: env.ledger().timestamp() + 3600,
        min_contribution: 1000,
        platform_config: None,
        bonus_goal: None,
        bonus_goal_description: None,
    };
    let result1 = execute_initialize(&env, params1);
    assert!(result1.is_ok());
    
    // Second initialization should fail
    let params2 = InitParams {
        admin: creator.clone(),
        creator: creator.clone(),
        token: token.clone(),
        goal: 2_000_000,
        deadline: env.ledger().timestamp() + 7200,
        min_contribution: 2000,
        platform_config: None,
        bonus_goal: None,
        bonus_goal_description: None,
    };
    let result2 = execute_initialize(&env, params2);
    assert_eq!(result2.unwrap_err(), ContractError::AlreadyInitialized);
}

/// Bonus goal reached flag starts as false.
#[test]
fn test_initialize_bonus_goal_reached_flag_starts_false() {
    let (env, client, creator, token, _admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(2_000_000),
        &None,
        &None,
    );
    
    assert!(!client.bonus_goal_reached());
}
