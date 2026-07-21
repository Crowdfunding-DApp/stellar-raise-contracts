#![cfg(test)]

use crate::{ContractError, CrowdfundContract, CrowdfundContractClient};
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

// ── TTL / Rent-extension policy tests (issue #1306) ─────────────────────────

/// TTL constants must satisfy the invariant: LEDGER_BUMP_AMOUNT > LEDGER_THRESHOLD.
/// This prevents a pathological case where bumping never brings the TTL above
/// the threshold and the entry would be re-bumped on every single ledger.
#[test]
fn test_ttl_constants_invariant() {
    use crate::{LEDGER_BUMP_AMOUNT, LEDGER_THRESHOLD};
    assert!(
        LEDGER_BUMP_AMOUNT > LEDGER_THRESHOLD,
        "LEDGER_BUMP_AMOUNT ({}) must be greater than LEDGER_THRESHOLD ({}) \
         to ensure each bump provides a net TTL increase",
        LEDGER_BUMP_AMOUNT,
        LEDGER_THRESHOLD
    );
}

/// `keep_alive` is callable by anyone (no auth required) and succeeds even
/// when called on a freshly-initialized, contributor-free campaign.
#[test]
fn test_keep_alive_succeeds_without_contributors() {
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

    // Any address (not just the creator) can call keep_alive.
    let bystander = Address::generate(&env);
    let _ = bystander; // keep_alive takes no address argument
    client.keep_alive();
}

/// `keep_alive` also succeeds when contributors are present (persistent
/// Contributors list exists).
#[test]
fn test_keep_alive_succeeds_with_contributors() {
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
    token_client.mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    // keep_alive should not panic or fail.
    client.keep_alive();

    // State is unchanged.
    assert_eq!(client.total_raised(), 5_000);
    assert_eq!(client.contributors().len(), 1);
}

/// `keep_alive` can be called multiple times without side-effects.
#[test]
fn test_keep_alive_idempotent() {
    let (env, client, platform_admin, creator, token_address, _token_client) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &platform_admin,
        &creator,
        &token_address,
        &500_000,
        &deadline,
        &1_000,
        &None,
        &None,
        &None,
    );

    // Call keep_alive three times in a row — none should fail.
    client.keep_alive();
    client.keep_alive();
    client.keep_alive();

    assert_eq!(client.goal(), 500_000);
}

/// Every public entry-point implicitly extends instance TTL (smoke-test: the
/// contract remains fully functional after a round-trip through each path).
#[test]
fn test_all_entry_points_callable_after_initialization() {
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

    // View functions (each bumps instance TTL internally).
    let _ = client.goal();
    let _ = client.deadline();
    let _ = client.total_raised();
    let _ = client.min_contribution();
    let _ = client.contributors();
    let _ = client.title();
    let _ = client.description();
    let _ = client.socials();
    let _ = client.token();
    let _ = client.version();
    let _ = client.bonus_goal();
    let _ = client.bonus_goal_description();
    let _ = client.bonus_goal_reached();
    let _ = client.bonus_goal_progress_bps();
    let _ = client.current_milestone();
    let _ = client.get_stats();
    let _ = client.roadmap();

    // Mutating: contribute.
    let contributor = Address::generate(&env);
    token_client.mint(&contributor, &50_000);
    client.contribute(&contributor, &50_000);
    assert_eq!(client.contribution(&contributor), 50_000);

    // keep_alive.
    client.keep_alive();
}
