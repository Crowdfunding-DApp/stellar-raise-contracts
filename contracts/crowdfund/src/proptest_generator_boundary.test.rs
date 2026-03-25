//! Comprehensive property-based and edge-case tests for proptest generator
//! boundary conditions.
//!
//! # Coverage goals
//!
//! - Every exported constant is verified at its exact value.
//! - Every validator is exercised across valid, boundary, and invalid inputs.
//! - Every clamping helper is verified to never panic and to stay in range.
//! - Derived helpers (`compute_progress_bps`, `compute_fee_amount`) are
//!   verified for correctness and overflow safety.
//!
//! # NatSpec-style module notice
//!
//! @notice These tests are the authoritative regression suite for boundary
//!         constants. Any change to a constant must be reflected here.
//! @dev    Property tests run 256 cases each. Edge-case tests cover exact
//!         boundary values and known regression seeds.

use proptest::prelude::*;
use proptest::strategy::Just;

use crate::proptest_generator_boundary::{
    boundary_log_tag, clamp_progress_bps, clamp_proptest_cases, compute_fee_amount,
    compute_progress_bps, is_valid_contribution_amount, is_valid_deadline_offset,
    is_valid_fee_bps, is_valid_generator_batch_size, is_valid_goal, is_valid_min_contribution,
    DEADLINE_OFFSET_MAX, DEADLINE_OFFSET_MIN, FEE_BPS_CAP, GENERATOR_BATCH_MAX, GOAL_MAX,
    GOAL_MIN, MIN_CONTRIBUTION_FLOOR, PROGRESS_BPS_CAP, PROPTEST_CASES_MAX, PROPTEST_CASES_MIN,
};

// ── Reusable strategies ───────────────────────────────────────────────────────

fn valid_deadline_offset_strategy() -> impl Strategy<Value = u64> {
    DEADLINE_OFFSET_MIN..=DEADLINE_OFFSET_MAX
}

fn valid_goal_strategy() -> impl Strategy<Value = i128> {
    GOAL_MIN..=GOAL_MAX
}

fn valid_fee_bps_strategy() -> impl Strategy<Value = u32> {
    0u32..=FEE_BPS_CAP
}

fn valid_batch_size_strategy() -> impl Strategy<Value = u32> {
    1u32..=GENERATOR_BATCH_MAX
}

fn valid_proptest_cases_strategy() -> impl Strategy<Value = u32> {
    PROPTEST_CASES_MIN..=PROPTEST_CASES_MAX
}

// ── Property tests ────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    // ── Deadline offset ───────────────────────────────────────────────────────

    /// @notice Valid deadline offsets are always accepted.
    #[test]
    fn prop_valid_deadline_offset_accepted(offset in valid_deadline_offset_strategy()) {
        prop_assert!(is_valid_deadline_offset(offset));
    }

    /// @notice Offsets below DEADLINE_OFFSET_MIN are always rejected.
    #[test]
    fn prop_deadline_offset_below_min_rejected(offset in 0u64..DEADLINE_OFFSET_MIN) {
        prop_assert!(!is_valid_deadline_offset(offset));
    }

    /// @notice Offsets above DEADLINE_OFFSET_MAX are always rejected.
    #[test]
    fn prop_deadline_offset_above_max_rejected(
        offset in (DEADLINE_OFFSET_MAX + 1)..=(DEADLINE_OFFSET_MAX + 100_000),
    ) {
        prop_assert!(!is_valid_deadline_offset(offset));
    }

    // ── Goal ──────────────────────────────────────────────────────────────────

    /// @notice Goals inside [GOAL_MIN, GOAL_MAX] are always accepted.
    #[test]
    fn prop_valid_goal_accepted(goal in valid_goal_strategy()) {
        prop_assert!(is_valid_goal(goal));
    }

    /// @notice Goals below GOAL_MIN are always rejected.
    #[test]
    fn prop_goal_below_min_rejected(goal in (-1_000_000i128..GOAL_MIN)) {
        prop_assert!(!is_valid_goal(goal));
    }

    /// @notice Goals above GOAL_MAX are always rejected.
    #[test]
    fn prop_goal_above_max_rejected(goal in (GOAL_MAX + 1)..=(GOAL_MAX + 1_000_000)) {
        prop_assert!(!is_valid_goal(goal));
    }

    // ── Min contribution ──────────────────────────────────────────────────────

    /// @notice Min contribution in [MIN_CONTRIBUTION_FLOOR, goal] is always valid.
    #[test]
    fn prop_min_contribution_valid_for_goal(
        (goal, min) in valid_goal_strategy()
            .prop_flat_map(|g| (Just(g), MIN_CONTRIBUTION_FLOOR..=g)),
    ) {
        prop_assert!(is_valid_min_contribution(min, goal));
    }

    /// @notice Min contribution above goal is always invalid.
    #[test]
    fn prop_min_contribution_above_goal_invalid(
        (goal, excess) in valid_goal_strategy()
            .prop_flat_map(|g| (Just(g), 1i128..=1_000i128)),
    ) {
        prop_assert!(!is_valid_min_contribution(goal + excess, goal));
    }

    // ── Contribution amount ───────────────────────────────────────────────────

    /// @notice Contributions >= min_contribution are always valid.
    #[test]
    fn prop_contribution_at_or_above_min_valid(
        (min_contribution, amount) in (MIN_CONTRIBUTION_FLOOR..=1_000_000i128)
            .prop_flat_map(|m| (Just(m), m..=(m + 10_000_000))),
    ) {
        prop_assert!(is_valid_contribution_amount(amount, min_contribution));
    }

    /// @notice Contributions below min_contribution are always invalid.
    #[test]
    fn prop_contribution_below_min_invalid(
        (min_contribution, shortfall) in (2i128..=1_000_000i128)
            .prop_flat_map(|m| (Just(m), 1i128..m)),
    ) {
        prop_assert!(!is_valid_contribution_amount(min_contribution - shortfall, min_contribution));
    }

    // ── Fee bps ───────────────────────────────────────────────────────────────

    /// @notice Fee bps in [0, FEE_BPS_CAP] is always valid.
    #[test]
    fn prop_valid_fee_bps_accepted(fee_bps in valid_fee_bps_strategy()) {
        prop_assert!(is_valid_fee_bps(fee_bps));
    }

    /// @notice Fee bps above FEE_BPS_CAP is always invalid.
    #[test]
    fn prop_fee_bps_above_cap_rejected(excess in 1u32..=100_000u32) {
        prop_assert!(!is_valid_fee_bps(FEE_BPS_CAP + excess));
    }

    // ── clamp_progress_bps ────────────────────────────────────────────────────

    /// @notice Clamped progress bps never exceeds PROGRESS_BPS_CAP.
    #[test]
    fn prop_clamp_progress_bps_never_exceeds_cap(raw in -100_000i128..=100_000i128) {
        prop_assert!(clamp_progress_bps(raw) <= PROGRESS_BPS_CAP);
    }

    /// @notice Negative inputs always clamp to 0.
    #[test]
    fn prop_clamp_progress_bps_non_negative(raw in i128::MIN..=0i128) {
        prop_assert_eq!(clamp_progress_bps(raw), 0u32);
    }

    /// @notice clamp_progress_bps is idempotent for values already in range.
    #[test]
    fn prop_clamp_progress_bps_idempotent_in_range(raw in 0i128..=10_000i128) {
        let first = clamp_progress_bps(raw);
        let second = clamp_progress_bps(first as i128);
        prop_assert_eq!(first, second);
    }

    /// @notice clamp_progress_bps does not panic for any i128 input.
    #[test]
    fn prop_clamp_progress_bps_no_panic(raw in any::<i128>()) {
        let _ = clamp_progress_bps(raw);
    }

    // ── clamp_proptest_cases ──────────────────────────────────────────────────

    /// @notice Case clamp always stays within [PROPTEST_CASES_MIN, PROPTEST_CASES_MAX].
    #[test]
    fn prop_clamp_proptest_cases_is_bounded(requested in any::<u32>()) {
        let cases = clamp_proptest_cases(requested);
        prop_assert!(cases >= PROPTEST_CASES_MIN);
        prop_assert!(cases <= PROPTEST_CASES_MAX);
    }

    /// @notice Case clamp is identity for values already in range.
    #[test]
    fn prop_clamp_proptest_cases_identity_in_range(requested in valid_proptest_cases_strategy()) {
        prop_assert_eq!(clamp_proptest_cases(requested), requested);
    }

    // ── is_valid_generator_batch_size ─────────────────────────────────────────

    /// @notice Batch sizes in [1, GENERATOR_BATCH_MAX] are always valid.
    #[test]
    fn prop_valid_batch_size_accepted(size in valid_batch_size_strategy()) {
        prop_assert!(is_valid_generator_batch_size(size));
    }

    /// @notice Batch sizes above GENERATOR_BATCH_MAX are always invalid.
    #[test]
    fn prop_batch_size_above_max_rejected(excess in 1u32..=1_000u32) {
        prop_assert!(!is_valid_generator_batch_size(GENERATOR_BATCH_MAX + excess));
    }

    /// @notice Batch size validator matches expected range predicate.
    #[test]
    fn prop_generator_batch_size_matches_range(size in 0u32..=1_024u32) {
        let expected = size >= 1 && size <= GENERATOR_BATCH_MAX;
        prop_assert_eq!(is_valid_generator_batch_size(size), expected);
    }

    // ── compute_progress_bps ──────────────────────────────────────────────────

    /// @notice Progress bps is always in [0, PROGRESS_BPS_CAP].
    #[test]
    fn prop_compute_progress_bps_in_range(
        raised in 0i128..=200_000_000i128,
        goal in 1i128..=100_000_000i128,
    ) {
        let bps = compute_progress_bps(raised, goal);
        prop_assert!(bps <= PROGRESS_BPS_CAP);
    }

    /// @notice Progress bps is 10_000 when raised >= goal.
    #[test]
    fn prop_compute_progress_bps_fully_funded(
        goal in GOAL_MIN..=GOAL_MAX,
        extra in 0i128..=1_000_000i128,
    ) {
        prop_assert_eq!(compute_progress_bps(goal + extra, goal), PROGRESS_BPS_CAP);
    }

    /// @notice Progress bps is 0 when raised is 0.
    #[test]
    fn prop_compute_progress_bps_zero_raised(goal in GOAL_MIN..=GOAL_MAX) {
        prop_assert_eq!(compute_progress_bps(0, goal), 0u32);
    }

    // ── compute_fee_amount ────────────────────────────────────────────────────

    /// @notice Fee amount is always non-negative.
    #[test]
    fn prop_compute_fee_amount_non_negative(
        amount in 0i128..=100_000_000i128,
        fee_bps in valid_fee_bps_strategy(),
    ) {
        prop_assert!(compute_fee_amount(amount, fee_bps) >= 0);
    }

    /// @notice Fee amount never exceeds the contribution amount.
    #[test]
    fn prop_compute_fee_amount_never_exceeds_amount(
        amount in 1i128..=100_000_000i128,
        fee_bps in valid_fee_bps_strategy(),
    ) {
        prop_assert!(compute_fee_amount(amount, fee_bps) <= amount);
    }

    /// @notice Zero fee bps always yields zero fee.
    #[test]
    fn prop_compute_fee_amount_zero_fee_bps(amount in 0i128..=100_000_000i128) {
        prop_assert_eq!(compute_fee_amount(amount, 0), 0i128);
    }
}

// ── Edge-case and regression tests ───────────────────────────────────────────

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    // ── Typo-fix regression ───────────────────────────────────────────────────

    /// Regression: 100 was the old (wrong) minimum; must remain rejected.
    #[test]
    fn regression_deadline_100_rejected() {
        assert!(!is_valid_deadline_offset(100));
    }

    /// Regression: 1000 is the corrected minimum; must be accepted.
    #[test]
    fn regression_deadline_1000_accepted() {
        assert!(is_valid_deadline_offset(1_000));
    }

    /// Regression seed: goal=1_000_000, deadline=100 was flaky; 100 now invalid.
    #[test]
    fn regression_seed_goal_1m_deadline_100() {
        assert!(is_valid_goal(1_000_000));
        assert!(!is_valid_deadline_offset(100));
    }

    /// Regression seed: goal=2_000_000, deadline=100, contribution=100_000.
    #[test]
    fn regression_seed_goal_2m_deadline_100_contribution_100k() {
        assert!(is_valid_goal(2_000_000));
        assert!(!is_valid_deadline_offset(100));
        assert!(is_valid_contribution_amount(100_000, 1_000));
    }

    // ── Exact boundary values ─────────────────────────────────────────────────

    #[test]
    fn deadline_offset_exact_boundaries() {
        assert!(!is_valid_deadline_offset(DEADLINE_OFFSET_MIN - 1));
        assert!(is_valid_deadline_offset(DEADLINE_OFFSET_MIN));
        assert!(is_valid_deadline_offset(DEADLINE_OFFSET_MAX));
        assert!(!is_valid_deadline_offset(DEADLINE_OFFSET_MAX + 1));
    }

    #[test]
    fn goal_exact_boundaries() {
        assert!(!is_valid_goal(GOAL_MIN - 1));
        assert!(is_valid_goal(GOAL_MIN));
        assert!(is_valid_goal(GOAL_MAX));
        assert!(!is_valid_goal(GOAL_MAX + 1));
    }

    #[test]
    fn min_contribution_exact_boundaries() {
        let goal = 10_000i128;
        assert!(!is_valid_min_contribution(0, goal));
        assert!(is_valid_min_contribution(1, goal));
        assert!(is_valid_min_contribution(goal, goal));
        assert!(!is_valid_min_contribution(goal + 1, goal));
    }

    #[test]
    fn fee_bps_exact_boundaries() {
        assert!(is_valid_fee_bps(0));
        assert!(is_valid_fee_bps(FEE_BPS_CAP));
        assert!(!is_valid_fee_bps(FEE_BPS_CAP + 1));
    }

    #[test]
    fn batch_size_exact_boundaries() {
        assert!(!is_valid_generator_batch_size(0));
        assert!(is_valid_generator_batch_size(1));
        assert!(is_valid_generator_batch_size(GENERATOR_BATCH_MAX));
        assert!(!is_valid_generator_batch_size(GENERATOR_BATCH_MAX + 1));
    }

    // ── Clamp extreme values ──────────────────────────────────────────────────

    #[test]
    fn clamp_progress_bps_extreme_inputs() {
        assert_eq!(clamp_progress_bps(i128::MIN), 0);
        assert_eq!(clamp_progress_bps(i128::MAX), PROGRESS_BPS_CAP);
    }

    #[test]
    fn clamp_proptest_cases_extreme_inputs() {
        assert_eq!(clamp_proptest_cases(0), PROPTEST_CASES_MIN);
        assert_eq!(clamp_proptest_cases(u32::MAX), PROPTEST_CASES_MAX);
    }

    // ── compute_progress_bps edge values ──────────────────────────────────────

    #[test]
    fn compute_progress_bps_zero_goal_safe() {
        assert_eq!(compute_progress_bps(1_000_000, 0), 0);
        assert_eq!(compute_progress_bps(0, 0), 0);
    }

    #[test]
    fn compute_progress_bps_negative_goal_safe() {
        assert_eq!(compute_progress_bps(1_000, -1), 0);
    }

    #[test]
    fn compute_progress_bps_quarter_funded() {
        assert_eq!(compute_progress_bps(250, 1_000), 2_500);
    }

    #[test]
    fn compute_progress_bps_three_quarters_funded() {
        assert_eq!(compute_progress_bps(750, 1_000), 7_500);
    }

    #[test]
    fn compute_progress_bps_half_funded() {
        assert_eq!(compute_progress_bps(500, 1_000), 5_000);
    }

    #[test]
    fn compute_progress_bps_fully_funded() {
        assert_eq!(compute_progress_bps(1_000, 1_000), 10_000);
    }

    #[test]
    fn compute_progress_bps_over_funded_clamped() {
        assert_eq!(compute_progress_bps(2_000, 1_000), PROGRESS_BPS_CAP);
    }

    // ── compute_fee_amount edge values ────────────────────────────────────────

    #[test]
    fn compute_fee_amount_negative_amount_returns_zero() {
        assert_eq!(compute_fee_amount(-1, 500), 0);
    }

    #[test]
    fn compute_fee_amount_1_percent() {
        // 1 % = 100 bps; 1_000_000 * 100 / 10_000 = 10_000
        assert_eq!(compute_fee_amount(1_000_000, 100), 10_000);
    }

    #[test]
    fn compute_fee_amount_5_percent() {
        // 5 % = 500 bps; 1_000_000 * 500 / 10_000 = 50_000
        assert_eq!(compute_fee_amount(1_000_000, 500), 50_000);
    }

    #[test]
    fn compute_fee_amount_100_percent() {
        assert_eq!(compute_fee_amount(1_000_000, 10_000), 1_000_000);
    }

    #[test]
    fn compute_fee_amount_floors_fractional() {
        // 1 stroop * 1 bps / 10_000 = 0 (integer floor)
        assert_eq!(compute_fee_amount(1, 1), 0);
    }

    // ── boundary_log_tag ──────────────────────────────────────────────────────

    #[test]
    fn boundary_log_tag_value() {
        assert_eq!(boundary_log_tag(), "proptest_boundary");
    }

    #[test]
    fn boundary_log_tag_is_non_empty() {
        assert!(!boundary_log_tag().is_empty());
    }

    // ── Constant stability ────────────────────────────────────────────────────

    #[test]
    fn all_constants_stable() {
        assert_eq!(DEADLINE_OFFSET_MIN, 1_000);
        assert_eq!(DEADLINE_OFFSET_MAX, 1_000_000);
        assert_eq!(GOAL_MIN, 1_000);
        assert_eq!(GOAL_MAX, 100_000_000);
        assert_eq!(MIN_CONTRIBUTION_FLOOR, 1);
        assert_eq!(PROGRESS_BPS_CAP, 10_000);
        assert_eq!(FEE_BPS_CAP, 10_000);
        assert_eq!(PROPTEST_CASES_MIN, 32);
        assert_eq!(PROPTEST_CASES_MAX, 256);
        assert_eq!(GENERATOR_BATCH_MAX, 512);
    }
}
