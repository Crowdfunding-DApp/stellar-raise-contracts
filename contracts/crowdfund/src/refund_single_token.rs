//! Pull-based refund helpers for a single contributor claim.
//!
//! This module keeps the public `refund_single()` entrypoint in `lib.rs` small
//! and pushes the safety-sensitive logic into focused helpers:
//!
//! - `validate_refund_preconditions()` performs all read-only checks and returns
//!   the exact amount owed to the contributor.
//! - `execute_refund_single()` applies the refund using a CEI-style flow:
//!   pre-compute arithmetic, persist effects, then call the token contract.
//! - `refund_single_transfer()` locks the token transfer direction to
//!   `contract -> contributor`.
//!
//! ## Validated security invariants
//!
//! - The public entrypoint must authenticate `contributor` before calling these
//!   helpers.
//! - Refund eligibility is rejected while the campaign is still active or after
//!   the goal has been met.
//! - Arithmetic is checked before any storage mutation so an inconsistent
//!   `total_raised` value cannot strand a contributor or drive accounting
//!   negative.
//! - The contributor record is zeroed before the external token transfer.
//! - Only the targeted contributor record and `total_raised` are mutated.

use soroban_sdk::{token, Address, Env};

use crate::{ContractError, DataKey, Status};

/// @notice Transfer refund tokens from the contract balance to the contributor.
/// @dev Locks the transfer direction to `contract -> contributor` so call sites
///      cannot accidentally reverse sender and recipient.
/// @param token_client Pre-built client for the configured token contract.
/// @param contract_address The crowdfund contract address that currently holds funds.
/// @param contributor The contributor receiving the refund.
/// @param amount The validated refund amount to transfer.
pub fn refund_single_transfer(
    token_client: &token::Client,
    contract_address: &Address,
    contributor: &Address,
    amount: i128,
) {
    token_client.transfer(contract_address, contributor, &amount);
}

/// @notice Validate every read-only precondition for a single refund claim.
/// @dev Returns the stored contribution amount so the caller can pass it
///      directly into `execute_refund_single()` without re-reading storage.
/// @param env The current Soroban environment.
/// @param contributor The contributor requesting the refund.
/// @return `Ok(amount)` when the refund is allowed, otherwise the precise
///         `ContractError` explaining why it is blocked.
/// @security This helper performs no writes and is safe to call speculatively.
/// @security The public entrypoint must still enforce `contributor.require_auth()`.
pub fn validate_refund_preconditions(
    env: &Env,
    contributor: &Address,
) -> Result<i128, ContractError> {
    let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
    if status == Status::Successful || status == Status::Cancelled {
        panic!("campaign is not active");
    }

    let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
    if env.ledger().timestamp() <= deadline {
        return Err(ContractError::CampaignStillActive);
    }

    let goal: i128 = env.storage().instance().get(&DataKey::Goal).unwrap();
    let total: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalRaised)
        .unwrap_or(0);
    if total >= goal {
        return Err(ContractError::GoalReached);
    }

    let amount: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Contribution(contributor.clone()))
        .unwrap_or(0);
    if amount == 0 {
        return Err(ContractError::NothingToRefund);
    }

    Ok(amount)
}

/// @notice Execute a validated single-contributor refund.
/// @dev Applies a CEI-style flow: first validate arithmetic, then persist local
///      effects, and only then call the token contract.
/// @param env The current Soroban environment.
/// @param contributor The contributor to refund.
/// @param amount The amount returned by `validate_refund_preconditions()`.
/// @return `Ok(())` when the refund completes, or `Err(ContractError::Overflow)`
///         if `total_raised` is inconsistent with the refund amount.
/// @security The caller must authenticate the contributor before invoking this helper.
/// @security Arithmetic is checked before storage mutation so an inconsistent
///           total cannot zero contributor state or push accounting negative.
pub fn execute_refund_single(
    env: &Env,
    contributor: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    let total: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalRaised)
        .unwrap_or(0);
    if amount > total {
        return Err(ContractError::Overflow);
    }
    let new_total = total.checked_sub(amount).ok_or(ContractError::Overflow)?;

    let contribution_key = DataKey::Contribution(contributor.clone());

    // Effects: persist state before the external token transfer.
    env.storage().persistent().set(&contribution_key, &0i128);
    env.storage()
        .persistent()
        .extend_ttl(&contribution_key, 100, 100);
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &new_total);

    // Interaction: transfer only after local state is settled.
    let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let token_client = token::Client::new(env, &token_address);
    refund_single_transfer(
        &token_client,
        &env.current_contract_address(),
        contributor,
        amount,
    );

    env.events()
        .publish(("campaign", "refund_single"), (contributor.clone(), amount));

    Ok(())
}
