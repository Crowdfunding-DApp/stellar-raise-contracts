#![allow(missing_docs)]

//! # Milestone-Gated Partial Fund Release
//!
//! Backer-majority-vote gating for releasing raised funds to the creator in
//! discrete milestone slices, instead of the all-or-nothing [`crate::CrowdfundContract::withdraw`].
//!
//! ## Flow
//! ```text
//! execute_propose_milestones(creator, schedule)
//!   └─► schedule must sum exactly to total_raised (frozen as `MilestoneBasis`)
//! execute_vote_milestone(voter, id, approve)  [repeated by backers]
//!   └─► Approved once yes_weight*2 > MilestoneBasis
//!   └─► Rejected once no_weight*2 >= MilestoneBasis
//! execute_finalize_milestone_vote(id)          [permissionless backstop after voting_deadline]
//! execute_release_milestone(creator, id)       [only once Approved]
//! execute_claim_milestone_refund(contributor, id) [only once Rejected]
//! ```
//!
//! ## Why a frozen `MilestoneBasis` instead of the live `TotalRaised`
//!
//! `TotalRaised` decrements as milestones release or refund. Using it as the
//! vote-threshold/pro-rata denominator would let a milestone's required
//! majority silently shift depending on how many earlier milestones already
//! paid out. `MilestoneBasis` is written once at proposal time and never
//! touched again.
//!
//! ## CEI ordering
//!
//! `execute_release_milestone` and `execute_claim_milestone_refund` both
//! flip storage (milestone status / claimed flag / `TotalRaised`) *before*
//! the token transfer, mirroring [`crate::refund_single_token`].

use soroban_sdk::{token, Address, Env, Vec};

use crate::contract_state_size;
use crate::withdraw_event_emission::{
    emit_fee_transferred, emit_milestone_refund_claimed, emit_milestone_released,
    emit_milestone_resolved, emit_milestone_schedule_completed, emit_milestone_vote_cast,
    emit_milestones_proposed, mint_nfts_in_batch,
};
use crate::{
    ContractError, DataKey, Milestone, MilestoneInput, MilestoneRefundKey, MilestoneStatus,
    MilestoneVoteKey, PlatformConfig, Status,
};

/// How long a milestone stays open for voting before it can be force-resolved
/// by [`execute_finalize_milestone_vote`]. 14 days.
pub const MILESTONE_VOTING_PERIOD_SECS: u64 = 1_209_600;

// ── Pure validation helpers ───────────────────────────────────────────────────

/// Validates a proposed milestone schedule against the frozen `total_raised`.
///
/// Pure — no storage reads or writes. Every failure collapses to
/// [`ContractError::InvalidMilestoneSchedule`], matching how the existing
/// contributor/pledger/roadmap capacity checks already collapse to a single
/// generic error code.
pub fn validate_milestone_schedule(
    milestones: &Vec<MilestoneInput>,
    total_raised: i128,
) -> Result<(), ContractError> {
    let len = milestones.len();
    if len == 0 || !contract_state_size::validate_milestone_capacity(len) {
        return Err(ContractError::InvalidMilestoneSchedule);
    }

    let mut sum: i128 = 0;
    for item in milestones.iter() {
        if item.amount <= 0 {
            return Err(ContractError::InvalidMilestoneSchedule);
        }
        if !contract_state_size::validate_milestone_description(&item.description) {
            return Err(ContractError::InvalidMilestoneSchedule);
        }
        sum = sum.checked_add(item.amount).ok_or(ContractError::Overflow)?;
    }

    if sum != total_raised {
        return Err(ContractError::InvalidMilestoneSchedule);
    }

    Ok(())
}

/// Resolves a milestone's vote tally against its frozen basis.
///
/// Returns `Some(Approved)` once yes-weight strictly exceeds half the basis,
/// `Some(Rejected)` once no-weight reaches half the basis, or `None` while
/// the outcome is still undecided. Uses `weight * 2` vs. `basis` (rather than
/// `weight` vs. `basis / 2`) to avoid integer-division truncation on odd
/// totals.
pub fn resolve_vote_status(yes_weight: i128, no_weight: i128, basis: i128) -> Option<MilestoneStatus> {
    if yes_weight.checked_mul(2)? > basis {
        Some(MilestoneStatus::Approved)
    } else if no_weight.checked_mul(2)? >= basis {
        Some(MilestoneStatus::Rejected)
    } else {
        None
    }
}

fn find_milestone_index(milestones: &Vec<Milestone>, milestone_id: u32) -> Option<u32> {
    for i in 0..milestones.len() {
        if milestones.get(i).unwrap().id == milestone_id {
            return Some(i);
        }
    }
    None
}

// ── propose_milestones ────────────────────────────────────────────────────────

pub fn execute_propose_milestones(
    env: &Env,
    creator: Address,
    milestones_input: Vec<MilestoneInput>,
) -> Result<(), ContractError> {
    let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
    if status != Status::Active {
        return Err(ContractError::CampaignNotActive);
    }

    let stored_creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
    if creator != stored_creator {
        return Err(ContractError::Unauthorized);
    }
    creator.require_auth();

    let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
    if env.ledger().timestamp() <= deadline {
        return Err(ContractError::CampaignStillActive);
    }

    let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
    let total_raised: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
    if total_raised < goal {
        return Err(ContractError::GoalNotReached);
    }

    let existing: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));
    if !existing.is_empty() {
        return Err(ContractError::MilestonesAlreadyProposed);
    }

    validate_milestone_schedule(&milestones_input, total_raised)?;

    let voting_deadline = env
        .ledger()
        .timestamp()
        .checked_add(MILESTONE_VOTING_PERIOD_SECS)
        .ok_or(ContractError::Overflow)?;

    let mut milestones: Vec<Milestone> = Vec::new(env);
    let mut id: u32 = 0;
    for item in milestones_input.iter() {
        milestones.push_back(Milestone {
            id,
            description: item.description.clone(),
            amount: item.amount,
            status: MilestoneStatus::Pending,
            yes_weight: 0,
            no_weight: 0,
            voting_deadline,
        });
        id += 1;
    }

    let count = milestones.len();
    env.storage()
        .instance()
        .set(&DataKey::Milestones, &milestones);
    env.storage()
        .instance()
        .set(&DataKey::MilestoneBasis, &total_raised);

    emit_milestones_proposed(env, &creator, count, total_raised);

    Ok(())
}

// ── vote_milestone ────────────────────────────────────────────────────────────

pub fn execute_vote_milestone(
    env: &Env,
    voter: Address,
    milestone_id: u32,
    approve: bool,
) -> Result<(), ContractError> {
    voter.require_auth();

    let mut milestones: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));
    let idx =
        find_milestone_index(&milestones, milestone_id).ok_or(ContractError::MilestoneNotFound)?;
    let mut milestone = milestones.get(idx).unwrap();

    let now = env.ledger().timestamp();
    if milestone.status != MilestoneStatus::Pending || now >= milestone.voting_deadline {
        return Err(ContractError::MilestoneNotPending);
    }

    let vote_key = DataKey::MilestoneVote(MilestoneVoteKey {
        milestone_id,
        voter: voter.clone(),
    });
    if env.storage().persistent().has(&vote_key) {
        return Err(ContractError::AlreadyVoted);
    }

    let weight: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Contribution(voter.clone()))
        .unwrap_or(0);
    if weight == 0 {
        return Err(ContractError::NoContributionWeight);
    }

    env.storage().persistent().set(&vote_key, &true);
    env.storage().persistent().extend_ttl(&vote_key, 100, 100);

    if approve {
        milestone.yes_weight = milestone
            .yes_weight
            .checked_add(weight)
            .ok_or(ContractError::Overflow)?;
    } else {
        milestone.no_weight = milestone
            .no_weight
            .checked_add(weight)
            .ok_or(ContractError::Overflow)?;
    }

    let basis: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MilestoneBasis)
        .unwrap();
    let resolved = resolve_vote_status(milestone.yes_weight, milestone.no_weight, basis);
    if let Some(ref new_status) = resolved {
        milestone.status = new_status.clone();
    }

    milestones.set(idx, milestone.clone());
    env.storage()
        .instance()
        .set(&DataKey::Milestones, &milestones);

    emit_milestone_vote_cast(
        env,
        milestone_id,
        &voter,
        approve,
        weight,
        milestone.yes_weight,
        milestone.no_weight,
    );

    if let Some(new_status) = resolved {
        let approved = new_status == MilestoneStatus::Approved;
        emit_milestone_resolved(env, milestone_id, approved, milestone.yes_weight, milestone.no_weight);
        if !approved {
            maybe_complete_milestone_schedule(env);
        }
    }

    Ok(())
}

// ── finalize_milestone_vote (permissionless backstop) ─────────────────────────

pub fn execute_finalize_milestone_vote(env: &Env, milestone_id: u32) -> Result<(), ContractError> {
    let mut milestones: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));
    let idx =
        find_milestone_index(&milestones, milestone_id).ok_or(ContractError::MilestoneNotFound)?;
    let mut milestone = milestones.get(idx).unwrap();

    if milestone.status != MilestoneStatus::Pending {
        return Err(ContractError::MilestoneNotPending);
    }
    if env.ledger().timestamp() < milestone.voting_deadline {
        return Err(ContractError::MilestoneNotPending);
    }

    let basis: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MilestoneBasis)
        .unwrap();
    // Silence defaults to Rejected, not Approved — consistent with backer protection.
    let new_status =
        resolve_vote_status(milestone.yes_weight, milestone.no_weight, basis).unwrap_or(MilestoneStatus::Rejected);
    milestone.status = new_status.clone();

    milestones.set(idx, milestone.clone());
    env.storage()
        .instance()
        .set(&DataKey::Milestones, &milestones);

    let approved = new_status == MilestoneStatus::Approved;
    emit_milestone_resolved(env, milestone_id, approved, milestone.yes_weight, milestone.no_weight);

    if !approved {
        maybe_complete_milestone_schedule(env);
    }

    Ok(())
}

// ── release_milestone ─────────────────────────────────────────────────────────

pub fn execute_release_milestone(
    env: &Env,
    creator: Address,
    milestone_id: u32,
) -> Result<(), ContractError> {
    let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
    if status != Status::Active {
        return Err(ContractError::CampaignNotActive);
    }

    let stored_creator: Address = env.storage().instance().get(&DataKey::Creator).unwrap();
    if creator != stored_creator {
        return Err(ContractError::Unauthorized);
    }
    creator.require_auth();

    let mut milestones: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));
    let idx =
        find_milestone_index(&milestones, milestone_id).ok_or(ContractError::MilestoneNotFound)?;
    let mut milestone = milestones.get(idx).unwrap();

    if milestone.status != MilestoneStatus::Approved {
        return Err(ContractError::MilestoneNotApproved);
    }

    let amount = milestone.amount;

    // ── Effects (before interaction) ──────────────────────────────────────
    milestone.status = MilestoneStatus::Released;
    milestones.set(idx, milestone);
    env.storage()
        .instance()
        .set(&DataKey::Milestones, &milestones);

    let total_raised: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
    let new_total = total_raised
        .checked_sub(amount)
        .ok_or(ContractError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &new_total);

    // ── Interaction ─────────────────────────────────────────────────────────
    let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let token_client = token::Client::new(env, &token_address);

    let platform_config: Option<PlatformConfig> =
        env.storage().instance().get(&DataKey::PlatformConfig);
    let (creator_payout, platform_fee) = if let Some(config) = platform_config {
        let fee = amount
            .checked_mul(config.fee_bps as i128)
            .expect("fee calculation overflow")
            .checked_div(10_000)
            .expect("fee division by zero");
        token_client.transfer(&env.current_contract_address(), &config.address, &fee);
        emit_fee_transferred(env, &config.address, fee);
        (
            amount.checked_sub(fee).expect("creator payout underflow"),
            fee,
        )
    } else {
        (amount, 0)
    };

    token_client.transfer(&env.current_contract_address(), &creator, &creator_payout);

    emit_milestone_released(env, milestone_id, &creator, creator_payout, platform_fee);

    maybe_complete_milestone_schedule(env);

    Ok(())
}

// ── claim_milestone_refund ────────────────────────────────────────────────────

pub fn execute_claim_milestone_refund(
    env: &Env,
    contributor: Address,
    milestone_id: u32,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let milestones: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));
    let idx =
        find_milestone_index(&milestones, milestone_id).ok_or(ContractError::MilestoneNotFound)?;
    let milestone = milestones.get(idx).unwrap();

    if milestone.status != MilestoneStatus::Rejected {
        return Err(ContractError::MilestoneNotRejected);
    }

    let claim_key = DataKey::MilestoneRefundClaimed(MilestoneRefundKey {
        milestone_id,
        contributor: contributor.clone(),
    });
    if env
        .storage()
        .persistent()
        .get::<_, bool>(&claim_key)
        .unwrap_or(false)
    {
        return Err(ContractError::NothingToRefund);
    }

    let weight: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Contribution(contributor.clone()))
        .unwrap_or(0);
    if weight == 0 {
        return Err(ContractError::NoContributionWeight);
    }

    let basis: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MilestoneBasis)
        .unwrap();
    let share = milestone
        .amount
        .checked_mul(weight)
        .ok_or(ContractError::Overflow)?
        .checked_div(basis)
        .ok_or(ContractError::Overflow)?;

    if share == 0 {
        return Err(ContractError::NothingToRefund);
    }

    // ── Effects (before interaction) ──────────────────────────────────────
    env.storage().persistent().set(&claim_key, &true);
    env.storage().persistent().extend_ttl(&claim_key, 100, 100);

    let total_raised: i128 = env.storage().instance().get(&DataKey::TotalRaised).unwrap();
    let new_total = total_raised
        .checked_sub(share)
        .ok_or(ContractError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &new_total);

    // ── Interaction ─────────────────────────────────────────────────────────
    let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let token_client = token::Client::new(env, &token_address);
    token_client.transfer(&env.current_contract_address(), &contributor, &share);

    emit_milestone_refund_claimed(env, milestone_id, &contributor, share);

    Ok(())
}

// ── Completion helper ─────────────────────────────────────────────────────────

/// Flips the campaign to `Status::Successful` and mints reward NFTs once
/// every milestone has reached a terminal state (`Released` or `Rejected`).
///
/// Deliberately does **not** zero `TotalRaised` here (unlike `withdraw`,
/// `refund`, and `cancel`, which zero it because they pay out everything
/// atomically): a `Rejected` milestone's pro-rata share may still be
/// unclaimed by some backers when the schedule completes, and
/// `execute_claim_milestone_refund` must still be able to decrement
/// `TotalRaised` correctly for those late claims, which can happen after
/// `Status` has already flipped to `Successful`.
pub fn maybe_complete_milestone_schedule(env: &Env) {
    let milestones: Vec<Milestone> = env
        .storage()
        .instance()
        .get(&DataKey::Milestones)
        .unwrap_or_else(|| Vec::new(env));

    if milestones.is_empty() {
        return;
    }

    let mut total_released: i128 = 0;
    let mut total_rejected: i128 = 0;
    let mut all_settled = true;
    for m in milestones.iter() {
        match m.status {
            MilestoneStatus::Released => total_released += m.amount,
            MilestoneStatus::Rejected => total_rejected += m.amount,
            _ => all_settled = false,
        }
    }

    if !all_settled {
        return;
    }

    env.storage()
        .instance()
        .set(&DataKey::Status, &Status::Successful);

    let nft_contract: Option<Address> = env.storage().instance().get(&DataKey::NFTContract);
    mint_nfts_in_batch(env, &nft_contract);

    emit_milestone_schedule_completed(env, total_released, total_rejected);
}

// ── Frontend helpers ──────────────────────────────────────────────────────────

/// Maps a milestone-related `ContractError` repr value to a human-readable
/// message, following the pattern in `crowdfund_initialize_function::describe_init_error`.
#[inline]
pub fn describe_milestone_error(code: u32) -> &'static str {
    match code {
        19 => "A milestone schedule has already been proposed for this campaign",
        20 => {
            "Milestone schedule is invalid: check amounts and descriptions, and confirm they sum exactly to the raised total"
        }
        21 => "No milestone exists with that id",
        22 => "This milestone isn't open for voting (already resolved, or its voting window has closed)",
        23 => "This milestone hasn't been approved yet",
        24 => "You've already voted on this milestone",
        25 => "Only backers with a recorded contribution can vote or claim a refund",
        26 => "This campaign uses milestone-gated release; use the milestone actions instead",
        27 => "This milestone wasn't rejected, so there's no refund to claim",
        _ => "Unknown milestone error",
    }
}

/// Returns `true` if the error reflects a correctable input/timing issue
/// that the caller can retry after adjusting (e.g. re-submitting a schedule,
/// waiting for a voting window, or refreshing milestone state).
#[inline]
pub fn is_milestone_error_retryable(code: u32) -> bool {
    matches!(code, 20 | 22 | 24 | 25 | 27)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod unit_tests {
    use super::*;
    use soroban_sdk::String as SorobanString;

    #[test]
    fn resolve_vote_status_yes_exceeds_half_approves() {
        // basis = 100, yes = 51 -> 51*2=102 > 100
        assert_eq!(
            resolve_vote_status(51, 0, 100),
            Some(MilestoneStatus::Approved)
        );
    }

    #[test]
    fn resolve_vote_status_no_reaches_half_rejects() {
        // basis = 100, no = 50 -> 50*2=100 >= 100
        assert_eq!(
            resolve_vote_status(0, 50, 100),
            Some(MilestoneStatus::Rejected)
        );
    }

    #[test]
    fn resolve_vote_status_tie_stays_pending() {
        // basis = 100, yes = 40, no = 40 -> neither threshold crossed
        assert_eq!(resolve_vote_status(40, 40, 100), None);
    }

    #[test]
    fn resolve_vote_status_exact_half_yes_does_not_approve() {
        // yes must strictly exceed half; exactly half is not enough.
        assert_eq!(resolve_vote_status(50, 0, 100), None);
    }

    #[test]
    fn validate_milestone_schedule_rejects_sum_mismatch() {
        let env = Env::default();
        let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
        milestones.push_back(MilestoneInput {
            description: SorobanString::from_str(&env, "phase 1"),
            amount: 40,
        });
        milestones.push_back(MilestoneInput {
            description: SorobanString::from_str(&env, "phase 2"),
            amount: 40,
        });
        assert_eq!(
            validate_milestone_schedule(&milestones, 100),
            Err(ContractError::InvalidMilestoneSchedule)
        );
    }

    #[test]
    fn validate_milestone_schedule_accepts_exact_match() {
        let env = Env::default();
        let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
        milestones.push_back(MilestoneInput {
            description: SorobanString::from_str(&env, "phase 1"),
            amount: 60,
        });
        milestones.push_back(MilestoneInput {
            description: SorobanString::from_str(&env, "phase 2"),
            amount: 40,
        });
        assert_eq!(validate_milestone_schedule(&milestones, 100), Ok(()));
    }

    #[test]
    fn validate_milestone_schedule_rejects_empty_schedule() {
        let env = Env::default();
        let milestones: Vec<MilestoneInput> = Vec::new(&env);
        assert_eq!(
            validate_milestone_schedule(&milestones, 100),
            Err(ContractError::InvalidMilestoneSchedule)
        );
    }

    #[test]
    fn validate_milestone_schedule_rejects_zero_amount() {
        let env = Env::default();
        let mut milestones: Vec<MilestoneInput> = Vec::new(&env);
        milestones.push_back(MilestoneInput {
            description: SorobanString::from_str(&env, "phase 1"),
            amount: 0,
        });
        assert_eq!(
            validate_milestone_schedule(&milestones, 0),
            Err(ContractError::InvalidMilestoneSchedule)
        );
    }

    #[test]
    fn pro_rata_share_rounds_down_leaves_dust() {
        // amount=10, weight=3, basis=100 -> 30/100 = 0 (rounds down to zero, no dust paid)
        let amount: i128 = 10;
        let weight: i128 = 3;
        let basis: i128 = 100;
        let share = amount.checked_mul(weight).unwrap().checked_div(basis).unwrap();
        assert_eq!(share, 0);
    }
}
