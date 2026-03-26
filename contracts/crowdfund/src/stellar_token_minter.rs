//! Logging bounds for the Stellar token minter / crowdfund contract.
//!
//! Soroban contracts run inside a metered host environment where every event
//! emission and every storage read/write consumes CPU and memory instructions.
//! Unbounded iteration over contributor or pledger lists therefore creates a
//! denial-of-service vector: a campaign with thousands of contributors could
//! make `withdraw` or `collect_pledges` exceed the per-transaction resource
//! limits and become permanently un-callable.
//!
//! This module centralises the bound-checking logic so that:
//! * The limits are defined in one place and easy to audit.
//! * Helper functions can be unit-tested in isolation.
//! * The contract implementation stays readable.
//!
//! # Limits
//!
//! | Constant | Value | Governs |
//! |---|---|---|
//! | [`MAX_EVENTS_PER_TX`] | 100 | Total events emitted in one transaction |
//! | [`MAX_MINT_BATCH`] | 50 | NFT mints per `withdraw` call |
//! | [`MAX_LOG_ENTRIES`] | 200 | Diagnostic log entries per transaction |
//!
//! # Security assumptions
//!
//! * Limits are enforced **before** the loop that would exceed them, not after.
//! * All arithmetic uses `checked_*` to prevent overflow.
//! * No limit can be bypassed by the caller — they are compile-time constants.

use soroban_sdk::Env;

// ── Test constants ────────────────────────────────────────────────────────────
//
// Centralised numeric literals used across the stellar_token_minter test suites.
// Defining them here means CI/CD only needs to update one location when campaign
// parameters change, and test intent is self-documenting.

/// Default campaign funding goal used in tests (1 000 000 stroops).
pub const TEST_GOAL: i128 = 1_000_000;

/// Default minimum contribution used in tests (1 000 stroops).
pub const TEST_MIN_CONTRIBUTION: i128 = 1_000;

/// Default campaign duration used in tests (1 hour in seconds).
pub const TEST_DEADLINE_OFFSET: u64 = 3_600;

/// Initial token balance minted to the creator in the test setup helper.
pub const TEST_CREATOR_BALANCE: i128 = 100_000_000;

/// Initial token balance minted to the token-minter test setup helper.
pub const TEST_MINTER_CREATOR_BALANCE: i128 = 10_000_000;

/// Standard single-contributor balance used in most integration tests.
pub const TEST_CONTRIBUTOR_BALANCE: i128 = 1_000_000;

/// Contribution amount used in NFT-batch tests (goal / MAX_MINT_BATCH).
pub const TEST_NFT_CONTRIBUTION: i128 = 25_000;

/// Contribution amount used in the "below batch limit" NFT test.
pub const TEST_NFT_SMALL_CONTRIBUTION: i128 = 400_000;

/// Contribution amount used in collect_pledges / two-contributor tests.
pub const TEST_PLEDGE_CONTRIBUTION: i128 = 300_000;

/// Bonus goal threshold used in idempotency tests.
pub const TEST_BONUS_GOAL: i128 = 1_000_000;

/// Primary goal used in bonus-goal idempotency tests.
pub const TEST_BONUS_PRIMARY_GOAL: i128 = 500_000;

/// Per-contribution amount used in bonus-goal crossing tests.
pub const TEST_BONUS_CONTRIBUTION: i128 = 600_000;

/// Seed balance for overflow protection test (small initial contribution).
pub const TEST_OVERFLOW_SEED: i128 = 10_000;

/// Maximum platform fee in basis points (100 %).
pub const TEST_FEE_BPS_MAX: u32 = 10_000;

/// Platform fee that exceeds the maximum (triggers panic).
pub const TEST_FEE_BPS_OVER: u32 = 10_001;

/// Platform fee of 10 % used in fee-deduction tests.
pub const TEST_FEE_BPS_10PCT: u32 = 1_000;

/// Progress basis points representing 80 % funding.
pub const TEST_PROGRESS_BPS_80PCT: u32 = 8_000;

/// Progress basis points representing 99.999 % funding (just below goal).
pub const TEST_PROGRESS_BPS_JUST_BELOW: u32 = 9_999;

/// Contribution amount that is one stroop below the goal.
pub const TEST_JUST_BELOW_GOAL: i128 = 999_999;

/// Contribution amount used in the "partial accumulation" test.
pub const TEST_PARTIAL_CONTRIBUTION_A: i128 = 300_000;

/// Second contribution amount used in the "partial accumulation" test.
pub const TEST_PARTIAL_CONTRIBUTION_B: i128 = 200_000;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum number of events that may be emitted in a single transaction.
///
/// Soroban's host enforces its own hard cap; this constant is a conservative
/// application-level guard that keeps us well below that limit.
pub const MAX_EVENTS_PER_TX: u32 = 100;

/// Maximum number of NFT mint calls (and their associated events) that
/// `withdraw` will process in one invocation.
///
/// Mirrors [`crate::MAX_NFT_MINT_BATCH`] and is re-exported here so that
/// tests can import it from a single location.
pub const MAX_MINT_BATCH: u32 = 50;

/// Maximum number of diagnostic log entries per transaction.
///
/// Kept separate from [`MAX_EVENTS_PER_TX`] because diagnostic logs are
/// cheaper but still bounded to prevent runaway output.
pub const MAX_LOG_ENTRIES: u32 = 200;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Returns `true` when `count` is within the per-transaction event budget.
///
/// # Arguments
/// * `count` – Number of events already scheduled for this transaction.
///
/// # Examples
/// ```ignore
/// assert!(within_event_budget(99));
/// assert!(!within_event_budget(100));
/// ```
#[inline]
pub fn within_event_budget(count: u32) -> bool {
    count < MAX_EVENTS_PER_TX
}

/// Returns `true` when `count` is within the NFT mint batch limit.
///
/// # Arguments
/// * `count` – Number of NFTs already minted in this `withdraw` call.
#[inline]
pub fn within_mint_batch(count: u32) -> bool {
    count < MAX_MINT_BATCH
}

/// Returns `true` when `count` is within the diagnostic log entry limit.
///
/// # Arguments
/// * `count` – Number of log entries already written in this transaction.
#[inline]
pub fn within_log_budget(count: u32) -> bool {
    count < MAX_LOG_ENTRIES
}

/// Calculates how many items can still be processed before the event budget
/// is exhausted, given that `reserved` events are already committed.
///
/// Returns `0` when the budget is already exhausted.
///
/// # Arguments
/// * `reserved` – Events already emitted or guaranteed to be emitted.
pub fn remaining_event_budget(reserved: u32) -> u32 {
    MAX_EVENTS_PER_TX.saturating_sub(reserved)
}

/// Calculates how many NFT mints remain in the current batch budget.
///
/// Returns `0` when the batch limit is already reached.
///
/// # Arguments
/// * `minted` – NFTs already minted in this `withdraw` call.
pub fn remaining_mint_budget(minted: u32) -> u32 {
    MAX_MINT_BATCH.saturating_sub(minted)
}

/// Emits a bounded summary event for a batch operation.
///
/// Instead of emitting one event per item (which would be unbounded), callers
/// emit a single summary event carrying the count of processed items.  This
/// function enforces that the summary is only emitted when `count > 0` and
/// that the event budget has not been exhausted.
///
/// # Arguments
/// * `env`      – The Soroban environment.
/// * `topic`    – Two-part event topic `(namespace, name)`.
/// * `count`    – Number of items processed in the batch.
/// * `emitted`  – Events already emitted in this transaction (budget check).
///
/// # Returns
/// `true` if the event was emitted, `false` if skipped (count == 0 or budget
/// exhausted).
pub fn emit_batch_summary(
    env: &Env,
    topic: (&'static str, &'static str),
    count: u32,
    emitted: u32,
) -> bool {
    if count == 0 || !within_event_budget(emitted) {
        return false;
    }
    env.events().publish((topic.0, topic.1), count);
    true
}
