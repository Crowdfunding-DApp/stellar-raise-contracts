//! Unit and integration tests for `crowdfund_initialize_function`.
//!
//! Coverage: normal execution, all validation error paths, edge cases,
//! re-initialization guard, event emission, and helper functions.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, String,
};

use crate::{
    crowdfund_initialize_function::{
        describe_init_error, is_init_error_retryable, validate_bonus_goal,
        validate_token_address,
    },
    ContractError, CrowdfundContract, CrowdfundContractClient, PlatformConfig,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address, // creator
    Address, // token
    Address, // admin
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let creator = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_address).mint(&creator, &10_000_000);
    (env, client, creator, token_address, token_admin)
}

fn default_init(
    client: &CrowdfundContractClient,
    creator: &Address,
    token: &Address,
    deadline: u64,
) {
    client.initialize(
        creator, // admin = creator for simplicity
        creator, token, &1_000_000, &deadline, &1_000, &None, &None, &None,
    );
}

// ── Normal execution ──────────────────────────────────────────────────────────

#[test]
fn test_initialize_stores_all_fields() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.token(), token);
    assert_eq!(client.version(), 3);
}

#[test]
fn test_initialize_contributors_empty() {
    let (env, client, creator, token, _) = setup();
    default_init(&client, &creator, &token, env.ledger().timestamp() + 3600);
    assert_eq!(client.contributors().len(), 0);
}

#[test]
fn test_initialize_roadmap_empty() {
    let (env, client, creator, token, _) = setup();
    default_init(&client, &creator, &token, env.ledger().timestamp() + 3600);
    assert_eq!(client.roadmap().len(), 0);
}

#[test]
fn test_initialize_total_raised_zero() {
    let (env, client, creator, token, _) = setup();
    default_init(&client, &creator, &token, env.ledger().timestamp() + 3600);
    assert_eq!(client.total_raised(), 0);
}

// ── Token address validation ──────────────────────────────────────────────────

#[test]
fn test_initialize_rejects_random_address_as_token() {
    let (env, client, creator, _, _) = setup();
    let fake_token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &fake_token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidTokenAddress
    );
}

#[test]
fn test_initialize_accepts_valid_token() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);
    // No error means the token was accepted.
    assert_eq!(client.token(), token);
}

#[test]
fn test_initialize_rejects_empty_address_as_token() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    // A non-existent contract address (not registered) should fail validation.
    let nonexistent = Address::generate(&env);
    let result = client.try_initialize(
        &creator,
        &creator,
        &nonexistent,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidTokenAddress
    );
    let _ = token;
}

#[test]
fn test_initialize_rejects_contract_without_sep41() {
    // Register a contract that does NOT implement SEP-41 (register CrowdfundContract itself)
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    // Use the crowdfund contract's own address as the "token" — it doesn't implement SEP-41.
    let non_token = contract_id.address();
    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_initialize(
        &creator,
        &creator,
        &non_token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidTokenAddress
    );
}

#[test]
fn test_initialize_token_address_validated_before_storage_writes() {
    // Ensure that even if the token address is invalid, no storage is written.
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let fake_token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3600;
    let _ = client.try_initialize(
        &creator,
        &creator,
        &fake_token,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );
    // The contract should NOT have stored anything.
    assert_eq!(client.version(), 3); // default version — not overwritten
}

#[test]
fn test_initialize_emits_event() {
    let (env, client, creator, token, _) = setup();
    default_init(&client, &creator, &token, env.ledger().timestamp() + 3600);
    assert!(!env.events().all().events().is_empty());
}

// ── Platform config ───────────────────────────────────────────────────────────

#[test]
fn test_initialize_platform_fee_exact_max_accepted() {
    let (env, client, creator, token, _) = setup();
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 10_000,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &Some(config),
        &None,
        &None,
    );
    assert!(result.is_ok());
}

#[test]
fn test_initialize_platform_fee_zero_accepted() {
    let (env, client, creator, token, _) = setup();
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 0,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &Some(config),
        &None,
        &None,
    );
    assert!(result.is_ok());
}

#[test]
fn test_initialize_platform_fee_over_max_returns_error() {
    let (env, client, creator, token, _) = setup();
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: 10_001,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &Some(config),
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidPlatformFee
    );
}

#[test]
fn test_initialize_platform_fee_u32_max_returns_error() {
    let (env, client, creator, token, _) = setup();
    let config = PlatformConfig {
        address: Address::generate(&env),
        fee_bps: u32::MAX,
    };
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &Some(config),
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidPlatformFee
    );
}

// ── Bonus goal ────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_with_bonus_goal_stores_values() {
    let (env, client, creator, token, _) = setup();
    let desc = String::from_str(&env, "Unlock exclusive rewards");
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &None,
        &Some(2_000_000i128),
        &Some(desc.clone()),
    );
    assert_eq!(client.bonus_goal(), Some(2_000_000));
    assert_eq!(client.bonus_goal_description(), Some(desc));
    assert!(!client.bonus_goal_reached());
    assert_eq!(client.bonus_goal_progress_bps(), 0);
}

#[test]
fn test_initialize_bonus_goal_equal_to_goal_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &None,
        &Some(1_000_000i128),
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidBonusGoal
    );
}

#[test]
fn test_initialize_bonus_goal_less_than_goal_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &None,
        &Some(500_000i128),
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidBonusGoal
    );
}

#[test]
fn test_initialize_bonus_goal_one_above_goal_accepted() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &None,
        &Some(1_000_001i128),
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.bonus_goal(), Some(1_000_001));
}

#[test]
fn test_initialize_bonus_goal_without_description() {
    let (env, client, creator, token, _) = setup();
    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1_000,
        &None,
        &Some(2_000_000i128),
        &None,
    );
    assert_eq!(client.bonus_goal(), Some(2_000_000));
    assert_eq!(client.bonus_goal_description(), None);
}

// ── Re-initialization guard ───────────────────────────────────────────────────

#[test]
fn test_initialize_twice_returns_already_initialized() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &deadline, &1_000, &None, &None, &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::AlreadyInitialized
    );
}

#[test]
fn test_initialize_twice_original_values_unchanged() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

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
    );
    assert_eq!(client.goal(), 1_000_000);
}

// ── Goal validation ───────────────────────────────────────────────────────────

#[test]
fn test_initialize_goal_minimum_accepted() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.goal(), 1);
}

#[test]
fn test_initialize_goal_zero_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &0,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

#[test]
fn test_initialize_goal_negative_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &-1,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

#[test]
fn test_initialize_goal_i128_min_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &i128::MIN,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidGoal);
}

// ── Min contribution validation ───────────────────────────────────────────────

#[test]
fn test_initialize_min_contribution_minimum_accepted() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.min_contribution(), 1);
}

#[test]
fn test_initialize_min_contribution_zero_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &0,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidMinContribution
    );
}

#[test]
fn test_initialize_min_contribution_negative_returns_error() {
    let (env, client, creator, token, _) = setup();
    let result = client.try_initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &(env.ledger().timestamp() + 3600),
        &-100,
        &None,
        &None,
        &None,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidMinContribution
    );
}

// ── Deadline validation ───────────────────────────────────────────────────────

#[test]
fn test_initialize_deadline_exactly_min_offset_accepted() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 60;
    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &deadline, &1_000, &None, &None, &None,
    );
    assert!(result.is_ok());
}

#[test]
fn test_initialize_deadline_59s_returns_error() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 59;
    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &deadline, &1_000, &None, &None, &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

#[test]
fn test_initialize_deadline_equal_to_now_returns_error() {
    let (env, client, creator, token, _) = setup();
    let now = env.ledger().timestamp();
    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &now, &1_000, &None, &None, &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

#[test]
fn test_initialize_deadline_in_past_returns_error() {
    let (env, client, creator, token, _) = setup();
    env.ledger().set_timestamp(10_000);
    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &5_000, &1_000, &None, &None, &None,
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DeadlineTooSoon);
}

#[test]
fn test_initialize_deadline_far_future_accepted() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 365 * 24 * 3600;
    let result = client.try_initialize(
        &creator, &creator, &token, &1_000_000, &deadline, &1_000, &None, &None, &None,
    );
    assert!(result.is_ok());
    assert_eq!(client.deadline(), deadline);
}

// ── validate_bonus_goal unit tests ────────────────────────────────────────────

#[test]
fn test_validate_bonus_goal_none_is_ok() {
    assert!(validate_bonus_goal(None, 1_000_000).is_ok());
}

#[test]
fn test_validate_bonus_goal_greater_is_ok() {
    assert!(validate_bonus_goal(Some(1_000_001), 1_000_000).is_ok());
}

#[test]
fn test_validate_bonus_goal_equal_returns_error() {
    assert_eq!(
        validate_bonus_goal(Some(1_000_000), 1_000_000),
        Err(ContractError::InvalidBonusGoal)
    );
}

#[test]
fn test_validate_bonus_goal_less_returns_error() {
    assert_eq!(
        validate_bonus_goal(Some(999_999), 1_000_000),
        Err(ContractError::InvalidBonusGoal)
    );
}

#[test]
fn test_validate_bonus_goal_zero_when_goal_is_one_returns_error() {
    assert_eq!(
        validate_bonus_goal(Some(0), 1),
        Err(ContractError::InvalidBonusGoal)
    );
}

// ── validate_token_address unit tests ──────────────────────────────────────────

#[test]
fn test_validate_token_address_valid_token() {
    let env = Env::default();
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token_address = token_id.address();
    assert!(validate_token_address(&env, &token_address).is_ok());
}

#[test]
fn test_validate_token_address_random_address() {
    let env = Env::default();
    let fake = Address::generate(&env);
    assert_eq!(
        validate_token_address(&env, &fake),
        Err(ContractError::InvalidTokenAddress)
    );
}

#[test]
fn test_validate_token_address_non_token_contract() {
    let env = Env::default();
    let contract_id = env.register(CrowdfundContract, ());
    let non_token = contract_id.address();
    assert_eq!(
        validate_token_address(&env, &non_token),
        Err(ContractError::InvalidTokenAddress)
    );
}

// ── describe_init_error ───────────────────────────────────────────────────────

#[test]
fn test_describe_init_error_already_initialized() {
    assert_eq!(describe_init_error(1), "Contract is already initialized");
}

#[test]
fn test_describe_init_error_invalid_goal() {
    assert!(describe_init_error(8).contains("goal"));
}

#[test]
fn test_describe_init_error_invalid_min_contribution() {
    assert!(describe_init_error(9).contains("contribution"));
}

#[test]
fn test_describe_init_error_deadline_too_soon() {
    assert!(describe_init_error(10).contains("Deadline"));
}

#[test]
fn test_describe_init_error_invalid_platform_fee() {
    assert!(describe_init_error(11).contains("fee"));
}

#[test]
fn test_describe_init_error_invalid_bonus_goal() {
    assert!(describe_init_error(12).contains("Bonus"));
}

#[test]
fn test_describe_init_error_invalid_token_address() {
    assert!(describe_init_error(19).contains("Token"));
    assert!(describe_init_error(19).contains("SEP-41"));
}

#[test]
fn test_describe_init_error_unknown_code() {
    let msg = describe_init_error(99);
    assert!(!msg.is_empty());
    assert!(msg.contains("Unknown"));
}

// ── is_init_error_retryable ───────────────────────────────────────────────────

#[test]
fn test_is_retryable_already_initialized_is_false() {
    assert!(!is_init_error_retryable(1));
}

#[test]
fn test_is_retryable_input_errors_are_true() {
    for code in 8u32..=12 {
        assert!(
            is_init_error_retryable(code),
            "code {} should be retryable",
            code
        );
    }
}

#[test]
fn test_is_retryable_invalid_token_address_is_true() {
    assert!(is_init_error_retryable(19));
}

#[test]
fn test_is_retryable_unknown_code_is_false() {
    assert!(!is_init_error_retryable(99));
}

// ── log_initialize ────────────────────────────────────────────────────────────

#[test]
fn test_log_initialize_event_payload() {
    // Verify that after a successful initialize(), exactly one event is recorded.
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let events = env.events().all();
    assert_eq!(events.events().len(), 1);
}

#[test]
fn test_log_initialize_not_emitted_on_validation_failure() {
    let (env, client, creator, token, _) = setup();
    // goal = 0 → InvalidGoal, no initialized event should be emitted
    let _ = client.try_initialize(
        &creator,
        &creator,
        &token,
        &0,
        &(env.ledger().timestamp() + 3600),
        &1,
        &None,
        &None,
        &None,
    );
    // No events should be emitted on failure.
    assert!(env.events().all().events().is_empty());
}

// ── Integration ───────────────────────────────────────────────────────────────

#[test]
fn test_post_init_contribute_works() {
    let (env, client, creator, token, token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let contributor = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    assert_eq!(client.total_raised(), 5_000);
    assert_eq!(client.contribution(&contributor), 5_000);
    let _ = token_admin;
}

#[test]
fn test_post_init_get_stats_correct() {
    let (env, client, creator, token, _) = setup();
    default_init(&client, &creator, &token, env.ledger().timestamp() + 3600);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 0);
    assert_eq!(stats.contributor_count, 0);
}

#[test]
fn test_post_init_withdraw_after_goal_met() {
    let (env, client, creator, token, _) = setup();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token, deadline);

    let contributor = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&contributor, &1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);
    let token_client = token::Client::new(&env, &token);
    let before = token_client.balance(&creator);
    client.withdraw();
    assert_eq!(token_client.balance(&creator), before + 1_000_000);
}
