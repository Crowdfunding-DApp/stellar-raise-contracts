#![cfg(test)]
#![allow(clippy::too_many_arguments)]

use crate::{FactoryContract, FactoryContractClient};
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

    let campaign_addr =
        create_campaign(&factory, &creator, &token_address, &wasm_hash, goal, deadline);

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

    let campaign_addr =
        create_campaign(&factory, &creator, &token_address, &wasm_hash, goal, deadline);
    let campaign = crowdfund_wasm::Client::new(&env, &campaign_addr);

    assert_eq!(campaign.goal(), goal);
    assert_eq!(campaign.deadline(), deadline);
}

#[test]
fn test_create_campaign_rejects_missing_creator_auth() {
    let (env, factory_id, token_address, wasm_hash) = setup_factory(false);
    let factory = FactoryContractClient::new(&env, &factory_id);
    let creator = Address::generate(&env);

    let result = factory.try_create_campaign(
        &creator,
        &token_address,
        &1000i128,
        &100u64,
        &wasm_hash,
    );

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

    let first_campaign =
        create_campaign(&factory, &creator, &token_address, &wasm_hash, 1000, 100);
    assert_eq!(factory.campaign_count(), 1);

    let result = factory.try_create_campaign(
        &creator,
        &token_address,
        &2000i128,
        &200u64,
        &wasm_hash,
    );

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
