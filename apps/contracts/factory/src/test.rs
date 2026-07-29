#![cfg(test)]
#![allow(clippy::too_many_arguments)]

use crate::{CampaignStatus, FactoryContract, FactoryContractClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal,
};

extern crate std;

// Import the crowdfund contract WASM.
#[allow(clippy::too_many_arguments)]
mod crowdfund_wasm {
    soroban_sdk::contractimport!(
        file = "../../../target/wasm32-unknown-unknown/release/crowdfund.wasm"
    );
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

fn setup_factory(mock_auths: bool) -> (Env, Address, Address, soroban_sdk::BytesN<32>) {
    let env = Env::default();
    if mock_auths {
        env.mock_all_auths();
    }

    let factory_id = env.register(FactoryContract, ());

    let token_admin = Address::generate(&env);
    let (token_address, _token_client) = create_token_contract(&env, &token_admin);
    let wasm_hash = env.deployer().upload_contract_wasm(crowdfund_wasm::WASM);

    (env, factory_id, token_address, wasm_hash)
}

fn create_campaign(
    factory: &FactoryContractClient<'_>,
    creator: &Address,
    token_address: &Address,
    wasm_hash: &soroban_sdk::BytesN<32>,
    goal: i128,
    deadline: u64,
) -> Address {
    factory.create_campaign(creator, token_address, &goal, &deadline, wasm_hash)
}

#[test]
fn test_create_single_campaign_registers_returned_address() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);
    let goal = 1000i128;
    let deadline = 100u64;

    let campaign_addr = create_campaign(
        &factory,
        &creator,
        &token_address,
        &wasm_hash,
        goal,
        deadline,
    );

    assert_ne!(campaign_addr, factory_id);
    assert_ne!(campaign_addr, token_address);

    let campaigns = factory.campaigns();
    assert_eq!(campaigns.len(), 1);
    assert_eq!(campaigns.get(0).unwrap(), campaign_addr);
    assert_eq!(factory.campaign_count(), 1);
}

#[test]
fn test_campaign_count_increments_after_each_deployment() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    assert_eq!(factory.campaign_count(), 0);

    let creator1 = Address::generate(&env);
    create_campaign(&factory, &creator1, &token_address, &wasm_hash, 1000, 100);
    assert_eq!(factory.campaign_count(), 1);

    let creator2 = Address::generate(&env);
    create_campaign(&factory, &creator2, &token_address, &wasm_hash, 2000, 200);
    assert_eq!(factory.campaign_count(), 2);

    let creator3 = Address::generate(&env);
    create_campaign(&factory, &creator3, &token_address, &wasm_hash, 3000, 300);
    assert_eq!(factory.campaign_count(), 3);
}

#[test]
fn test_multiple_campaigns_are_registered_in_insertion_order() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);

    let creators = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    let campaign1 = create_campaign(
        &factory,
        &creators[0],
        &token_address,
        &wasm_hash,
        1000,
        100,
    );
    let campaign2 = create_campaign(
        &factory,
        &creators[1],
        &token_address,
        &wasm_hash,
        2000,
        200,
    );
    let campaign3 = create_campaign(
        &factory,
        &creators[2],
        &token_address,
        &wasm_hash,
        3000,
        300,
    );

    let campaigns = factory.campaigns();
    assert_eq!(campaigns.len(), 3);
    assert_eq!(campaigns.get(0).unwrap(), campaign1);
    assert_eq!(campaigns.get(1).unwrap(), campaign2);
    assert_eq!(campaigns.get(2).unwrap(), campaign3);
    assert_eq!(factory.campaign_count(), 3);
}

#[test]
fn test_factory_deployed_campaign_is_callable() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);
    let goal = 5000i128;
    let deadline = 600u64;

    let campaign_addr = create_campaign(
        &factory,
        &creator,
        &token_address,
        &wasm_hash,
        goal,
        deadline,
    );
    let campaign = crowdfund_wasm::Client::new(&env, &campaign_addr);

    assert_eq!(campaign.goal(), goal);
    assert_eq!(campaign.deadline(), deadline);
}

#[test]
fn test_create_campaign_rejects_missing_creator_auth() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(false);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);

    let result =
        factory.try_create_campaign(&creator, &token_address, &1000i128, &100u64, &wasm_hash);

    assert!(result.is_err());
    assert_eq!(factory.campaign_count(), 0);
}

#[test]
fn test_create_campaign_rejects_non_creator_auth() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(false);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);
    let attacker = Address::generate(&env);
    let goal = 1000i128;
    let deadline = 100u64;

    let result = factory
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &factory_id,
                fn_name: "create_campaign",
                args: soroban_sdk::vec![
                    &env,
                    creator.clone().into_val(&env),
                    token_address.clone().into_val(&env),
                    goal.into_val(&env),
                    deadline.into_val(&env),
                    wasm_hash.clone().into_val(&env),
                ],
                sub_invokes: &[],
            },
        }])
        .try_create_campaign(&creator, &token_address, &goal, &deadline, &wasm_hash);

    assert!(result.is_err());
    assert_eq!(factory.campaign_count(), 0);
}

#[test]
fn test_duplicate_creator_salt_collision_is_rejected_without_registry_mutation() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);

    let first_campaign = create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);
    assert_eq!(factory.campaign_count(), 1);

    let result =
        factory.try_create_campaign(&creator, &token_address, &2000i128, &200u64, &wasm_hash);

    assert!(result.is_err());
    let campaigns = factory.campaigns();
    assert_eq!(campaigns.len(), 1);
    assert_eq!(campaigns.get(0).unwrap(), first_campaign);
}

#[test]
fn test_empty_registry() {
    let env = Env::default();

    let factory_id = env.register(FactoryContract, ());
    let factory = FactoryContractClient::new(&env, &factory_id);

    // Verify empty state.
    let campaigns = factory.campaigns();
    assert_eq!(campaigns.len(), 0);
    assert_eq!(factory.campaign_count(), 0);
}

// ── Pagination ───────────────────────────────────────────────────────────────

#[test]
fn test_campaigns_page_slices_the_raw_registry_in_order() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);

    let mut deployed = std::vec::Vec::new();
    for i in 0..5 {
        let creator = Address::generate(&env);
        deployed.push(create_campaign(
            &factory,
            &creator,
            &token_address,
            &wasm_hash,
            1000 + i,
            100 + i as u64,
        ));
    }

    let page = factory.campaigns_page(&1, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap(), deployed[1]);
    assert_eq!(page.get(1).unwrap(), deployed[2]);

    // Limit beyond the remaining length is clamped, not an error.
    let tail = factory.campaigns_page(&3, &10);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail.get(0).unwrap(), deployed[3]);
    assert_eq!(tail.get(1).unwrap(), deployed[4]);
}

#[test]
fn test_campaigns_page_out_of_range_returns_empty() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);
    create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);

    assert_eq!(factory.campaigns_page(&5, &10).len(), 0);
    assert_eq!(factory.campaigns_page(&0, &0).len(), 0);
}

// ── Moderation: admin bootstrap ─────────────────────────────────────────────

#[test]
fn test_initialize_sets_admin_and_rejects_reinitialization() {
    let (env, factory_id, ..) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);

    factory.initialize(&admin);
    assert_eq!(factory.admin(), admin);

    let other = Address::generate(&env);
    let result = factory.try_initialize(&other);
    assert!(result.is_err());
    assert_eq!(factory.admin(), admin);
}

#[test]
#[should_panic]
fn test_admin_getter_panics_before_initialize() {
    let (env, factory_id, ..) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    factory.admin();
}

// ── Moderation: flagging ────────────────────────────────────────────────────

#[test]
fn test_set_campaign_status_flags_a_campaign_and_hides_it_from_active_page() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);
    factory.initialize(&admin);

    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);
    let good = create_campaign(&factory, &creator1, &token_address, &wasm_hash, 1000, 100);
    let fraud = create_campaign(&factory, &creator2, &token_address, &wasm_hash, 2000, 200);

    assert_eq!(factory.campaign_status(&good), CampaignStatus::Active);
    assert_eq!(factory.campaign_status(&fraud), CampaignStatus::Active);

    factory.set_campaign_status(&admin, &fraud, &CampaignStatus::Flagged);
    assert_eq!(factory.campaign_status(&fraud), CampaignStatus::Flagged);

    // The raw log is untouched — still contains both.
    assert_eq!(factory.campaigns().len(), 2);

    // But the active-only page hides the flagged one.
    let active = factory.active_campaigns_page(&0, &10);
    assert_eq!(active.len(), 1);
    assert_eq!(active.get(0).unwrap(), good);
}

#[test]
fn test_set_campaign_status_can_reinstate_a_campaign() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);
    factory.initialize(&admin);

    let creator = Address::generate(&env);
    let campaign = create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);

    factory.set_campaign_status(&admin, &campaign, &CampaignStatus::Cancelled);
    assert_eq!(factory.active_campaigns_page(&0, &10).len(), 0);

    factory.set_campaign_status(&admin, &campaign, &CampaignStatus::Active);
    assert_eq!(factory.active_campaigns_page(&0, &10).len(), 1);
}

#[test]
fn test_set_campaign_status_rejects_non_admin_caller() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);
    factory.initialize(&admin);

    let creator = Address::generate(&env);
    let campaign = create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);

    let attacker = Address::generate(&env);
    let result = factory.try_set_campaign_status(&attacker, &campaign, &CampaignStatus::Flagged);
    assert!(result.is_err());
    assert_eq!(factory.campaign_status(&campaign), CampaignStatus::Active);
}

#[test]
fn test_set_campaign_status_rejects_unregistered_campaign() {
    let (env, factory_id, ..) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);
    factory.initialize(&admin);

    let not_a_campaign = Address::generate(&env);
    let result = factory.try_set_campaign_status(&admin, &not_a_campaign, &CampaignStatus::Flagged);
    assert!(result.is_err());
}

#[test]
fn test_set_campaign_status_rejects_before_initialize() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);
    let campaign = create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);

    let admin = Address::generate(&env);
    let result = factory.try_set_campaign_status(&admin, &campaign, &CampaignStatus::Flagged);
    assert!(result.is_err());
}

#[test]
fn test_active_campaigns_page_pagination_over_filtered_results() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(true);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let admin = Address::generate(&env);
    factory.initialize(&admin);

    let mut deployed = std::vec::Vec::new();
    for i in 0..5 {
        let creator = Address::generate(&env);
        deployed.push(create_campaign(
            &factory,
            &creator,
            &token_address,
            &wasm_hash,
            1000 + i,
            100 + i as u64,
        ));
    }
    // Flag the 2nd and 4th deployed campaigns.
    factory.set_campaign_status(&admin, &deployed[1], &CampaignStatus::Flagged);
    factory.set_campaign_status(&admin, &deployed[3], &CampaignStatus::Cancelled);

    // Active set, in deployment order, is [0, 2, 4].
    let all_active = factory.active_campaigns_page(&0, &10);
    assert_eq!(all_active.len(), 3);
    assert_eq!(all_active.get(0).unwrap(), deployed[0]);
    assert_eq!(all_active.get(1).unwrap(), deployed[2]);
    assert_eq!(all_active.get(2).unwrap(), deployed[4]);

    // Page boundaries index into the filtered set, not the raw registry.
    let page = factory.active_campaigns_page(&1, &1);
    assert_eq!(page.len(), 1);
    assert_eq!(page.get(0).unwrap(), deployed[2]);
}
