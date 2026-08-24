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
    token, Address, Env, String as SorobanString, Vec,
};

extern crate std;

use crate::refund_single_token::execute_refund_single;
use crate::{
    ContractError, CrowdfundContract, CrowdfundContractClient, MilestoneInput, MilestoneStatus,
};

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
        creator, creator, token, &goal, &deadline, &1_000, &None, &None, &None, &7,
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

    let result = client.try_refund_single(&alice);

    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::CampaignNotActive,
        "refund_single must return CampaignNotActive once the campaign is Successful"
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

// === Milestone mode guard (double-spend prevention)
//
// Once a milestone schedule exists, execute_release_milestone pays out
// slices of TotalRaised to the creator without ever flipping Status to a
// terminal state. A refund path that only checks `total < goal` after the
// deadline would treat that drop as "contributor is owed a refund" and pay
// the same capital out a second time. Both refund_single and the deprecated
// refund() must refuse to run at all once milestones are in play —
// claim_milestone_refund is the only legal refund route from that point on.

#[test]
fn test_refund_single_rejected_once_milestone_mode_active() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal = 1_000_000;
    init_campaign(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &goal);
    client.contribute(&alice, &goal);

    env.ledger().set_timestamp(deadline + 1);

    // Two milestones so releasing the first leaves the schedule (and
    // Status::Active) unresolved — the second milestone is still Pending.
    // TotalRaised drops from `goal` to 400_000, below `goal`, exactly the
    // state validate_refund_preconditions's deadline-model checks would
    // otherwise treat as "contributor is owed a refund".
    let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: SorobanString::from_str(&env, "first slice"),
        amount: 600_000,
    });
    milestones.push_back(MilestoneInput {
        description: SorobanString::from_str(&env, "second slice"),
        amount: 400_000,
    });
    client.propose_milestones(&creator, &milestones);
    client.vote_milestone(&alice, &0u32, &true);
    assert_eq!(
        client.milestone(&0u32).unwrap().status,
        MilestoneStatus::Approved
    );
    client.release_milestone(&creator, &0u32);
    assert_eq!(client.total_raised(), 400_000);

    // Alice's Contribution record is untouched by milestone release, so a
    // deadline-model refund would (incorrectly) still see her as owed
    // `goal` on top of the milestone payout the creator already received.
    let result = client.try_refund_single(&alice);

    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneModeActive,
        "refund_single must refuse once milestones exist, not pay out a second time"
    );
}

#[test]
fn test_refund_batch_rejected_once_milestone_mode_active() {
    let (env, client, creator, token, token_admin_client) = setup();
    let deadline = env.ledger().timestamp() + 3_600;
    let goal = 1_000_000;
    init_campaign(&client, &creator, &token, goal, deadline);

    let alice = Address::generate(&env);
    token_admin_client.mint(&alice, &goal);
    client.contribute(&alice, &goal);

    env.ledger().set_timestamp(deadline + 1);

    // Two milestones so releasing the first leaves the schedule (and
    // Status::Active) unresolved — see the sibling refund_single test above
    // for why this matters.
    let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: SorobanString::from_str(&env, "first slice"),
        amount: 600_000,
    });
    milestones.push_back(MilestoneInput {
        description: SorobanString::from_str(&env, "second slice"),
        amount: 400_000,
    });
    client.propose_milestones(&creator, &milestones);
    client.vote_milestone(&alice, &0u32, &true);
    client.release_milestone(&creator, &0u32);
    assert_eq!(client.total_raised(), 400_000);

    let result = client.try_refund();

    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneModeActive,
        "deprecated refund() must refuse once milestones exist, not pay out a second time"
    );
}
