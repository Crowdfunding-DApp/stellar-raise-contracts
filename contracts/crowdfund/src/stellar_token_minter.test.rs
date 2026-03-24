//! Tests for [`stellar_token_minter`](crate::stellar_token_minter) and withdraw NFT batching.
//!
//! This file merges:
//! - Unit tests for pure helpers (`mint_batch_is_full`, `bump_token_id_after_mint`).
//! - Integration tests previously in `withdraw_event_emission_test.rs` (were **not**
//!   wired into `lib.rs`, so they never ran).
//! - Crowdfund edge cases previously in `stellar_token_minter_test.rs`.

extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, TryFromVal,
};

use crate::stellar_token_minter::{
    bump_token_id_after_mint, mint_batch_is_full, MAX_NFT_MINT_BATCH, NFT_MINT_FN_NAME,
};
use crate::{CrowdfundContract, CrowdfundContractClient, MAX_NFT_MINT_BATCH as REEXPORT_CAP};

// ── Unit: pure helpers ───────────────────────────────────────────────────────

#[test]
fn nft_mint_fn_name_is_mint() {
    assert_eq!(NFT_MINT_FN_NAME, "mint");
}

#[test]
fn reexported_cap_matches_module() {
    assert_eq!(REEXPORT_CAP, MAX_NFT_MINT_BATCH);
}

#[test]
fn mint_batch_is_full_false_below_cap() {
    assert!(!mint_batch_is_full(0));
    assert!(!mint_batch_is_full(MAX_NFT_MINT_BATCH - 1));
}

#[test]
fn mint_batch_is_full_true_at_and_above_cap() {
    assert!(mint_batch_is_full(MAX_NFT_MINT_BATCH));
    assert!(mint_batch_is_full(MAX_NFT_MINT_BATCH + 1));
}

#[test]
fn bump_token_id_increments() {
    assert_eq!(bump_token_id_after_mint(1), 2);
    assert_eq!(bump_token_id_after_mint(u64::MAX - 1), u64::MAX);
    assert_eq!(bump_token_id_after_mint(u64::MAX), u64::MAX);
}

// ── Mock: bounded NFT (counts mint calls) ───────────────────────────────────

#[derive(Clone)]
#[contracttype]
enum BoundedNftKey {
    Count,
}

#[contract]
struct BoundedMockNft;

#[contractimpl]
impl BoundedMockNft {
    pub fn mint(env: Env, _to: Address, _token_id: u64) {
        let n: u32 = env
            .storage()
            .instance()
            .get(&BoundedNftKey::Count)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&BoundedNftKey::Count, &(n + 1));
    }
    pub fn count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&BoundedNftKey::Count)
            .unwrap_or(0)
    }
}

fn setup_bounded_nft_scenario(
    contributor_count: u32,
) -> (Env, CrowdfundContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_reg = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_reg.address();
    let sac = token::StellarAssetClient::new(&env, &token_addr);

    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &creator,
        &creator,
        &token_addr,
        &(contributor_count as i128 * 100),
        &deadline,
        &1,
        &None,
        &None,
        &None,
    );

    let nft_id = env.register(BoundedMockNft, ());
    client.set_nft_contract(&creator, &nft_id);

    for _ in 0..contributor_count {
        let c = Address::generate(&env);
        sac.mint(&c, &100);
        client.contribute(&c, &100);
    }

    env.ledger().set_timestamp(deadline + 1);

    (env, client, creator, token_addr, nft_id)
}

fn count_events_with_topic(env: &Env, t1: &str, t2: &str) -> usize {
    let s1 = String::from_str(env, t1);
    let s2 = String::from_str(env, t2);
    env.events()
        .all()
        .iter()
        .filter(|(_, topics, _)| {
            if topics.len() < 2 {
                return false;
            }
            let v1 = topics.get(0).unwrap();
            let v2 = topics.get(1).unwrap();
            String::try_from_val(env, &v1).map(|s| s == s1).unwrap_or(false)
                && String::try_from_val(env, &v2).map(|s| s == s2).unwrap_or(false)
        })
        .count()
}

#[test]
fn test_withdraw_mints_all_when_within_cap() {
    let count = MAX_NFT_MINT_BATCH - 1;
    let (env, client, _creator, _token, nft_id) = setup_bounded_nft_scenario(count);
    client.withdraw();

    let nft = BoundedMockNftClient::new(&env, &nft_id);
    assert_eq!(nft.count(), count);
}

#[test]
fn test_withdraw_caps_minting_at_max_batch() {
    let count = MAX_NFT_MINT_BATCH + 10;
    let (env, client, _creator, _token, nft_id) = setup_bounded_nft_scenario(count);
    client.withdraw();

    let nft = BoundedMockNftClient::new(&env, &nft_id);
    assert_eq!(nft.count(), MAX_NFT_MINT_BATCH);
}

#[test]
fn test_withdraw_mints_exactly_at_cap_boundary() {
    let (env, client, _creator, _token, nft_id) =
        setup_bounded_nft_scenario(MAX_NFT_MINT_BATCH);
    client.withdraw();

    let nft = BoundedMockNftClient::new(&env, &nft_id);
    assert_eq!(nft.count(), MAX_NFT_MINT_BATCH);
}

#[test]
fn test_withdraw_emits_single_batch_event() {
    let (env, client, _creator, _token, _nft_id) = setup_bounded_nft_scenario(5);
    client.withdraw();

    assert_eq!(
        count_events_with_topic(&env, "campaign", "nft_batch_minted"),
        1
    );
}

#[test]
fn test_withdraw_no_batch_event_without_nft_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_reg = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_reg.address();
    let sac = token::StellarAssetClient::new(&env, &token_addr);

    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &creator,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1,
        &None,
        &None,
        &None,
    );

    let contributor = Address::generate(&env);
    sac.mint(&contributor, &1_000);
    client.contribute(&contributor, &1_000);

    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    assert_eq!(
        count_events_with_topic(&env, "campaign", "nft_batch_minted"),
        0
    );
}

#[test]
fn test_withdraw_emits_withdrawn_event_once() {
    let (env, client, _creator, _token, _nft_id) = setup_bounded_nft_scenario(2);
    client.withdraw();

    assert_eq!(count_events_with_topic(&env, "campaign", "withdrawn"), 1);
}

#[test]
fn test_withdraw_emits_one_nft_batch_event_with_eligible_contributors() {
    let (env, client, _creator, _token, _nft_id) = setup_bounded_nft_scenario(1);
    client.withdraw();

    assert_eq!(
        count_events_with_topic(&env, "campaign", "nft_batch_minted"),
        1
    );
}

// ── Helpers: pledge / admin / stats (from legacy stellar_token_minter_test) ─

fn setup_env_simple(
) -> (Env, CrowdfundContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_reg = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_reg.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let creator = Address::generate(&env);
    token_admin_client.mint(&creator, &10_000_000);

    (env, client, creator, token_address, token_admin, contract_id)
}

fn mint_to(env: &Env, token_address: &Address, to: &Address, amount: i128) {
    let sac = token::StellarAssetClient::new(env, token_address);
    sac.mint(to, &amount);
}

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

/// `collect_pledges` returns `CampaignStillActive` before the deadline and
/// `GoalNotReached` after the deadline when pledges alone do not meet the goal.
#[test]
fn test_collect_pledges_guards_until_goal_met() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env_simple();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let pledger = Address::generate(&env);
    mint_to(&env, &token_address, &pledger, 600_000);

    client.pledge(&pledger, &500_000);

    let early = client.try_collect_pledges();
    assert_eq!(
        early.unwrap_err().unwrap(),
        crate::ContractError::CampaignStillActive
    );

    env.ledger().set_timestamp(deadline + 1);

    let late = client.try_collect_pledges();
    assert_eq!(
        late.unwrap_err().unwrap(),
        crate::ContractError::GoalNotReached
    );
}

/// Non-admin cannot authorize `upgrade` (Soroban auth failure).
#[test]
#[should_panic]
fn test_upgrade_only_admin_auth_required() {
    let (env, client, creator, token_address, _token_admin, contract_id) = setup_env_simple();
    let deadline = env.ledger().timestamp() + 3600;
    let _admin = default_init(&client, &creator, &token_address, deadline);

    let non_admin = Address::generate(&env);
    env.set_auths(&[]);
    client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "upgrade",
            args: soroban_sdk::vec![&env, soroban_sdk::BytesN::from_array(&env, &[0u8; 32]).into_val(&env)],
            sub_invokes: &[],
        },
    }]);

    client.upgrade(&soroban_sdk::BytesN::from_array(&env, &[0u8; 32]));
}

#[test]
fn test_bonus_goal_progress_bps_capped_at_100_percent() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env_simple();
    let deadline = env.ledger().timestamp() + 3600;
    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
        &Some(2_000_000i128),
        &None,
    );

    let a = Address::generate(&env);
    mint_to(&env, &token_address, &a, 2_500_000);
    client.contribute(&a, &2_500_000);

    assert!(client.bonus_goal_reached());
    assert_eq!(client.bonus_goal_progress_bps(), 10_000);
}

#[test]
fn test_get_stats_when_no_contributions() {
    let (env, client, creator, token_address, _token_admin, _contract_id) = setup_env_simple();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.contributor_count, 0);
    assert_eq!(stats.average_contribution, 0);
}
