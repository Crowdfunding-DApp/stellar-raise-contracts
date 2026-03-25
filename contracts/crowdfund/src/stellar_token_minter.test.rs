//! Security and readability-focused tests for Stellar token minter flows.

use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, BytesN, Env, IntoVal,
};

use crate::stellar_token_minter::{
    deadline_offset_seconds, TEST_GOAL, TEST_MIN_CONTRIBUTION, TEST_MINT_AMOUNT,
};
use crate::{ContractError, CrowdfundContract, CrowdfundContractClient};

/// @notice Creates a clean test environment with a funded creator.
/// @dev Uses `mock_all_auths` so tests can focus on business logic branches.
fn setup_env() -> (Env, CrowdfundContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let creator = Address::generate(&env);
    token_admin_client.mint(&creator, &TEST_MINT_AMOUNT);

    (env, client, creator, token_address, token_admin, contract_id)
}

/// @notice Initializes a campaign with defaults suitable for minter tests.
fn init_campaign(
    client: &CrowdfundContractClient,
    creator: &Address,
    token_address: &Address,
    deadline: u64,
) {
    client.initialize(
        creator,
        creator,
        token_address,
        &TEST_GOAL,
        &deadline,
        &TEST_MIN_CONTRIBUTION,
        &None,
        &None,
        &None,
    );
}

/// @notice Mints test tokens to an account.
fn mint_to(env: &Env, token_address: &Address, to: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token_address).mint(to, &amount);
}

/// @notice Enforces deadline guard for pledge collection.
/// @security Collecting before deadline must fail to prevent early token pulls.
#[test]
fn collect_pledges_rejects_before_deadline() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    init_campaign(&client, &creator, &token_address, deadline);

    let pledger = Address::generate(&env);
    mint_to(&env, &token_address, &pledger, 600_000);
    client.pledge(&pledger, &600_000);

    let result = client.try_collect_pledges();
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::CampaignStillActive
    );
}

/// @notice Enforces goal guard for pledge collection.
/// @security Campaign cannot collect pledges if total funds are below goal.
#[test]
fn collect_pledges_rejects_when_goal_not_reached() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    init_campaign(&client, &creator, &token_address, deadline);

    let pledger = Address::generate(&env);
    mint_to(&env, &token_address, &pledger, 500_000);
    client.pledge(&pledger, &500_000);

    env.ledger().set_timestamp(deadline + 1);
    let result = client.try_collect_pledges();
    assert_eq!(result.unwrap_err().unwrap(), ContractError::GoalNotReached);
}

/// @notice Validates successful pledge collection once deadline and goal are met.
/// @security Ensures pledges are materialized into raised balance exactly once.
#[test]
fn collect_pledges_succeeds_after_deadline_when_goal_met() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    init_campaign(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &contributor, TEST_GOAL);
    client.contribute(&contributor, &(TEST_GOAL / 2));

    let pledger = Address::generate(&env);
    mint_to(&env, &token_address, &pledger, TEST_GOAL);
    client.pledge(&pledger, &(TEST_GOAL / 2));

    env.ledger().set_timestamp(deadline + 1);
    client.collect_pledges();

    assert_eq!(client.total_raised(), TEST_GOAL);
}

/// @notice Asserts upgrade access control is admin-only.
/// @security Unauthorized users must never be able to replace contract WASM.
#[test]
#[should_panic]
fn upgrade_requires_admin_auth() {
    let (env, client, creator, token_address, _token_admin, contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    init_campaign(&client, &creator, &token_address, deadline);

    let non_admin = Address::generate(&env);
    env.set_auths(&[]);
    client.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "upgrade",
            args: soroban_sdk::vec![
                &env,
                BytesN::from_array(&env, &[0u8; 32]).into_val(&env)
            ],
            sub_invokes: &[],
        },
    }]);

    client.upgrade(&BytesN::from_array(&env, &[0u8; 32]));
}

/// @notice Verifies bonus-goal progress is capped at 100%.
/// @security Prevents misleading UI values and overflow-like progress reporting.
#[test]
fn bonus_goal_progress_is_capped() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    client.initialize(
        &creator,
        &creator,
        &token_address,
        &TEST_GOAL,
        &deadline,
        &TEST_MIN_CONTRIBUTION,
        &None,
        &Some(TEST_GOAL * 2),
        &None,
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &contributor, TEST_MINT_AMOUNT);
    client.contribute(&contributor, &(TEST_GOAL * 3));

    assert!(client.bonus_goal_reached());
    assert_eq!(client.bonus_goal_progress_bps(), 10_000);
}

/// @notice Verifies aggregate stats for an untouched campaign.
#[test]
fn get_stats_is_zeroed_for_new_campaign() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env();
    let deadline = env.ledger().timestamp() + deadline_offset_seconds();
    init_campaign(&client, &creator, &token_address, deadline);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.contributor_count, 0);
    assert_eq!(stats.average_contribution, 0);
    assert_eq!(stats.largest_contribution, 0);
}

