//! # Audit #29 — Equivalence Tests
//!
//! Verifies that the three progress-bps implementations are behaviourally
//! identical across all relevant input classes **before** the inline variants
//! are deleted.  Once the refactor is done, these tests remain as regression
//! guards to ensure the safe helper is never accidentally replaced.
//!
//! ## Three implementations under test
//!
//! | ID   | Location                                         | Description                        |
//! |------|--------------------------------------------------|------------------------------------|
//! | A    | `campaign_goal_minimum::compute_progress_bps`    | Safe helper (checked_mul)          |
//! | B    | `lib.rs::get_stats` (inline, before refactor)    | Raw `*` — no overflow protection   |
//! | C    | `lib.rs::bonus_goal_progress_bps` (inline)       | Raw `*` — no overflow protection   |
//!
//! After the refactor, B and C are replaced by A.  The tests below prove
//! behavioural identity for the normal-value domain that smart-contract
//! inputs can realistically inhabit.
//!
//! ## Overflow note
//!
//! The inline implementations use `total_raised * 10_000` without overflow
//! protection.  For values where `total_raised * 10_000` would overflow i128
//! (i.e. `total_raised > i128::MAX / 10_000 ≈ 1.7 × 10^34`), the inline
//! variants panic in debug builds and produce wrong results in release builds.
//! `compute_progress_bps` saturates to `i128::MAX` via `checked_mul` and
//! then caps at `MAX_PROGRESS_BPS`.  These overflow cases are documented in
//! the overflow section below — they are NOT equivalent, which is precisely
//! why the refactor is needed.

#![cfg(test)]

use crate::campaign_goal_minimum::{
    compute_progress_bps, MAX_PROGRESS_BPS, PROGRESS_BPS_SCALE,
};

// ── Reference implementations (mirrors of the pre-refactor inline code) ──────

/// Mirrors the inline calculation in `get_stats()` before the refactor.
/// Kept here only to document prior behaviour; do not use in production.
fn inline_get_stats_progress(total_raised: i128, goal: i128) -> u32 {
    if goal > 0 {
        let raw = (total_raised * 10_000) / goal;
        if raw > 10_000 {
            10_000
        } else {
            raw as u32
        }
    } else {
        0
    }
}

/// Mirrors the inline calculation in `bonus_goal_progress_bps()` before
/// the refactor.  Kept here only to document prior behaviour.
fn inline_bonus_goal_progress(total_raised: i128, bg: i128) -> u32 {
    if bg > 0 {
        let raw = (total_raised * 10_000) / bg;
        if raw > 10_000 {
            10_000
        } else {
            raw as u32
        }
    } else {
        0
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Asserts that all three implementations return the same value for the
/// given (total_raised, goal) pair.
///
/// Only valid for inputs in the safe domain (no overflow).
fn assert_all_equal(total_raised: i128, goal: i128) {
    let safe   = compute_progress_bps(total_raised, goal);
    let stats  = inline_get_stats_progress(total_raised, goal);
    let bonus  = inline_bonus_goal_progress(total_raised, goal);
    assert_eq!(
        safe, stats,
        "compute_progress_bps != inline_get_stats for ({total_raised}, {goal}): {safe} vs {stats}"
    );
    assert_eq!(
        safe, bonus,
        "compute_progress_bps != inline_bonus_goal for ({total_raised}, {goal}): {safe} vs {bonus}"
    );
}

// ── Zero / edge inputs ────────────────────────────────────────────────────────

#[test]
fn equivalence_zero_raised_positive_goal() {
    // 0 % — all three must return 0.
    assert_all_equal(0, 1_000_000);
}

#[test]
fn equivalence_zero_goal_returns_zero() {
    // Division-by-zero guard — compute_progress_bps returns 0; inline also
    // returns 0 because of the `goal > 0` / `bg > 0` guard.
    assert_eq!(compute_progress_bps(1_000, 0), 0);
    assert_eq!(inline_get_stats_progress(1_000, 0), 0);
    assert_eq!(inline_bonus_goal_progress(1_000, 0), 0);
}

#[test]
fn equivalence_negative_goal_returns_zero() {
    // All three return 0 for a negative goal (different guard paths but
    // same output).
    assert_eq!(compute_progress_bps(1_000, -1), 0);
    // inline_get_stats uses `goal > 0` so negative → 0
    assert_eq!(inline_get_stats_progress(1_000, -1), 0);
    assert_eq!(inline_bonus_goal_progress(1_000, -1), 0);
}

// ── Normal-value equivalence ──────────────────────────────────────────────────

#[test]
fn equivalence_quarter_goal() {
    // 25 % = 2 500 bps
    assert_all_equal(250_000, 1_000_000);
}

#[test]
fn equivalence_half_goal() {
    // 50 % = 5 000 bps
    assert_all_equal(500_000, 1_000_000);
}

#[test]
fn equivalence_exact_goal() {
    // 100 % = 10 000 bps
    assert_all_equal(1_000_000, 1_000_000);
}

#[test]
fn equivalence_99_percent() {
    // 99 % = 9 900 bps
    assert_all_equal(9_900, 10_000);
}

#[test]
fn equivalence_1_bps() {
    // 0.01 % = 1 bps  (smallest distinguishable progress)
    assert_all_equal(1, 10_000);
}

#[test]
fn equivalence_minimum_goal_minimum_raised() {
    // 1 / 1 = 100 % capped at MAX_PROGRESS_BPS
    assert_all_equal(1, 1);
}

// ── Capping equivalence ───────────────────────────────────────────────────────

#[test]
fn equivalence_two_x_goal_capped_at_max() {
    // 200 % must be capped at MAX_PROGRESS_BPS, not returned as 20 000.
    let safe   = compute_progress_bps(2_000_000, 1_000_000);
    let stats  = inline_get_stats_progress(2_000_000, 1_000_000);
    let bonus  = inline_bonus_goal_progress(2_000_000, 1_000_000);
    assert_eq!(safe,  MAX_PROGRESS_BPS);
    assert_eq!(stats, MAX_PROGRESS_BPS);
    assert_eq!(bonus, MAX_PROGRESS_BPS);
}

#[test]
fn equivalence_ten_x_goal_capped() {
    assert_all_equal(10_000_000, 1_000_000);
}

#[test]
fn equivalence_large_realistic_values() {
    // Realistic token amounts (e.g. 10^7 raised of a 10^7 goal).
    assert_all_equal(10_000_000, 10_000_000);
    assert_all_equal(7_500_000, 10_000_000);
    assert_all_equal(1, 100_000_000);
}

#[test]
fn equivalence_rounding_truncation() {
    // Integer division truncates — all three should produce the same
    // truncated result.
    // 3 / 10_000 = 0 bps (rounds down)
    assert_all_equal(3, 10_000);
    // 9_999 / 10_000 = 9_999 bps (not rounded up to 10_000)
    assert_all_equal(9_999, 10_000);
}

// ── Negative total_raised divergence (documented, not asserted equal) ─────────
//
// NOTE: For negative `total_raised` the inline implementations diverge from
// `compute_progress_bps`:
//   - `compute_progress_bps` returns 0  (total_raised <= 0 guard)
//   - inline variants do NOT guard on total_raised and would return a
//     negative `raw`, then cast to u32, yielding a large garbage value.
//
// This is the *primary* safety bug fixed by the refactor.  We document the
// divergence here rather than asserting equality (they are NOT equal).

#[test]
fn safe_helper_returns_zero_for_negative_total_raised() {
    // The safe helper guards against negative total_raised.
    assert_eq!(compute_progress_bps(-1, 1_000_000), 0);
    assert_eq!(compute_progress_bps(i128::MIN, 1_000_000), 0);
}

#[test]
fn inline_stats_does_not_guard_negative_total_raised() {
    // The inline get_stats variant lacks a total_raised guard.
    // For total_raised large enough in magnitude that the scaled product
    // (-total_raised * 10_000) exceeds the goal, the raw i128 result is
    // negative, and `raw as u32` wraps to a large garbage value — unlike
    // compute_progress_bps which returns 0.
    //
    // Use -1_000_000 (> goal of 1) so raw = -10^10 / 1 = -10^10 != 0.
    let safe_result = compute_progress_bps(-1_000_000, 1);
    let inline_result = inline_get_stats_progress(-1_000_000, 1);
    assert_eq!(safe_result, 0, "safe helper must return 0 for negative total_raised");
    // The inline variant will cast a large negative i128 to u32 (wrapping),
    // producing a non-zero garbage value — demonstrating the safety bug.
    assert_ne!(
        inline_result, 0,
        "inline_get_stats should produce garbage (non-zero) for large negative total_raised"
    );
}

#[test]
fn inline_bonus_does_not_guard_negative_total_raised() {
    let safe_result = compute_progress_bps(-1_000_000, 1);
    let inline_result = inline_bonus_goal_progress(-1_000_000, 1);
    assert_eq!(safe_result, 0, "safe helper must return 0 for negative total_raised");
    assert_ne!(
        inline_result, 0,
        "inline_bonus should produce garbage (non-zero) for large negative total_raised"
    );
}

// ── After-refactor regression: compute_progress_bps must be called ───────────
//
// These tests double-check that the safe helper alone exhibits the correct
// post-refactor behaviour so that replacing the inline code with
// `compute_progress_bps` preserves the expected outputs.

#[test]
fn post_refactor_zero_raised_zero_result() {
    assert_eq!(compute_progress_bps(0, 1_000_000), 0);
}

#[test]
fn post_refactor_half_goal_five_thousand_bps() {
    assert_eq!(compute_progress_bps(500_000, 1_000_000), 5_000);
}

#[test]
fn post_refactor_exact_goal_max_bps() {
    assert_eq!(compute_progress_bps(1_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn post_refactor_over_goal_capped() {
    assert_eq!(compute_progress_bps(2_000_000, 1_000_000), MAX_PROGRESS_BPS);
}

#[test]
fn post_refactor_zero_goal_no_panic() {
    // Must not panic — should return 0.
    assert_eq!(compute_progress_bps(1_000, 0), 0);
}

#[test]
fn post_refactor_overflow_safe() {
    // Very large total_raised: checked_mul saturates, result capped at MAX.
    assert_eq!(compute_progress_bps(i128::MAX, 1), MAX_PROGRESS_BPS);
}

#[test]
fn post_refactor_progress_scale_matches_max() {
    // PROGRESS_BPS_SCALE and MAX_PROGRESS_BPS are semantically the same value.
    assert_eq!(PROGRESS_BPS_SCALE as u32, MAX_PROGRESS_BPS);
}
