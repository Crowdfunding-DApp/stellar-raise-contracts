extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String as SorobanString, Vec,
};

use crate::{
    ContractError, CrowdfundContract, CrowdfundContractClient, MilestoneInput, MilestoneStatus,
    PlatformConfig,
};

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

/// Sets up a campaign funded to exactly `goal`, with an optional platform
/// fee, and a single milestone proposed for the full `goal` and approved by
/// the sole contributor.
/// Returns (env, client, creator).
fn setup_approved_single_milestone(
    goal: i128,
    fee_bps: Option<u32>,
) -> (Env, CrowdfundContractClient<'static>, Address) {
    let (env, token_addr, sac) = make_env();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 3_600;
    let config = fee_bps.map(|fee_bps| PlatformConfig {
        address: Address::generate(&env),
        fee_bps,
    });

    client.initialize(
        &admin,
        &creator,
        &token_addr,
        &goal,
        &deadline,
        &1_i128,
        &config,
        &None,
        &None,
        &7,
    );

    let contributor = Address::generate(&env);
    sac.mint(&contributor, &goal);
    client.contribute(&contributor, &goal);

    env.ledger().set_timestamp(deadline + 1);

    let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: SorobanString::from_str(&env, "full payout"),
        amount: goal,
    });
    client.propose_milestones(&creator, &milestones);

    // Sole contributor holds the entire basis, so a single yes vote
    // immediately crosses the approval threshold (yes_weight*2 > basis).
    client.vote_milestone(&contributor, &0u32, &true);
    assert_eq!(
        client.milestone(&0u32).unwrap().status,
        MilestoneStatus::Approved
    );

    (env, client, creator)
}

// ── fee math typed error tests ────────────────────────────────────────────────
// Mirrors withdraw_fee_overflow_returns_typed_error in withdraw_event_emission_test.rs:
// execute_release_milestone's fee arithmetic must return typed ContractError
// variants instead of panicking (see audit #28, and its reintroduction here).

/// A fee_bps/milestone-amount combination that overflows `amount * fee_bps`
/// should return `ContractError::FeeOverflow` rather than panicking.
#[test]
fn release_milestone_fee_overflow_returns_typed_error() {
    // goal chosen so that `goal * 2` (the voting-threshold multiply, used to
    // resolve the vote) stays well within i128, but `goal * fee_bps` (the
    // fee-math multiply) overflows.
    let goal: i128 = i128::MAX / 4;

    // Largest valid fee (fee_bps must be < 10_000): goal * 9_999 overflows i128.
    let (_env, client, creator) = setup_approved_single_milestone(goal, Some(9_999));

    let result = client.try_release_milestone(&creator, &0u32);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::FeeOverflow,
        "expected FeeOverflow when amount * fee_bps overflows i128"
    );

    // The transaction reverts entirely: the milestone stays Approved, not
    // half-flipped to Released, and total_raised is unchanged.
    assert_eq!(
        client.milestone(&0u32).unwrap().status,
        MilestoneStatus::Approved
    );
    assert_eq!(client.total_raised(), goal);
}

/// A normal, non-overflowing fee_bps/amount combination should release the
/// milestone successfully with no panic or typed error.
#[test]
fn release_milestone_with_valid_fee_does_not_error() {
    let goal: i128 = 10_000;
    let (_env, client, creator) = setup_approved_single_milestone(goal, Some(500)); // 5%

    client.release_milestone(&creator, &0u32);
    assert_eq!(
        client.milestone(&0u32).unwrap().status,
        MilestoneStatus::Released
    );
    assert_eq!(client.total_raised(), 0);
}
