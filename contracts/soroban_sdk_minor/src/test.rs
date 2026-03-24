#![cfg(test)]
use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

#[test]
fn test_init_and_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SorobanSdkMinor, ());
    let client = SorobanSdkMinorClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_check_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SorobanSdkMinor, ());
    let client = SorobanSdkMinorClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    assert!(client.check_auth(&user));
}
