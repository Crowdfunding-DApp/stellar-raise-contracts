//! Shared constants and helpers for Stellar token minter tests.
//!
//! This module keeps test fixtures consistent so security-sensitive tests read
//! clearly and are easier to review.

/// Default campaign goal used in token minter tests.
pub const TEST_GOAL: i128 = 1_000_000;

/// Default minimum contribution used in token minter tests.
pub const TEST_MIN_CONTRIBUTION: i128 = 1_000;

/// Default funding amount used to mint test balances.
pub const TEST_MINT_AMOUNT: i128 = 10_000_000;

/// Returns a deadline offset used by token minter tests.
pub const fn deadline_offset_seconds() -> u64 {
    3_600
}
