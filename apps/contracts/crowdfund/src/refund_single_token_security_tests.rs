//! Security regression tests for the `refund_single` pull-based refund path.
//!
//! These tests exercise the highest-risk fund-movement code in the
//! contract: double-refund prevention, campaign-status guards, and the
//! end-state invariant that the contract holds zero tokens once every
//! backer has claimed their refund.
//!
//! Run with:
//!   cargo test -p crowdfund refund_single_token_security -- --nocapture

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

extern crate std;
use std::panic;

use crate::refund_single_token::execute_refund_single;
use crate::{ContractError, CrowdfundContract, CrowdfundContractClient};

// === Helpers

fn setup() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();
    let token_client = token::StellarAssetClient::new(&env, &token_address);

    (env, client, creator, token_address, token_client)
}

fn init_campaign(
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

// === Double refund

#[test]
fn test_double_refund_rejected() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init_campaign(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &500_000);
    client.contribute(&alice, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    let second = client.try_refund_single(&alice);

    assert_eq!(second.unwrap_err().unwrap(), ContractError::NothingToRefund);
}

// === Status guards

#[test]
fn test_refund_rejected_while_active() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init_campaign(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &500_000);
    client.contribute(&alice, &500_000);

    // Deadline has not passed - campaign is still Active.
    let result = client.try_refund_single(&alice);

    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::CampaignStillActive
    );
}

#[test]
fn test_refund_rejected_when_successful() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal = 1_000_000;
    init_campaign(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &goal);
    client.contribute(&alice, &goal);

    env.ledger().set_timestamp(deadline + 1);
    client.withdraw(); // Active -> Successful

    let alice_for_panic = alice.clone();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        client.refund_single(&alice_for_panic);
    }));

    assert!(
        result.is_err(),
        "refund_single must panic once the campaign is Successful"
    );
}

// === Non-contributor

#[test]
fn test_refund_rejected_for_non_contributor() {
    let (env, client, creator, token, _token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init_campaign(&client, &creator, &token, 1_000_000, deadline);

    env.ledger().set_timestamp(deadline + 1);

    let stranger = Address::generate(&env);
    let result = client.try_refund_single(&stranger);

    assert_eq!(result.unwrap_err().unwrap(), ContractError::NothingToRefund);
}

// === Defense in depth: execute_refund_single can no longer be handed a forged amount

#[test]
fn test_execute_refund_single_derives_amount_from_storage() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init_campaign(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &250_000);
    client.contribute(&alice, &250_000);

    env.ledger().set_timestamp(deadline + 1);

    // execute_refund_single no longer accepts a caller-supplied amount; it
    // always refunds exactly what is on record for the contributor.
    let refunded = env.as_contract(&client.address, || {
        execute_refund_single(&env, &alice).unwrap()
    });

    assert_eq!(refunded, 250_000);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&alice), 250_000);
    assert_eq!(client.contribution(&alice), 0);
}

// === Zero dust after all backers refund

#[test]
fn test_all_backers_refunded_leaves_zero_contract_balance() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    init_campaign(&client, &creator, &token, 1_000_000, deadline);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);

    token_admin_client.mint(&alice, &200_000);
    token_admin_client.mint(&bob, &150_000);
    token_admin_client.mint(&carol, &75_000);

    client.contribute(&alice, &200_000);
    client.contribute(&bob, &150_000);
    client.contribute(&carol, &75_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&alice);
    client.refund_single(&bob);
    client.refund_single(&carol);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(client.total_raised(), 0);
}
