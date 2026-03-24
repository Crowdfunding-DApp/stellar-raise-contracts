//! # refund_single_token
//!
//! @title   Shared Stellar token transfer helpers for contributor refunds
//!
//! @notice  Centralizes the **contract → contributor** transfer direction for
//!          every refund code path (`CrowdfundContract::refund_single`, batch
//!          `refund`, and the module-level [`refund_single`] helper used in
//!          tests).  A single function name and argument order reduces audit
//!          surface and prevents accidental reversal of `from` / `to`.
//!
//! @dev     The Soroban `token::Client::transfer` API is `transfer(from, to, amount)`.
//!          All refunds **must** use `from = crowdfund contract` and `to = contributor`.
//!
//! ## Security assumptions
//!
//! 1. **`refund_single_transfer`** — The only sanctioned way to move tokens out
//!    of the crowdfund contract for refunds; call sites pass the contract
//!    address explicitly as `from`.
//! 2. **`CrowdfundContract::refund_single`** (in `lib.rs`) — Performs
//!    checks-effects-interactions: updates storage after transfer via the same
//!    helper pattern (see `lib.rs` for ordering).
//! 3. **Module [`refund_single`]** — Legacy/test helper that performs transfer
//!    then storage zeroing; it now delegates the transfer to
//!    [`refund_single_transfer`] so bytecode and tests stay aligned with
//!    production paths.
//! 4. **Re-entrancy** — Not applicable on Soroban; still, storage is cleared
//!    after a successful transfer in `lib.rs`’s `refund_single`.

use soroban_sdk::{token, Address, Env};

use crate::DataKey;

/// Performs the canonical refund token transfer: **crowdfund contract → contributor**.
///
/// @notice Sends `amount` of the campaign asset from `contract_address` to `contributor`.
/// @param  token_client       Soroban token client for the campaign asset.
/// @param  contract_address   Address of the crowdfund contract (transfer `from`).
/// @param  contributor        Recipient of the refund (transfer `to`).
/// @param  amount             Amount in the token’s smallest unit.
///
/// @dev    Parameter names are explicit to avoid swapping `from` and `to` at
///         call sites (a common source of critical bugs in financial contracts).
#[inline]
pub fn refund_single_transfer(
    token_client: &token::Client,
    contract_address: &Address,
    contributor: &Address,
    amount: i128,
) {
    token_client.transfer(contract_address, contributor, &amount);
}

/// Refunds a single contributor by transferring their stored contribution
/// amount from the contract to their address, then clearing persistent storage.
///
/// @notice Intended for **tests** and low-level tooling; production UI should
///         call `CrowdfundContract::refund_single` which enforces auth and
///         campaign state.
///
/// @param  env            Execution environment (must be the crowdfund contract context).
/// @param  token_address  Campaign token contract.
/// @param  contributor    Address whose `Contribution` entry is refunded.
/// @return Amount transferred, or `0` if nothing was owed.
///
/// @dev    Uses [`refund_single_transfer`] for the actual token movement so the
///         transfer path matches `lib.rs` and `refund()`.
pub fn refund_single(env: &Env, token_address: &Address, contributor: &Address) -> i128 {
    let contribution_key = DataKey::Contribution(contributor.clone());
    let amount: i128 = env
        .storage()
        .persistent()
        .get(&contribution_key)
        .unwrap_or(0);

    if amount == 0 {
        return 0;
    }

    let token_client = token::Client::new(env, token_address);
    refund_single_transfer(
        &token_client,
        &env.current_contract_address(),
        contributor,
        amount,
    );

    env.storage().persistent().set(&contribution_key, &0i128);
    env.storage()
        .persistent()
        .extend_ttl(&contribution_key, 100, 100);

    env.events()
        .publish(("campaign", "refund_single"), (contributor.clone(), amount));

    amount
}

/// Returns the stored contribution balance for `contributor` without transferring.
///
/// @param  env           Execution environment (crowdfund contract context).
/// @param  contributor   Contributor address.
/// @return Stored amount, or `0` if missing.
#[inline]
pub fn get_contribution(env: &Env, contributor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Contribution(contributor.clone()))
        .unwrap_or(0)
}
