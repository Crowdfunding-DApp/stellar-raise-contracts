//! contribute() error handling — constants, helpers, and off-chain utilities.
//!
//! # Error taxonomy for `contribute()`
//!
//! | Code | Variant          | Trigger                                          |
//! |------|------------------|--------------------------------------------------|
//! |  2   | `CampaignEnded`  | `ledger.timestamp > deadline`                    |
//! |  6   | `Overflow`       | `checked_add` would wrap on contribution totals  |
//! |  9   | `AmountTooLow`   | `amount < min_contribution`                      |
//! | 10   | `ZeroAmount`     | `amount == 0`                                    |
//!
//! # Security assumptions
//!
//! - `contributor.require_auth()` is called before any state mutation.
//! - Token transfer happens before storage writes; if the transfer fails the
//!   transaction rolls back atomically — no partial state is persisted.
//! - Overflow is caught with `checked_add` on both the per-contributor total
//!   and `total_raised`, returning `ContractError::Overflow` rather than
//!   wrapping silently.
//! - The deadline check uses strict `>`, so a contribution at exactly the
//!   deadline timestamp is accepted — scripts should account for this boundary.

/// Numeric error codes returned by the contract host for `contribute()`.
///
/// These mirror the `#[repr(u32)]` values of `ContractError` and are intended
/// for use in off-chain scripts that inspect raw error codes.
pub mod error_codes {
    /// `contribute()` was called after the campaign deadline.
    pub const CAMPAIGN_ENDED: u32 = 2;
    /// A checked arithmetic operation overflowed.
    pub const OVERFLOW: u32 = 6;
    /// The contribution amount is below the campaign minimum.
    pub const AMOUNT_TOO_LOW: u32 = 9;
    /// The contribution amount is zero.
    pub const ZERO_AMOUNT: u32 = 10;
}

/// Returns a human-readable description for a `contribute()` error code.
///
/// # Example
/// ```
/// use contribute_error_handling::{describe_error, error_codes};
/// assert_eq!(describe_error(error_codes::CAMPAIGN_ENDED), "Campaign has ended");
/// assert_eq!(describe_error(error_codes::AMOUNT_TOO_LOW), "Amount is below the campaign minimum");
/// ```
pub fn describe_error(code: u32) -> &'static str {
    match code {
        error_codes::CAMPAIGN_ENDED => "Campaign has ended",
        error_codes::OVERFLOW => "Arithmetic overflow — contribution amount too large",
        error_codes::AMOUNT_TOO_LOW => "Amount is below the campaign minimum",
        error_codes::ZERO_AMOUNT => "Contribution amount must be greater than zero",
        _ => "Unknown error",
    }
}

/// Returns `true` if the error code is retryable by the caller.
///
/// - `AmountTooLow` and `ZeroAmount` are retryable — the caller can submit a
///   higher amount in a new transaction.
/// - `CampaignEnded` and `Overflow` are permanent for the current campaign
///   state and cannot be resolved by retrying the same call.
pub fn is_retryable(code: u32) -> bool {
    matches!(code, error_codes::AMOUNT_TOO_LOW | error_codes::ZERO_AMOUNT)
}
