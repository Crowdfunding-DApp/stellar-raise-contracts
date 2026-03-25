//! Comprehensive tests for the single-contributor refund flow.
//!
//! This suite validates both the internal helpers in `refund_single_token.rs`
//! and the public `refund_single()` contract entrypoint.
//!
//! Security-focused coverage includes:
//! - contributor-only authorization
//! - deadline and goal gating
//! - CEI-style state updates
//! - underflow resistance without partial storage mutation
//! - contributor isolation
//! - event emission for off-chain indexing

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal, String, TryFromVal,
};

use crate::{
    refund_single_token::{execute_refund_single, validate_refund_preconditions},
    ContractError, CrowdfundContract, CrowdfundContractClient, PlatformConfig,
};

/// @notice Create a fresh environment with auth mocking enabled for most tests.
fn setup() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
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

/// @notice Mint test tokens to a contributor.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token).mint(to, &amount);
}

/// @notice Initialize the crowdfund with the minimum fields needed for refund tests.
fn init(
    client: &CrowdfundContractClient,
    creator: &Address,
    token: &Address,
    goal: i128,
    deadline: u64,
) {
    client.initialize(
        creator, creator, token, &goal, &deadline, &1_000, &None, &None, &None,
    );
}

/// @notice Count events whose topic pair is `("campaign", "refund_single")`.
fn count_refund_events(env: &Env) -> usize {
    let campaign = String::from_str(env, "campaign");
    let refund_single = String::from_str(env, "refund_single");

    env.events()
        .all()
        .iter()
        .filter(|event| {
            let (_, topics, _) = event;
            if topics.len() < 2 {
                return false;
            }

            let first = topics.get(0).unwrap();
            let second = topics.get(1).unwrap();

            String::try_from_val(env, &first)
                .map(|value| value == campaign)
                .unwrap_or(false)
                && String::try_from_val(env, &second)
                    .map(|value| value == refund_single)
                    .unwrap_or(false)
        })
        .count()
}

// ── validate_refund_preconditions ──────────────────────────────────────────

/// @notice Returns the stored contribution amount when every guard passes.
#[test]
fn test_validate_returns_amount_on_success() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 50_000);
    client.contribute(&alice, &50_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &alice)
    });

    assert_eq!(result, Ok(50_000));
}

/// @notice Rejects refund attempts before the deadline has passed.
#[test]
fn test_validate_before_deadline_returns_campaign_still_active() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 50_000);
    client.contribute(&alice, &50_000);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &alice)
    });

    assert_eq!(result, Err(ContractError::CampaignStillActive));
}

/// @notice Treats the exact deadline as still active to avoid off-by-one claims.
#[test]
fn test_validate_at_deadline_boundary_returns_campaign_still_active() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 50_000);
    client.contribute(&alice, &50_000);

    env.ledger().set_timestamp(deadline);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &alice)
    });

    assert_eq!(result, Err(ContractError::CampaignStillActive));
}

/// @notice Blocks refunds when the goal has been reached exactly.
#[test]
fn test_validate_goal_met_returns_goal_reached() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal: i128 = 100_000;
    init(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, goal);
    client.contribute(&alice, &goal);

    env.ledger().set_timestamp(deadline + 1);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &alice)
    });

    assert_eq!(result, Err(ContractError::GoalReached));
}

/// @notice Blocks refunds when contributions exceed the funding goal.
#[test]
fn test_validate_goal_exceeded_returns_goal_reached() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal: i128 = 100_000;
    init(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, goal + 50_000);
    client.contribute(&alice, &(goal + 50_000));

    env.ledger().set_timestamp(deadline + 1);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &alice)
    });

    assert_eq!(result, Err(ContractError::GoalReached));
}

/// @notice Returns `NothingToRefund` when the contributor has no recorded balance.
#[test]
fn test_validate_without_contribution_returns_nothing_to_refund() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let stranger = Address::generate(&env);
    env.ledger().set_timestamp(deadline + 1);

    let result = env.as_contract(&client.address, || {
        validate_refund_preconditions(&env, &stranger)
    });

    assert_eq!(result, Err(ContractError::NothingToRefund));
}

/// @notice Panics when refunds are attempted after a successful campaign.
#[test]
#[should_panic(expected = "campaign is not active")]
fn test_validate_panics_on_successful_campaign() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal: i128 = 100_000;
    init(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, goal);
    client.contribute(&alice, &goal);

    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    env.as_contract(&client.address, || {
        let _ = validate_refund_preconditions(&env, &alice);
    });
}

/// @notice Panics when refunds are attempted after cancellation.
#[test]
#[should_panic(expected = "campaign is not active")]
fn test_validate_panics_on_cancelled_campaign() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 10_000);
    client.contribute(&alice, &10_000);

    client.cancel();
    env.ledger().set_timestamp(deadline + 1);

    env.as_contract(&client.address, || {
        let _ = validate_refund_preconditions(&env, &alice);
    });
}

// ── execute_refund_single ──────────────────────────────────────────────────

/// @notice Transfers the stored refund amount and clears contributor state.
#[test]
fn test_execute_transfers_tokens_and_updates_state() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint(&env, &token, &alice, 75_000);
    mint(&env, &token, &bob, 25_000);
    client.contribute(&alice, &75_000);
    client.contribute(&bob, &25_000);

    env.ledger().set_timestamp(deadline + 1);

    let token_client = token::Client::new(&env, &token);
    let alice_balance_before = token_client.balance(&alice);

    env.as_contract(&client.address, || {
        execute_refund_single(&env, &alice, 75_000).unwrap();
    });

    assert_eq!(token_client.balance(&alice), alice_balance_before + 75_000);
    assert_eq!(client.contribution(&alice), 0);
    assert_eq!(client.contribution(&bob), 25_000);
    assert_eq!(client.total_raised(), 25_000);
}

/// @notice Emits exactly one refund event for a successful execution.
#[test]
fn test_execute_emits_refund_event_once() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 40_000);
    client.contribute(&alice, &40_000);
    env.ledger().set_timestamp(deadline + 1);

    assert_eq!(count_refund_events(&env), 0);

    env.as_contract(&client.address, || {
        execute_refund_single(&env, &alice, 40_000).unwrap();
    });

    assert_eq!(count_refund_events(&env), 1);
}

/// @notice Handles large but valid refund amounts without arithmetic failure.
#[test]
fn test_execute_large_amount_no_overflow() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let large_amount: i128 = 1_000_000_000_000;
    init(&client, &creator, &token, large_amount * 2, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, large_amount);
    client.contribute(&alice, &large_amount);
    env.ledger().set_timestamp(deadline + 1);

    env.as_contract(&client.address, || {
        execute_refund_single(&env, &alice, large_amount).unwrap();
    });

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), large_amount);
}

/// @notice Returns `Overflow` without partially mutating state when execution input is inconsistent.
/// @security Validates the defensive arithmetic preflight added before storage writes.
#[test]
fn test_execute_overflow_preserves_state_and_balance() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 50_000);
    client.contribute(&alice, &50_000);
    env.ledger().set_timestamp(deadline + 1);

    let token_client = token::Client::new(&env, &token);
    let alice_balance_before = token_client.balance(&alice);

    let result = env.as_contract(&client.address, || {
        execute_refund_single(&env, &alice, 75_000)
    });

    assert_eq!(result, Err(ContractError::Overflow));
    assert_eq!(client.contribution(&alice), 50_000);
    assert_eq!(client.total_raised(), 50_000);
    assert_eq!(token_client.balance(&alice), alice_balance_before);
    assert_eq!(count_refund_events(&env), 0);
}

// ── refund_single entrypoint ───────────────────────────────────────────────

/// @notice Supports multiple independent contributor claims on the same campaign.
#[test]
fn test_refund_single_multiple_contributors_claim_independently() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    mint(&env, &token, &alice, 200_000);
    mint(&env, &token, &bob, 300_000);
    mint(&env, &token, &carol, 100_000);

    client.contribute(&alice, &200_000);
    client.contribute(&bob, &300_000);
    client.contribute(&carol, &100_000);
    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    client.refund_single(&bob);
    client.refund_single(&carol);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 200_000);
    assert_eq!(token_client.balance(&bob), 300_000);
    assert_eq!(token_client.balance(&carol), 100_000);
    assert_eq!(client.total_raised(), 0);
}

/// @notice Returns the full accumulated amount when a contributor funded multiple times.
#[test]
fn test_refund_single_accumulated_contributions() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 900_000);
    client.contribute(&alice, &300_000);
    client.contribute(&alice, &300_000);
    client.contribute(&alice, &300_000);
    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 900_000);
    assert_eq!(client.contribution(&alice), 0);
}

/// @notice Rejects a second claim after the contributor has already been refunded.
#[test]
fn test_refund_single_double_claim_returns_nothing_to_refund() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 500_000);
    client.contribute(&alice, &500_000);
    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    let result = client.try_refund_single(&alice);

    assert_eq!(result.unwrap_err().unwrap(), ContractError::NothingToRefund);
}

/// @notice Requires contributor auth and leaves state untouched when auth is missing.
#[test]
fn test_refund_single_requires_contributor_auth() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 500_000);
    client.contribute(&alice, &500_000);
    env.ledger().set_timestamp(deadline + 1);

    env.set_auths(&[]);
    let unauthorized = client.try_refund_single(&alice);
    assert!(unauthorized.is_err());
    assert_eq!(client.contribution(&alice), 500_000);

    client
        .mock_auths(&[MockAuth {
            address: &alice,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "refund_single",
                args: soroban_sdk::vec![&env, alice.clone().into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .refund_single(&alice);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 500_000);
}

/// @notice A contributor already swept by deprecated batch refund has nothing left to claim.
#[test]
fn test_refund_single_after_batch_refund_returns_nothing_to_refund() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 500_000);
    client.contribute(&alice, &500_000);
    env.ledger().set_timestamp(deadline + 1);

    client.refund();

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 500_000);

    let result = client.try_refund_single(&alice);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NothingToRefund);
}

/// @notice Refunds are always full principal and ignore platform fees.
#[test]
fn test_refund_single_ignores_platform_fee_configuration() {
    let (env, client, creator, token, _token_admin) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let platform = Address::generate(&env);

    client.initialize(
        &creator,
        &creator,
        &token,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(PlatformConfig {
            address: platform.clone(),
            fee_bps: 500,
        }),
        &None,
        &None,
    );

    let alice = Address::generate(&env);
    mint(&env, &token, &alice, 500_000);
    client.contribute(&alice, &500_000);
    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 500_000);
    assert_eq!(token_client.balance(&platform), 0);
}
