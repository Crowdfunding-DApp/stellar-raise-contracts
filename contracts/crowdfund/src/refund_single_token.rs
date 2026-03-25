//! # `refund_single` Token Transfer Logic
//!
//! @title   RefundSingleToken — Pull-based contributor refund module.
//!
//! @notice  Centralises all logic for executing a single contributor refund.
//!          Exposes four public items:
//!
//!          | Symbol                          | Purpose                                              |
//!          |---------------------------------|------------------------------------------------------|
//!          | `refund_single_transfer`        | Direction-locked token transfer (contract→contributor)|
//!          | `validate_refund_preconditions` | Pure guard — checks all preconditions, returns amount |
//!          | `execute_refund_single`         | Atomic CEI execution — zero storage, then transfer    |
//!          | `get_contribution`              | Read-only helper for contribution balance             |
//!
//! ## Security Assumptions
//!
//! 1. **Authentication** — `contributor.require_auth()` must be called by the
//!    caller (`lib.rs`) before invoking any function in this module.
//! 2. **CEI order** — `execute_refund_single` zeroes storage *before* the token
//!    transfer, preventing re-entrancy double-claims.
//! 3. **Overflow protection** — `total_raised` is decremented with `checked_sub`;
//!    returns `ContractError::Overflow` rather than wrapping silently.
//! 4. **Direction lock** — `refund_single_transfer` always transfers
//!    `contract → contributor`; direction cannot be reversed by a caller.
//! 5. **Single transfer** — exactly one `token.transfer` call per refund;
//!    the previous bug called it twice (once via `refund_single_transfer` and
//!    once directly), which has been removed.

#![allow(missing_docs)]

use soroban_sdk::{token, Address, Env};

use crate::{ContractError, DataKey, Status};

// ── Storage helper ────────────────────────────────────────────────────────────

/// Read the stored contribution amount for `contributor` (0 if absent).
///
/// @param env         Soroban environment.
/// @param contributor The address to query.
/// @return Contribution amount, or 0 if no record exists.
pub fn get_contribution(env: &Env, contributor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Contribution(contributor.clone()))
        .unwrap_or(0)
}

// ── Low-level refund helper ───────────────────────────────────────────────────

/// Low-level helper: zero the contribution record then transfer tokens back.
///
/// @notice Does **not** check campaign status or auth — callers are responsible.
/// @notice CEI order: storage zeroed **before** token transfer.
///
/// @param env           Soroban environment.
/// @param token_address The token contract address stored at initialization.
/// @param contributor   The address to refund.
/// @return Amount transferred (0 if contributor had no balance).
pub fn refund_single(env: &Env, token_address: &Address, contributor: &Address) -> i128 {
    let amount = get_contribution(env, contributor);
    if amount > 0 {
        // Effect: zero storage before interaction
        env.storage()
            .persistent()
            .set(&DataKey::Contribution(contributor.clone()), &0i128);
        // Interaction: single transfer call
        let token_client = token::Client::new(env, token_address);
        refund_single_transfer(&token_client, &env.current_contract_address(), contributor, amount);
    }
    amount
}

// ── Transfer primitive ────────────────────────────────────────────────────────

/// Transfer `amount` tokens from the contract to `contributor`.
///
/// @notice Direction is fixed: contract → contributor.
/// @dev    Single call site prevents parameter-order typos.
///
/// @param token_client      Pre-built token client.
/// @param contract_address  The crowdfund contract's own address (sender).
/// @param contributor       Recipient of the refund.
/// @param amount            Token amount to transfer (must be > 0).
pub fn refund_single_transfer(
    token_client: &token::Client,
    contract_address: &Address,
    contributor: &Address,
    amount: i128,
) {
    token_client.transfer(contract_address, contributor, &amount);
}

// ── Precondition guard ────────────────────────────────────────────────────────

/// Validate all preconditions for a `refund_single` call.
///
/// @notice Pure read — does **not** mutate any state. Safe to call speculatively.
///
/// @param env         Soroban environment.
/// @param contributor The address requesting a refund.
/// @return `Ok(amount)` when the refund is valid, `Err(ContractError)` otherwise.
///
/// # Errors
/// * `ContractError::NothingToRefund` — contributor has no balance on record.
///
/// # Panics
/// * `"campaign must be in Expired state to refund"` when status is not `Expired`.
pub fn validate_refund_preconditions(
    env: &Env,
    contributor: &Address,
) -> Result<i128, ContractError> {
    let status: Status = env.storage().instance().get(&DataKey::Status).unwrap();
    if status != Status::Expired {
        panic!("campaign must be in Expired state to refund");
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

// ── Atomic CEI execution ──────────────────────────────────────────────────────

/// Execute a single contributor refund using the CEI pattern.
///
/// @notice Caller **must** have already called `contributor.require_auth()` and
///         `validate_refund_preconditions` before invoking this function.
///
/// @notice CEI order:
///         1. Effect:      zero contribution record in persistent storage.
///         2. Effect:      decrement `total_raised` in instance storage.
///         3. Interaction: single `token.transfer` call (contract → contributor).
///         4. Event:       emit `("campaign", "refund_single")`.
///
/// @param env         Soroban environment.
/// @param contributor The address to refund.
/// @param amount      The amount returned by `validate_refund_preconditions`.
/// @return `Ok(())` on success, `Err(ContractError::Overflow)` on underflow.
pub fn execute_refund_single(
    env: &Env,
    contributor: &Address,
    amount: i128,
) -> Result<(), ContractError> {
    let contribution_key = DataKey::Contribution(contributor.clone());

    // ── Effects (zero storage BEFORE transfer) ────────────────────────────
    env.storage().persistent().set(&contribution_key, &0i128);
    env.storage()
        .persistent()
        .extend_ttl(&contribution_key, 100, 100);

    let total: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalRaised)
        .unwrap_or(0);
    let new_total = total.checked_sub(amount).ok_or(ContractError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalRaised, &new_total);

    // ── Interaction (single transfer after state is settled) ──────────────
    let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let token_client = token::Client::new(env, &token_address);
    refund_single_transfer(
        &token_client,
        &env.current_contract_address(),
        contributor,
        amount,
    );

    // ── Event ─────────────────────────────────────────────────────────────
    env.events()
        .publish(("campaign", "refund_single"), (contributor.clone(), amount));

    Ok(())
}
