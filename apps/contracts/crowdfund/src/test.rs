#![cfg(test)]

use crate::{
    ContractError, CrowdfundContract, CrowdfundContractClient, MilestoneInput, MilestoneStatus,
    Status,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

extern crate std;

// Mock NFT contract for testing
#[allow(dead_code)]
pub struct MockNftContract;

#[allow(dead_code)]
impl MockNftContract {
    #[allow(dead_code)]
    pub fn mint(_env: Env, _to: Address, _token_id: u64) {
        // Mock implementation
    }
}

#[allow(dead_code)]
pub struct MockNftContractClient<'a> {
    pub env: &'a Env,
    pub contract_id: &'a Address,
}

#[allow(dead_code)]
impl<'a> MockNftContractClient<'a> {
    #[allow(dead_code)]
    pub fn new(env: &'a Env, contract_id: &'a Address) -> Self {
        Self { env, contract_id }
    }

    #[allow(dead_code)]
    pub fn minted(&self) -> std::vec::Vec<MintedNft> {
        // Mock implementation
        std::vec::Vec::new()
    }
}

#[allow(dead_code)]
pub struct MintedNft {
    pub to: Address,
    pub token_id: u64,
}

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (Address, token::StellarAssetClient<'a>) {
    let token_contract_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract_id.address();
    let token_client = token::StellarAssetClient::new(env, &token_address);
    (token_address, token_client)
}

fn setup_env() -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let platform_admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_address, token_client) = create_token_contract(&env, &token_admin);

    (
        env,
        client,
        platform_admin,
        creator,
        token_address,
        token_client,
    )
}

#[allow(dead_code)]
fn default_init(
    client: &CrowdfundContractClient,
    creator: &Address,
    token_address: &Address,
    deadline: u64,
) -> Address {
    let admin = creator.clone();
    client.initialize(
        &admin,
        creator,
        token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );
    admin
}

#[test]
fn test_initialize() {
    let (env, client, platform_admin, creator, token_address, _token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    // Verify initialization was successful
    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_contribute() {
    let (env, client, platform_admin, creator, token_address, token_admin_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    let amount = 5_000;

    // Mint tokens to contributor so they can contribute
    token_admin_client.mint(&contributor, &amount);

    client.contribute(&contributor, &amount);

    // Verify contribution was recorded
    assert_eq!(client.total_raised(), amount);
    assert_eq!(client.contributors().len(), 1);
}

#[test]
fn test_withdraw() {
    let (env, client, platform_admin, creator, token_address, token_admin_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    let goal_amount = 1_000_000;

    // Mint tokens to contributor so they can contribute the full goal
    token_admin_client.mint(&contributor, &goal_amount);

    client.contribute(&contributor, &goal_amount);

    // Fast forward past deadline
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    // Verify withdrawal was successful - total_raised should be 0 after withdrawal
    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_initialize_twice_returns_error() {
    let (env, client, platform_admin, creator, token_address, _token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let result = client.try_initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    assert!(result.is_err());
}

#[test]
fn test_empty_registry() {
    let (_env, client, _platform_admin, _creator, _token_address, _token_client) = setup_env();

    // Verify empty state - these should be default values before initialization
    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.contributors().len(), 0);
}

#[test]
fn test_lifecycle_successful_campaign_withdraw() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    assert_eq!(client.total_raised(), goal);
    assert_eq!(client.contributors().len(), 1);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 0);
}

/// Underfunded path: Contributions below goal → deadline passes → Refund available
#[test]
fn test_lifecycle_underfunded_refunds() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;
    let contrib_amount = 100_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &contrib_amount);
    client.contribute(&contributor, &contrib_amount);

    assert_eq!(client.total_raised(), contrib_amount);

    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&contributor);

    assert_eq!(client.contribution(&contributor), 0);
}

/// Multiple backers: Each tracked independently and refunded correctly
#[test]
fn test_lifecycle_multiple_backers_refund() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contrib1 = Address::generate(&env);
    let contrib2 = Address::generate(&env);
    let contrib3 = Address::generate(&env);

    let amt1 = 50_000;
    let amt2 = 75_000;
    let amt3 = 40_000;

    token_client.mint(&contrib1, &amt1);
    token_client.mint(&contrib2, &amt2);
    token_client.mint(&contrib3, &amt3);

    client.contribute(&contrib1, &amt1);
    client.contribute(&contrib2, &amt2);
    client.contribute(&contrib3, &amt3);

    let total = amt1 + amt2 + amt3;
    assert_eq!(client.total_raised(), total);
    assert_eq!(client.contributors().len(), 3);

    assert_eq!(client.contribution(&contrib1), amt1);
    assert_eq!(client.contribution(&contrib2), amt2);
    assert_eq!(client.contribution(&contrib3), amt3);

    env.ledger().set_timestamp(deadline + 1);

    client.refund_single(&contrib1);
    client.refund_single(&contrib2);
    client.refund_single(&contrib3);

    assert_eq!(client.contribution(&contrib1), 0);
    assert_eq!(client.contribution(&contrib2), 0);
    assert_eq!(client.contribution(&contrib3), 0);
}

/// Contribution after deadline is rejected
#[test]
fn test_contribution_after_deadline_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    env.ledger().set_timestamp(deadline + 1);

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &100_000);

    let result = client.try_contribute(&contributor, &10_000);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), ContractError::CampaignEnded);
}

/// Contribution below min_contribution is rejected
#[test]
fn test_contribution_below_minimum_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let min_contrib = 5_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &min_contrib,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &100_000);

    let result = client.try_contribute(&contributor, &(min_contrib - 1));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), ContractError::BelowMinimum);
}

/// Zero-amount contribution is rejected
#[test]
fn test_contribution_zero_amount_rejected() {
    let (env, client, platform_admin, creator, token_address, _token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);

    let result = client.try_contribute(&contributor, &0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

/// Status transitions: Active → Successful after goal met and funds withdrawn
#[test]
fn test_status_transition_to_successful() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 500_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    let result = client.try_contribute(&contributor, &1_000);
    assert!(result.is_err());
}

/// Multiple accumulated contributions: Same backer contributes multiple times
#[test]
fn test_multiple_contributions_same_backer() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &500_000);

    client.contribute(&contributor, &100_000);
    assert_eq!(client.contribution(&contributor), 100_000);
    assert_eq!(client.total_raised(), 100_000);
    assert_eq!(client.contributors().len(), 1);

    client.contribute(&contributor, &200_000);
    assert_eq!(client.contribution(&contributor), 300_000);
    assert_eq!(client.total_raised(), 300_000);
    assert_eq!(client.contributors().len(), 1);

    client.contribute(&contributor, &150_000);
    assert_eq!(client.contribution(&contributor), 450_000);
    assert_eq!(client.total_raised(), 450_000);
}

// ── Milestone-gated partial release ──────────────────────────────────────────

/// Full happy path: propose two milestones, both approved by a majority
/// vote, both released, campaign completes.
#[test]
fn test_lifecycle_milestone_happy_path_full_release() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);

    let schedule = soroban_sdk::vec![
        &env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 1"),
            amount: 600_000,
        },
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 2"),
            amount: 400_000,
        },
    ];
    client.propose_milestones(&creator, &schedule);
    assert_eq!(client.milestones().len(), 2);

    client.vote_milestone(&contributor, &0, &true);
    assert_eq!(client.milestone(&0).unwrap().status, MilestoneStatus::Approved);

    client.release_milestone(&creator, &0);
    assert_eq!(client.milestone(&0).unwrap().status, MilestoneStatus::Released);
    assert_eq!(client.total_raised(), 400_000);
    assert_eq!(client.status(), Status::Active);

    client.vote_milestone(&contributor, &1, &true);
    client.release_milestone(&creator, &1);

    assert_eq!(client.milestone(&1).unwrap().status, MilestoneStatus::Released);
    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.status(), Status::Successful);
}

/// A rejected milestone's funds go to pro-rata refund, not the creator; the
/// campaign still completes once every milestone is settled.
#[test]
fn test_lifecycle_milestone_rejected_then_refunded() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);

    let schedule = soroban_sdk::vec![
        &env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 1"),
            amount: 700_000,
        },
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 2"),
            amount: 300_000,
        },
    ];
    client.propose_milestones(&creator, &schedule);

    client.vote_milestone(&contributor, &0, &false);
    assert_eq!(client.milestone(&0).unwrap().status, MilestoneStatus::Rejected);
    assert_eq!(client.status(), Status::Active);

    client.claim_milestone_refund(&contributor, &0);
    assert!(client.has_claimed_milestone_refund(&0, &contributor));
    assert_eq!(client.total_raised(), 300_000);

    client.vote_milestone(&contributor, &1, &true);
    client.release_milestone(&creator, &1);

    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.status(), Status::Successful);
}

/// A milestone whose voting window closes without crossing either threshold
/// (an exact 50/50 split here) resolves to Rejected, not Approved.
#[test]
fn test_lifecycle_milestone_voting_timeout_defaults_to_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    token_client.mint(&c1, &500_000);
    token_client.mint(&c2, &500_000);
    client.contribute(&c1, &500_000);
    client.contribute(&c2, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    let schedule = soroban_sdk::vec![
        &env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "only phase"),
            amount: 1_000_000,
        },
    ];
    client.propose_milestones(&creator, &schedule);

    // Exactly half the basis votes yes -> stays Pending (must strictly exceed half).
    client.vote_milestone(&c1, &0, &true);
    assert_eq!(client.milestone(&0).unwrap().status, MilestoneStatus::Pending);

    let voting_deadline = client.milestone(&0).unwrap().voting_deadline;
    env.ledger().set_timestamp(voting_deadline + 1);

    client.finalize_milestone_vote(&0);
    assert_eq!(client.milestone(&0).unwrap().status, MilestoneStatus::Rejected);
    assert_eq!(client.status(), Status::Successful);

    client.claim_milestone_refund(&c1, &0);
    client.claim_milestone_refund(&c2, &0);
    assert_eq!(client.total_raised(), 0);
}

fn propose_single_milestone_for_full_goal(
    env: &Env,
    client: &CrowdfundContractClient,
    creator: &Address,
    amount: i128,
) {
    let schedule = soroban_sdk::vec![
        env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(env, "only phase"),
            amount,
        },
    ];
    client.propose_milestones(creator, &schedule);
}

#[test]
fn test_milestone_double_vote_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    token_client.mint(&c1, &500_000);
    token_client.mint(&c2, &500_000);
    client.contribute(&c1, &500_000);
    client.contribute(&c2, &500_000);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    // c1's vote alone doesn't cross either threshold, so the milestone stays
    // Pending and a second vote from c1 hits AlreadyVoted (not MilestoneNotPending).
    client.vote_milestone(&c1, &0, &true);
    let result = client.try_vote_milestone(&c1, &0, &true);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), ContractError::AlreadyVoted);
}

#[test]
fn test_milestone_release_before_approval_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let result = client.try_release_milestone(&creator, &0);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneNotApproved
    );
}

#[test]
fn test_milestone_propose_sum_mismatch_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);

    let schedule = soroban_sdk::vec![
        &env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 1"),
            amount: 500_000,
        },
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "phase 2"),
            amount: 400_000,
        },
    ];
    let result = client.try_propose_milestones(&creator, &schedule);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidMilestoneSchedule
    );
}

#[test]
fn test_milestone_propose_twice_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let schedule = soroban_sdk::vec![
        &env,
        MilestoneInput {
            description: soroban_sdk::String::from_str(&env, "second attempt"),
            amount: goal,
        },
    ];
    let result = client.try_propose_milestones(&creator, &schedule);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestonesAlreadyProposed
    );
}

#[test]
fn test_milestone_vote_zero_weight_rejected() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let stranger = Address::generate(&env);
    let result = client.try_vote_milestone(&stranger, &0, &true);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::NoContributionWeight
    );
}

#[test]
fn test_withdraw_blocked_once_milestones_active() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let result = client.try_withdraw();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneModeActive
    );
}

#[test]
fn test_cancel_blocked_once_milestones_active() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let result = client.try_cancel();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneModeActive
    );
}

#[test]
fn test_collect_pledges_blocked_once_milestones_active() {
    let (env, client, platform_admin, creator, token_address, token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    let goal = 1_000_000;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &goal,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);
    propose_single_milestone_for_full_goal(&env, &client, &creator, goal);

    let result = client.try_collect_pledges();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneModeActive
    );
}
