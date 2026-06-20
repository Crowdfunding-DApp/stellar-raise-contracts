extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Events, Ledger},
    token,
    xdr::{ContractEventBody, ScString, ScVal},
    Address, Env, TryFromVal, Val,
};

use crate::{
    withdraw_event_emission::{emit_fee_transferred, emit_nft_batch_minted, emit_withdrawn},
    CrowdfundContract, CrowdfundContractClient, PlatformConfig, MAX_NFT_MINT_BATCH,
};

// ── Mock NFT contract ─────────────────────────────────────────────────────────

#[derive(Clone)]
#[contracttype]
enum MockNftKey {
    Count,
}

#[contract]
struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(env: Env, _to: Address) -> u128 {
        let n: u32 = env
            .storage()
            .instance()
            .get(&MockNftKey::Count)
            .unwrap_or(0);
        env.storage().instance().set(&MockNftKey::Count, &(n + 1));
        n as u128
    }

    pub fn count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&MockNftKey::Count)
            .unwrap_or(0)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_env() -> (Env, Address, token::StellarAssetClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token_addr = token_id.address();
    let sac = token::StellarAssetClient::new(&env, &token_addr);
    (env, token_addr, sac)
}

/// Set up a funded campaign at exactly `goal` total raised, with an NFT contract.
/// Returns (env, client, creator, token_addr, nft_id, deadline).
fn setup_with_nft(
    contributor_count: u32,
) -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
    u64,
) {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);
    let nft_id = env.register(MockNft, ());

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    let goal = contributor_count as i128 * 100;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &goal,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );
    client.set_nft_contract(&creator, &nft_id);

    for _ in 0..contributor_count {
        let c = Address::generate(&env);
        sac.mint(&c, &100);
        client.contribute(&c, &100);
    }

    (env, client, creator, token_addr, nft_id, deadline)
}

/// Set up a campaign without an NFT contract.
/// The single contributor puts in exactly `goal` tokens.
fn setup_no_nft(
    goal: i128,
) -> (
    Env,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
    u64,
) {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &goal,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let c = Address::generate(&env);
    sac.mint(&c, &goal);
    client.contribute(&c, &goal);

    (env, client, creator, token_addr, c, deadline)
}

fn topics_match(e: &soroban_sdk::xdr::ContractEvent, ns: &str, action: &str) -> bool {
    let ContractEventBody::V0(body) = &e.body;
    if body.topics.len() < 2 {
        return false;
    }
    let ns_str = ScVal::String(ScString(ns.try_into().unwrap()));
    let act_str = ScVal::String(ScString(action.try_into().unwrap()));
    body.topics[0] == ns_str && body.topics[1] == act_str
}

fn event_data_as_val(env: &Env, e: &soroban_sdk::xdr::ContractEvent) -> Val {
    let ContractEventBody::V0(body) = &e.body;
    Val::try_from_val(env, &body.data).unwrap()
}

/// Count events matching a `("crowdfund", action)` topic pair.
fn count_events(env: &Env, action: &str) -> usize {
    env.events()
        .all()
        .events()
        .iter()
        .filter(|e| topics_match(e, "crowdfund", action))
        .count()
}

/// Return data of the first event matching `("crowdfund", action)`.
fn first_event_data(env: &Env, action: &str) -> Option<Val> {
    env.events()
        .all()
        .events()
        .iter()
        .find(|e| topics_match(e, "crowdfund", action))
        .map(|e| event_data_as_val(env, e))
}

/// Return all events matching `("crowdfund", action)`.
fn all_events_data(env: &Env, action: &str) -> std::vec::Vec<Val> {
    env.events()
        .all()
        .events()
        .iter()
        .filter(|e| topics_match(e, "crowdfund", action))
        .map(|e| event_data_as_val(env, e))
        .collect()
}

// ── contributed event ─────────────────────────────────────────────────────────

#[test]
fn contributed_event_is_emitted_on_contribution() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let backer = Address::generate(&env);
    sac.mint(&backer, &500);
    client.contribute(&backer, &500);

    assert_eq!(count_events(&env, "contributed"), 1);
}

#[test]
fn contributed_event_data_has_backer_amount_total_raised() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let backer = Address::generate(&env);
    sac.mint(&backer, &300);
    client.contribute(&backer, &300);

    let data = first_event_data(&env, "contributed").expect("contributed event not found");
    let (got_backer, got_amount, got_total): (Address, i128, i128) =
        TryFromVal::try_from_val(&env, &data).expect("decode failed");
    assert_eq!(got_backer, backer);
    assert_eq!(got_amount, 300);
    assert_eq!(got_total, 300);
}

#[test]
fn contributed_event_total_raised_accumulates() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    sac.mint(&b1, &200);
    sac.mint(&b2, &300);

    // Events are scoped to the last invocation, so check after each call.
    client.contribute(&b1, &200);
    let events_1 = all_events_data(&env, "contributed");
    assert_eq!(events_1.len(), 1);
    let (_, _, total1): (Address, i128, i128) =
        TryFromVal::try_from_val(&env, &events_1[0]).expect("decode 1");
    assert_eq!(total1, 200);

    client.contribute(&b2, &300);
    let events_2 = all_events_data(&env, "contributed");
    assert_eq!(events_2.len(), 1);
    let (_, _, total2): (Address, i128, i128) =
        TryFromVal::try_from_val(&env, &events_2[0]).expect("decode 2");
    assert_eq!(total2, 500);
}

// ── goal_reached event ────────────────────────────────────────────────────────

#[test]
fn goal_reached_event_emitted_exactly_once_when_goal_met() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &500,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    sac.mint(&b1, &300);
    sac.mint(&b2, &300);

    // First contribution — below goal
    client.contribute(&b1, &300);
    assert_eq!(count_events(&env, "goal_reached"), 0);

    // Second contribution — reaches goal
    client.contribute(&b2, &200);
    assert_eq!(count_events(&env, "goal_reached"), 1);

    // Verify data: total_raised and goal
    let data = first_event_data(&env, "goal_reached").expect("goal_reached not found");
    let (total_raised, goal): (i128, i128) = TryFromVal::try_from_val(&env, &data).expect("decode");
    assert_eq!(total_raised, 500);
    assert_eq!(goal, 500);
}

#[test]
fn goal_reached_not_emitted_when_goal_not_met() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let backer = Address::generate(&env);
    sac.mint(&backer, &500);
    client.contribute(&backer, &500);

    assert_eq!(count_events(&env, "goal_reached"), 0);
}

#[test]
fn goal_reached_not_emitted_twice_when_exceeded() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &100,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    // Events are scoped to the last invocation: check per-call.
    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    let b3 = Address::generate(&env);
    sac.mint(&b1, &200);
    sac.mint(&b2, &200);
    sac.mint(&b3, &200);

    // First contribution exceeds the goal — goal_reached fires.
    client.contribute(&b1, &200);
    assert_eq!(count_events(&env, "goal_reached"), 1);

    // Subsequent contributions — flag already set, event must NOT fire.
    client.contribute(&b2, &200);
    assert_eq!(count_events(&env, "goal_reached"), 0);

    client.contribute(&b3, &200);
    assert_eq!(count_events(&env, "goal_reached"), 0);
}

// ── withdrawn event ───────────────────────────────────────────────────────────

#[test]
fn withdrawn_event_emitted_once_on_withdraw() {
    let (env, client, _creator, _token, _c, deadline) = setup_no_nft(1_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();
    assert_eq!(count_events(&env, "withdrawn"), 1);
}

#[test]
fn withdrawn_event_data_has_creator_amount_fee() {
    let (env, client, creator, _token, _c, deadline) = setup_no_nft(1_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    let data = first_event_data(&env, "withdrawn").expect("withdrawn not found");
    let (got_creator, got_amount, got_fee): (Address, i128, i128) =
        TryFromVal::try_from_val(&env, &data).expect("decode");
    assert_eq!(got_creator, creator);
    assert_eq!(got_amount, 1_000);
    assert_eq!(got_fee, 0);
}

#[test]
fn withdrawn_event_reflects_platform_fee_deduction() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let platform = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    let goal: i128 = 1_000_000;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &goal,
        &deadline,
        &1_i128,
        &Some(PlatformConfig {
            address: platform,
            fee_bps: 500, // 5%
        }),
        &None,
        &None,
    );

    let c = Address::generate(&env);
    sac.mint(&c, &goal);
    client.contribute(&c, &goal);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    let data = first_event_data(&env, "withdrawn").expect("withdrawn not found");
    let (_, got_amount, got_fee): (Address, i128, i128) =
        TryFromVal::try_from_val(&env, &data).expect("decode");

    // 5% of 1_000_000 = 50_000 fee; creator receives 950_000
    assert_eq!(got_fee, 50_000);
    assert_eq!(got_amount, 950_000);
}

#[test]
fn fee_transferred_event_emitted_when_platform_configured() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let platform = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    let goal: i128 = 1_000_000;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &goal,
        &deadline,
        &1_i128,
        &Some(PlatformConfig {
            address: platform,
            fee_bps: 200, // 2%
        }),
        &None,
        &None,
    );

    let c = Address::generate(&env);
    sac.mint(&c, &goal);
    client.contribute(&c, &goal);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    assert_eq!(count_events(&env, "fee_transferred"), 1);

    let data = first_event_data(&env, "fee_transferred").expect("fee_transferred not found");
    let (_, got_fee): (Address, i128) = TryFromVal::try_from_val(&env, &data).expect("decode");
    // 2% of 1_000_000 = 20_000
    assert_eq!(got_fee, 20_000);
}

#[test]
fn fee_transferred_not_emitted_without_platform_config() {
    let (env, client, _creator, _token, _c, deadline) = setup_no_nft(1_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();
    assert_eq!(count_events(&env, "fee_transferred"), 0);
}

// ── NFT batch event ───────────────────────────────────────────────────────────

#[test]
fn nft_batch_minted_event_emitted_when_nft_contract_set() {
    let (env, client, _creator, _token, _nft_id, deadline) = setup_with_nft(3);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();
    assert_eq!(count_events(&env, "nft_batch_minted"), 1);
}

#[test]
fn nft_batch_minted_event_not_emitted_without_nft_contract() {
    let (env, client, _creator, _token, _c, deadline) = setup_no_nft(1_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();
    assert_eq!(count_events(&env, "nft_batch_minted"), 0);
}

#[test]
fn nft_batch_minted_event_data_matches_minted_count() {
    let count: u32 = 4;
    let (env, client, _creator, _token, nft_id, deadline) = setup_with_nft(count);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    let data = first_event_data(&env, "nft_batch_minted").expect("nft_batch_minted not found");
    let minted: u32 = TryFromVal::try_from_val(&env, &data).expect("decode");
    assert_eq!(minted, count);

    let nft = MockNftClient::new(&env, &nft_id);
    assert_eq!(nft.count(), count);
}

#[test]
fn nft_minting_capped_at_max_batch() {
    let count = MAX_NFT_MINT_BATCH + 5;
    let (env, client, _creator, _token, nft_id, deadline) = setup_with_nft(count);
    // Withdraw mints MAX_NFT_MINT_BATCH sub-contract calls; reset budget to unlimited
    // so the auth-tracking cost of those calls doesn't hit the mainnet per-tx limit.
    let mut budget = env.cost_estimate().budget();
    budget.reset_unlimited();
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    // Check events before any further contract calls — each invocation clears the event buffer.
    let data = first_event_data(&env, "nft_batch_minted").expect("nft_batch_minted not found");
    let minted: u32 = TryFromVal::try_from_val(&env, &data).expect("decode");
    assert_eq!(minted, MAX_NFT_MINT_BATCH);

    let nft = MockNftClient::new(&env, &nft_id);
    assert_eq!(nft.count(), MAX_NFT_MINT_BATCH);
}

// ── refunded event ────────────────────────────────────────────────────────────

#[test]
fn refunded_event_emitted_by_refund_single() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let backer = Address::generate(&env);
    sac.mint(&backer, &300);
    client.contribute(&backer, &300);

    // goal not met; advance past deadline
    env.ledger().set_timestamp(deadline + 1);
    client.refund_single(&backer);

    assert_eq!(count_events(&env, "refunded"), 1);

    let data = first_event_data(&env, "refunded").expect("refunded not found");
    let (got_backer, got_amount): (Address, i128) =
        TryFromVal::try_from_val(&env, &data).expect("decode");
    assert_eq!(got_backer, backer);
    assert_eq!(got_amount, 300);
}

#[test]
fn refunded_event_emitted_per_backer_in_batch_refund() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;

    // goal = 1000, total contributions = 600 (goal not met)
    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let mut backers: std::vec::Vec<Address> = std::vec::Vec::new();
    for _ in 0..3 {
        let b = Address::generate(&env);
        sac.mint(&b, &200);
        client.contribute(&b, &200);
        backers.push(b);
    }

    env.ledger().set_timestamp(deadline + 1);
    client.refund();

    // One refunded event per backer
    assert_eq!(count_events(&env, "refunded"), 3);
}

// ── cancelled event ───────────────────────────────────────────────────────────

#[test]
fn cancelled_event_emitted_on_cancel() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    let b = Address::generate(&env);
    sac.mint(&b, &200);
    client.contribute(&b, &200);

    client.cancel();

    assert_eq!(count_events(&env, "cancelled"), 1);
}

#[test]
fn cancel_emits_refunded_for_each_contributor() {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &1_000,
        &deadline,
        &1_i128,
        &None,
        &None,
        &None,
    );

    for _ in 0..3 {
        let b = Address::generate(&env);
        sac.mint(&b, &100);
        client.contribute(&b, &100);
    }

    client.cancel();

    assert_eq!(count_events(&env, "refunded"), 3);
    assert_eq!(count_events(&env, "cancelled"), 1);
}

// ── double-withdraw guard ─────────────────────────────────────────────────────

#[test]
#[should_panic]
fn double_withdraw_panics() {
    let (env, client, _creator, _token, _c, deadline) = setup_no_nft(1_000);
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();
    client.withdraw();
}

// ── Security unit tests for emit helpers ──────────────────────────────────────

#[test]
#[should_panic]
fn emit_fee_transferred_panics_on_zero_fee() {
    let env = Env::default();
    emit_fee_transferred(&env, &Address::generate(&env), 0);
}

#[test]
#[should_panic]
fn emit_fee_transferred_panics_on_negative_fee() {
    let env = Env::default();
    emit_fee_transferred(&env, &Address::generate(&env), -1);
}

#[test]
fn emit_fee_transferred_accepts_positive_fee() {
    let env = Env::default();
    emit_fee_transferred(&env, &Address::generate(&env), 1_000);
}

#[test]
#[should_panic]
fn emit_nft_batch_minted_panics_on_zero() {
    let env = Env::default();
    emit_nft_batch_minted(&env, 0);
}

#[test]
fn emit_nft_batch_minted_accepts_positive() {
    let env = Env::default();
    emit_nft_batch_minted(&env, 1);
}

#[test]
#[should_panic]
fn emit_withdrawn_panics_on_zero_payout() {
    let env = Env::default();
    emit_withdrawn(&env, &Address::generate(&env), 0, 0);
}

#[test]
#[should_panic]
fn emit_withdrawn_panics_on_negative_payout() {
    let env = Env::default();
    emit_withdrawn(&env, &Address::generate(&env), -100, 0);
}

#[test]
fn emit_withdrawn_accepts_valid_args() {
    let env = Env::default();
    emit_withdrawn(&env, &Address::generate(&env), 1_000, 50);
}
