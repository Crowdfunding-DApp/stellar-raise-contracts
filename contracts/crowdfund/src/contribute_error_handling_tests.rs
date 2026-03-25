//! Tests for contribute() error handling.
//!
//! Covers every typed error path in `contribute()`:
//!   - `ZeroAmount`     — amount == 0
//!   - `AmountTooLow`   — amount < min_contribution
//!   - `CampaignEnded`  — contribution after deadline
//!   - `Overflow`       — error code constant correctness
//!   - happy-path sanity check
//!   - exact-deadline boundary (contribution at deadline timestamp accepted)
//!   - `describe_error` / `is_retryable` helper coverage

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{contribute_error_handling, ContractError, CrowdfundContract, CrowdfundContractClient};

// ── helpers ──────────────────────────────────────────────────────────────────

const GOAL: i128 = 1_000;
const MIN: i128 = 10;
const DEADLINE_OFFSET: u64 = 1_000;

fn setup() -> (Env, CrowdfundContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_id.address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);

    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);

    asset_client.mint(&contributor, &i128::MAX);

    let now = env.ledger().timestamp();
    client.initialize(
        &Address::generate(&env),
        &creator,
        &token_addr,
        &GOAL,
        &(now + DEADLINE_OFFSET),
        &MIN,
        &None,
        &None,
        &None,
    );

    (env, client, contributor, token_addr)
}

// ── happy path ───────────────────────────────────────────────────────────────

#[test]
fn contribute_happy_path() {
    let (env, client, contributor, _) = setup();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    client.contribute(&contributor, &MIN);
    assert_eq!(client.contribution(&contributor), MIN);
    assert_eq!(client.total_raised(), MIN);
}

// ── ZeroAmount ────────────────────────────────────────────────────────────────

#[test]
fn contribute_zero_amount_returns_zero_amount_error() {
    let (env, client, contributor, _) = setup();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = client.try_contribute(&contributor, &0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

// ── AmountTooLow ──────────────────────────────────────────────────────────────

#[test]
fn contribute_below_minimum_returns_amount_too_low() {
    let (env, client, contributor, _) = setup();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = client.try_contribute(&contributor, &(MIN - 1));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::AmountTooLow);
}

#[test]
fn contribute_one_below_minimum_returns_amount_too_low() {
    let (env, client, contributor, _) = setup();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    // MIN - 1 is the boundary just below the threshold
    let result = client.try_contribute(&contributor, &(MIN - 1));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::AmountTooLow);
}

// ── CampaignEnded ─────────────────────────────────────────────────────────────

#[test]
fn contribute_after_deadline_returns_campaign_ended() {
    let (env, client, contributor, _) = setup();
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + DEADLINE_OFFSET + 1);
    let result = client.try_contribute(&contributor, &MIN);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::CampaignEnded);
}

#[test]
fn contribute_exactly_at_deadline_is_accepted() {
    let (env, client, contributor, _) = setup();
    // timestamp == deadline → NOT past deadline (strict >), so accepted
    let deadline = client.deadline();
    env.ledger().set_timestamp(deadline);
    client.contribute(&contributor, &MIN);
    assert_eq!(client.total_raised(), MIN);
}

// ── Overflow ──────────────────────────────────────────────────────────────────

/// Verifies the Overflow error code constant matches the `#[repr(u32)]` value.
#[test]
fn overflow_error_code_matches_enum_repr() {
    assert_eq!(contribute_error_handling::error_codes::OVERFLOW, 6);
    assert_eq!(ContractError::Overflow as u32, 6);
}

// ── error_codes constants ─────────────────────────────────────────────────────

#[test]
fn error_code_constants_match_enum_reprs() {
    assert_eq!(
        contribute_error_handling::error_codes::CAMPAIGN_ENDED,
        ContractError::CampaignEnded as u32
    );
    assert_eq!(
        contribute_error_handling::error_codes::AMOUNT_TOO_LOW,
        ContractError::AmountTooLow as u32
    );
    assert_eq!(
        contribute_error_handling::error_codes::ZERO_AMOUNT,
        ContractError::ZeroAmount as u32
    );
}

// ── describe_error helpers ────────────────────────────────────────────────────

#[test]
fn describe_error_campaign_ended() {
    assert_eq!(
        contribute_error_handling::describe_error(
            contribute_error_handling::error_codes::CAMPAIGN_ENDED
        ),
        "Campaign has ended"
    );
}

#[test]
fn describe_error_overflow() {
    assert_eq!(
        contribute_error_handling::describe_error(contribute_error_handling::error_codes::OVERFLOW),
        "Arithmetic overflow — contribution amount too large"
    );
}

#[test]
fn describe_error_amount_too_low() {
    assert_eq!(
        contribute_error_handling::describe_error(
            contribute_error_handling::error_codes::AMOUNT_TOO_LOW
        ),
        "Amount is below the campaign minimum"
    );
}

#[test]
fn describe_error_zero_amount() {
    assert_eq!(
        contribute_error_handling::describe_error(
            contribute_error_handling::error_codes::ZERO_AMOUNT
        ),
        "Contribution amount must be greater than zero"
    );
}

#[test]
fn describe_error_unknown() {
    assert_eq!(
        contribute_error_handling::describe_error(99),
        "Unknown error"
    );
}

// ── is_retryable helpers ──────────────────────────────────────────────────────

#[test]
fn is_retryable_amount_too_low_and_zero_amount_are_retryable() {
    assert!(contribute_error_handling::is_retryable(
        contribute_error_handling::error_codes::AMOUNT_TOO_LOW
    ));
    assert!(contribute_error_handling::is_retryable(
        contribute_error_handling::error_codes::ZERO_AMOUNT
    ));
}

#[test]
fn is_retryable_campaign_ended_and_overflow_are_not_retryable() {
    assert!(!contribute_error_handling::is_retryable(
        contribute_error_handling::error_codes::CAMPAIGN_ENDED
    ));
    assert!(!contribute_error_handling::is_retryable(
        contribute_error_handling::error_codes::OVERFLOW
    ));
}
