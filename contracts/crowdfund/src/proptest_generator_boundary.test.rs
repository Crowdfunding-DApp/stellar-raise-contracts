//! Comprehensive tests for the ProptestGeneratorBoundary contract.
//!
//! @title   ProptestGeneratorBoundary Tests
//! @notice  Validates correct return of boundary constants and logic for
//!          clamping, validation, and derived calculations.
//! @dev     Includes both unit tests and property-based tests for boundary
//!          safety. Property tests use 256 cases per property.
//!
//! ## Test Coverage
//!
//! - **Constant Sanity Checks**: Verify all constants return correct values.
//! - **Validation Functions**: Unit tests for each `is_valid_*` function.
//! - **Clamping Functions**: Unit tests for `clamp_*` functions.
//! - **Derived Calculations**: Unit tests for `compute_*` functions.
//! - **Property-Based Tests**: Proptest with 256 cases per property.
//! - **Edge Cases**: Boundary values, overflow scenarios, zero/negative inputs.
//! - **Regression Seeds**: Known problematic values from CI failures.
//!
//! Target: ≥95% line coverage.

#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, Symbol};
    use proptest::prelude::*;
    use crate::proptest_generator_boundary::{
        ProptestGeneratorBoundary, ProptestGeneratorBoundaryClient,
        DEADLINE_OFFSET_MIN, DEADLINE_OFFSET_MAX, GOAL_MIN, GOAL_MAX,
        MIN_CONTRIBUTION_FLOOR, PROGRESS_BPS_CAP, FEE_BPS_CAP,
        PROPTEST_CASES_MIN, PROPTEST_CASES_MAX, GENERATOR_BATCH_MAX,
    };

    // ── Setup Helper ──────────────────────────────────────────────────────────

    /// @notice Creates a fresh test environment with the boundary contract registered.
    /// @return (Env, ProptestGeneratorBoundaryClient)
    fn setup() -> (Env, ProptestGeneratorBoundaryClient<'static>) {
        let env = Env::default();
        let contract_id = env.register(ProptestGeneratorBoundary, ());
        let client = ProptestGeneratorBoundaryClient::new(&env, &contract_id);
        (env, client)
    }

    // ── Constant Sanity Checks ────────────────────────────────────────────────

    /// @notice Verifies every getter returns the expected compile-time constant.
    /// @dev    Single test covers all ten getters to keep the suite concise.
    #[test]
    fn test_constants_return_correct_values() {
        let (_env, client) = setup();
        assert_eq!(client.deadline_offset_min(), DEADLINE_OFFSET_MIN);
        assert_eq!(client.deadline_offset_max(), DEADLINE_OFFSET_MAX);
        assert_eq!(client.goal_min(), GOAL_MIN);
        assert_eq!(client.goal_max(), GOAL_MAX);
        assert_eq!(client.min_contribution_floor(), MIN_CONTRIBUTION_FLOOR);
        assert_eq!(client.progress_bps_cap(), PROGRESS_BPS_CAP);
        assert_eq!(client.fee_bps_cap(), FEE_BPS_CAP);
        assert_eq!(client.proptest_cases_min(), PROPTEST_CASES_MIN);
        assert_eq!(client.proptest_cases_max(), PROPTEST_CASES_MAX);
        assert_eq!(client.generator_batch_max(), GENERATOR_BATCH_MAX);
    }

    /// @notice Verifies that range constants are ordered correctly (min < max).
    /// @dev    Guards against accidental constant transposition during refactors.
    #[test]
    fn test_constants_are_ordered_correctly() {
        assert!(DEADLINE_OFFSET_MIN < DEADLINE_OFFSET_MAX);
        assert!(GOAL_MIN < GOAL_MAX);
        assert!(PROPTEST_CASES_MIN < PROPTEST_CASES_MAX);
        assert!(PROGRESS_BPS_CAP > 0);
        assert!(FEE_BPS_CAP > 0);
        assert!(GENERATOR_BATCH_MAX > 0);
    }

    // ── Deadline Offset Validation ────────────────────────────────────────────

    /// @notice Verifies boundary values for deadline offset validation.
    /// @dev    Tests MIN, MIN-1, MAX, MAX+1, and a mid-range value.
    #[test]
    fn test_is_valid_deadline_offset_boundary_values() {
        let (_env, client) = setup();
        assert!(client.is_valid_deadline_offset(&DEADLINE_OFFSET_MIN));
        assert!(!client.is_valid_deadline_offset(&(DEADLINE_OFFSET_MIN - 1)));
        assert!(client.is_valid_deadline_offset(&DEADLINE_OFFSET_MAX));
        assert!(!client.is_valid_deadline_offset(&(DEADLINE_OFFSET_MAX + 1)));
        assert!(client.is_valid_deadline_offset(&500_000));
    }

    /// @notice Verifies that zero, near-zero, and u64::MAX are all invalid offsets.
    /// @dev    Edge cases that could cause timestamp overflow or flaky tests.
    #[test]
    fn test_is_valid_deadline_offset_edge_cases() {
        let (_env, client) = setup();
        assert!(!client.is_valid_deadline_offset(&0));
        assert!(!client.is_valid_deadline_offset(&999));
        assert!(!client.is_valid_deadline_offset(&u64::MAX));
    }

    // ── Goal Validation ──────────────────────────────────────────────────────

    /// @notice Verifies boundary values for goal validation.
    /// @dev    Tests GOAL_MIN, GOAL_MIN-1, GOAL_MAX, GOAL_MAX+1, and mid-range.
    #[test]
    fn test_is_valid_goal_boundary_values() {
        let (_env, client) = setup();
        assert!(client.is_valid_goal(&GOAL_MIN));
        assert!(!client.is_valid_goal(&(GOAL_MIN - 1)));
        assert!(client.is_valid_goal(&GOAL_MAX));
        assert!(!client.is_valid_goal(&(GOAL_MAX + 1)));
        assert!(client.is_valid_goal(&50_000_000));
    }

    /// @notice Verifies that zero, negative, and i128::MIN are all invalid goals.
    /// @dev    Zero goal causes division-by-zero in progress calculations.
    #[test]
    fn test_is_valid_goal_edge_cases() {
        let (_env, client) = setup();
        assert!(!client.is_valid_goal(&0));
        assert!(!client.is_valid_goal(&-1));
        assert!(!client.is_valid_goal(&999));
        assert!(!client.is_valid_goal(&i128::MIN));
    }

    // ── Minimum Contribution Validation ───────────────────────────────────────

    /// @notice Verifies valid and invalid min_contribution values against a goal.
    /// @dev    min_contribution must be in [MIN_CONTRIBUTION_FLOOR, goal].
    #[test]
    fn test_is_valid_min_contribution() {
        let (_env, client) = setup();
        let goal = 1_000_000;
        assert!(client.is_valid_min_contribution(&MIN_CONTRIBUTION_FLOOR, &goal));
        assert!(client.is_valid_min_contribution(&500_000, &goal));
        assert!(client.is_valid_min_contribution(&goal, &goal));
        assert!(!client.is_valid_min_contribution(&0, &goal));
        assert!(!client.is_valid_min_contribution(&(goal + 1), &goal));
        assert!(!client.is_valid_min_contribution(&-1, &goal));
    }

    /// @notice Verifies min_contribution validation when goal equals GOAL_MIN.
    /// @dev    Boundary case: only MIN_CONTRIBUTION_FLOOR is valid.
    #[test]
    fn test_is_valid_min_contribution_with_min_goal() {
        let (_env, client) = setup();
        assert!(client.is_valid_min_contribution(&MIN_CONTRIBUTION_FLOOR, &GOAL_MIN));
        assert!(!client.is_valid_min_contribution(&(GOAL_MIN + 1), &GOAL_MIN));
    }

    // ── Contribution Amount Validation ────────────────────────────────────────

    /// @notice Verifies valid and invalid contribution amounts against a minimum.
    /// @dev    amount must be >= min_contribution.
    #[test]
    fn test_is_valid_contribution_amount() {
        let (_env, client) = setup();
        let min_contribution = 1_000;
        assert!(client.is_valid_contribution_amount(&min_contribution, &min_contribution));
        assert!(client.is_valid_contribution_amount(&(min_contribution + 1), &min_contribution));
        assert!(client.is_valid_contribution_amount(&1_000_000, &min_contribution));
        assert!(!client.is_valid_contribution_amount(&(min_contribution - 1), &min_contribution));
        assert!(!client.is_valid_contribution_amount(&0, &min_contribution));
        assert!(!client.is_valid_contribution_amount(&-1, &min_contribution));
    }

    // ── Fee Basis Points Validation ───────────────────────────────────────────

    /// @notice Verifies valid and invalid fee basis point values.
    /// @dev    fee_bps must be in [0, FEE_BPS_CAP]. Values above cap are rejected.
    #[test]
    fn test_is_valid_fee_bps() {
        let (_env, client) = setup();
        assert!(client.is_valid_fee_bps(&0));
        assert!(client.is_valid_fee_bps(&5_000));
        assert!(client.is_valid_fee_bps(&FEE_BPS_CAP));
        assert!(!client.is_valid_fee_bps(&(FEE_BPS_CAP + 1)));
        assert!(!client.is_valid_fee_bps(&u32::MAX));
    }

    // ── Generator Batch Size Validation ───────────────────────────────────────

    /// @notice Verifies valid and invalid generator batch sizes.
    /// @dev    batch_size must be in [1, GENERATOR_BATCH_MAX]. Zero is rejected.
    #[test]
    fn test_is_valid_generator_batch_size() {
        let (_env, client) = setup();
        assert!(client.is_valid_generator_batch_size(&1));
        assert!(client.is_valid_generator_batch_size(&256));
        assert!(client.is_valid_generator_batch_size(&GENERATOR_BATCH_MAX));
        assert!(!client.is_valid_generator_batch_size(&0));
        assert!(!client.is_valid_generator_batch_size(&(GENERATOR_BATCH_MAX + 1)));
    }

    // ── Clamping Functions ────────────────────────────────────────────────────

    /// @notice Verifies that clamp_proptest_cases clamps to [PROPTEST_CASES_MIN, PROPTEST_CASES_MAX].
    /// @dev    Values below min clamp up; values above max clamp down; in-range pass through.
    #[test]
    fn test_clamp_proptest_cases() {
        let (_env, client) = setup();
        assert_eq!(client.clamp_proptest_cases(&0), PROPTEST_CASES_MIN);
        assert_eq!(client.clamp_proptest_cases(&1), PROPTEST_CASES_MIN);
        assert_eq!(client.clamp_proptest_cases(&64), 64);
        assert_eq!(client.clamp_proptest_cases(&128), 128);
        assert_eq!(client.clamp_proptest_cases(&1000), PROPTEST_CASES_MAX);
        assert_eq!(client.clamp_proptest_cases(&u32::MAX), PROPTEST_CASES_MAX);
    }

    /// @notice Verifies that clamp_progress_bps clamps to [0, PROGRESS_BPS_CAP].
    /// @dev    Negative values floor to 0; values above 10,000 cap at 10,000.
    #[test]
    fn test_clamp_progress_bps() {
        let (_env, client) = setup();
        assert_eq!(client.clamp_progress_bps(&-1000), 0);
        assert_eq!(client.clamp_progress_bps(&-1), 0);
        assert_eq!(client.clamp_progress_bps(&0), 0);
        assert_eq!(client.clamp_progress_bps(&5000), 5000);
        assert_eq!(client.clamp_progress_bps(&10000), PROGRESS_BPS_CAP);
        assert_eq!(client.clamp_progress_bps(&10001), PROGRESS_BPS_CAP);
        assert_eq!(client.clamp_progress_bps(&i128::MAX), PROGRESS_BPS_CAP);
    }

    // ── compute_progress_bps Tests ────────────────────────────────────────────

    /// @notice Verifies basic progress calculations: 50%, 100%, and over-funded (capped).
    #[test]
    fn test_compute_progress_bps_basic() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&500, &1000), 5000);
        assert_eq!(client.compute_progress_bps(&1000, &1000), 10000);
        assert_eq!(client.compute_progress_bps(&2000, &1000), 10000);
    }

    /// @notice Verifies progress returns 0 for zero/negative goal and negative raised.
    /// @dev    Guards against division-by-zero and negative display values.
    #[test]
    fn test_compute_progress_bps_edge_cases() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&500, &0), 0);
        assert_eq!(client.compute_progress_bps(&500, &-1000), 0);
        assert_eq!(client.compute_progress_bps(&-100, &1000), 0);
        assert_eq!(client.compute_progress_bps(&1, &10000), 1);
    }

    /// @notice Verifies that large raised values do not overflow — result caps at PROGRESS_BPS_CAP.
    /// @dev    saturating_mul prevents i128 overflow before the division.
    #[test]
    fn test_compute_progress_bps_overflow_safety() {
        let (_env, client) = setup();
        let large_raised = i128::MAX / 2;
        let result = client.compute_progress_bps(&large_raised, &1_000);
        assert_eq!(result, PROGRESS_BPS_CAP);
    }

    /// @notice Verifies negative raised values always return 0.
    #[test]
    fn test_compute_progress_bps_negative_raised() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&-1_000, &1_000), 0);
        assert_eq!(client.compute_progress_bps(&-100_000_000, &1_000), 0);
    }

    /// @notice Verifies partial progress values (25%, 50%, near-zero).
    #[test]
    fn test_compute_progress_bps_partial_progress() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&500, &1_000), 5_000);
        assert_eq!(client.compute_progress_bps(&250, &1_000), 2_500);
        assert_eq!(client.compute_progress_bps(&1, &1_000), 10);
    }

    /// @notice Verifies that exactly meeting the goal returns 10,000 bps.
    #[test]
    fn test_compute_progress_bps_full_progress() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&1_000, &1_000), 10_000);
        assert_eq!(client.compute_progress_bps(&100_000_000, &100_000_000), 10_000);
    }

    /// @notice Verifies that exceeding the goal caps at 10,000 bps.
    #[test]
    fn test_compute_progress_bps_over_goal() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&2_000, &1_000), 10_000);
        assert_eq!(client.compute_progress_bps(&200_000_000, &100_000_000), 10_000);
    }

    // ── compute_fee_amount Tests ─────────────────────────────────────────────

    /// @notice Verifies basic fee calculations: 10%, 50%, 100%.
    #[test]
    fn test_compute_fee_amount_basic() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&1000, &1000), 100);
        assert_eq!(client.compute_fee_amount(&1000, &5000), 500);
        assert_eq!(client.compute_fee_amount(&1000, &10000), 1000);
    }

    /// @notice Verifies fee returns 0 for zero/negative amount and zero fee_bps.
    #[test]
    fn test_compute_fee_amount_edge_cases() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&0, &5000), 0);
        assert_eq!(client.compute_fee_amount(&-1000, &5000), 0);
        assert_eq!(client.compute_fee_amount(&1000, &0), 0);
        assert_eq!(client.compute_fee_amount(&0, &0), 0);
    }

    /// @notice Verifies that integer floor division is applied correctly.
    /// @dev    1/3 of 1000 = 333 (not 334); 2/3 of 1000 = 666 (not 667).
    #[test]
    fn test_compute_fee_amount_floor_division() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&1000, &3333), 333);
        assert_eq!(client.compute_fee_amount(&1000, &6666), 666);
    }

    /// @notice Verifies fee returns 0 when amount is zero, for any fee_bps.
    #[test]
    fn test_compute_fee_amount_zero_amount() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&0, &1_000), 0);
        assert_eq!(client.compute_fee_amount(&0, &10_000), 0);
    }

    /// @notice Verifies fee returns 0 when amount is negative.
    #[test]
    fn test_compute_fee_amount_negative_amount() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&-1_000, &1_000), 0);
        assert_eq!(client.compute_fee_amount(&-100_000_000, &5_000), 0);
    }

    /// @notice Verifies fee returns 0 when fee_bps is zero, for any amount.
    #[test]
    fn test_compute_fee_amount_zero_fee() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&1_000, &0), 0);
        assert_eq!(client.compute_fee_amount(&100_000_000, &0), 0);
    }

    /// @notice Verifies correct fee calculations for standard rates and amounts.
    #[test]
    fn test_compute_fee_amount_valid_calculations() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&1_000, &1_000), 100);
        assert_eq!(client.compute_fee_amount(&1_000, &5_000), 500);
        assert_eq!(client.compute_fee_amount(&1_000, &10_000), 1_000);
        assert_eq!(client.compute_fee_amount(&10_000, &1_000), 1_000);
    }

    /// @notice Verifies fee calculations for large contribution amounts.
    #[test]
    fn test_compute_fee_amount_large_values() {
        let (_env, client) = setup();
        assert_eq!(client.compute_fee_amount(&100_000_000, &1_000), 10_000_000);
        assert_eq!(client.compute_fee_amount(&100_000_000, &5_000), 50_000_000);
    }

    // ── log_tag Tests ────────────────────────────────────────────────────────

    /// @notice Verifies that log_tag returns the Symbol "boundary".
    /// @dev    Off-chain indexers filter boundary-related events by this tag.
    #[test]
    fn test_log_tag() {
        let (env, client) = setup();
        assert_eq!(client.log_tag(), Symbol::new(&env, "boundary"));
    }

    // ── Property-Based Tests ──────────────────────────────────────────────────
    /// @notice Property tests use proptest to explore the input space systematically.
    ///         Each property is tested with 256 randomly generated cases.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// @notice Property: every offset in [MIN, MAX] passes validation.
        #[test]
        fn prop_deadline_offset_validity(offset in DEADLINE_OFFSET_MIN..=DEADLINE_OFFSET_MAX) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_deadline_offset(&offset));
        }

        /// @notice Property: every offset below MIN fails validation.
        #[test]
        fn prop_deadline_offset_below_min_invalid(offset in 0u64..DEADLINE_OFFSET_MIN) {
            let (_env, client) = setup();
            prop_assert!(!client.is_valid_deadline_offset(&offset));
        }

        /// @notice Property: every offset above MAX fails validation.
        #[test]
        fn prop_deadline_offset_above_max_invalid(offset in (DEADLINE_OFFSET_MAX + 1)..u64::MAX) {
            let (_env, client) = setup();
            prop_assert!(!client.is_valid_deadline_offset(&offset));
        }

        /// @notice Property: every goal in [GOAL_MIN, GOAL_MAX] passes validation.
        #[test]
        fn prop_goal_validity(goal in GOAL_MIN..=GOAL_MAX) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_goal(&goal));
        }

        /// @notice Property: every goal below GOAL_MIN fails validation.
        #[test]
        fn prop_goal_below_min_invalid(goal in i128::MIN..GOAL_MIN) {
            let (_env, client) = setup();
            prop_assert!(!client.is_valid_goal(&goal));
        }

        /// @notice Property: every goal above GOAL_MAX fails validation.
        #[test]
        fn prop_goal_above_max_invalid(goal in (GOAL_MAX + 1)..i128::MAX) {
            let (_env, client) = setup();
            prop_assert!(!client.is_valid_goal(&goal));
        }

        /// @notice Property: compute_progress_bps never exceeds PROGRESS_BPS_CAP.
        #[test]
        fn prop_progress_bps_always_bounded(
            raised in -1_000_000_000i128..=1_000_000_000i128,
            goal in GOAL_MIN..=GOAL_MAX
        ) {
            let (_env, client) = setup();
            let bps = client.compute_progress_bps(&raised, &goal);
            prop_assert!(bps <= PROGRESS_BPS_CAP);
        }

        /// @notice Property: compute_progress_bps returns 0 when goal is zero.
        #[test]
        fn prop_progress_bps_zero_when_goal_zero(raised in -1_000_000i128..=1_000_000i128) {
            let (_env, client) = setup();
            let bps = client.compute_progress_bps(&raised, &0);
            prop_assert_eq!(bps, 0);
        }

        /// @notice Property: compute_progress_bps returns 0 when raised is negative.
        #[test]
        fn prop_progress_bps_zero_when_raised_negative(goal in GOAL_MIN..=GOAL_MAX) {
            let (_env, client) = setup();
            let bps = client.compute_progress_bps(&-1000, &goal);
            prop_assert_eq!(bps, 0);
        }

        /// @notice Property: compute_fee_amount is always non-negative.
        #[test]
        fn prop_fee_amount_always_non_negative(
            amount in -1_000_000i128..=1_000_000i128,
            fee_bps in 0u32..=FEE_BPS_CAP
        ) {
            let (_env, client) = setup();
            let fee = client.compute_fee_amount(&amount, &fee_bps);
            prop_assert!(fee >= 0);
        }

        /// @notice Property: compute_fee_amount returns 0 when amount is zero.
        #[test]
        fn prop_fee_amount_zero_when_amount_zero(fee_bps in 0u32..=FEE_BPS_CAP) {
            let (_env, client) = setup();
            let fee = client.compute_fee_amount(&0, &fee_bps);
            prop_assert_eq!(fee, 0);
        }

        /// @notice Property: compute_fee_amount returns 0 when fee_bps is zero.
        #[test]
        fn prop_fee_amount_zero_when_fee_zero(amount in -1_000_000i128..=1_000_000i128) {
            let (_env, client) = setup();
            let fee = client.compute_fee_amount(&amount, &0);
            prop_assert_eq!(fee, 0);
        }

        /// @notice Property: clamp_proptest_cases always returns a value in [MIN, MAX].
        #[test]
        fn prop_clamp_proptest_cases_within_bounds(requested in 0u32..=u32::MAX) {
            let (_env, client) = setup();
            let clamped = client.clamp_proptest_cases(&requested);
            prop_assert!(clamped >= PROPTEST_CASES_MIN);
            prop_assert!(clamped <= PROPTEST_CASES_MAX);
        }

        /// @notice Property: clamp_progress_bps always returns a value in [0, PROGRESS_BPS_CAP].
        #[test]
        fn prop_clamp_progress_bps_within_bounds(raw in i128::MIN..=i128::MAX) {
            let (_env, client) = setup();
            let clamped = client.clamp_progress_bps(&raw);
            prop_assert!(clamped <= PROGRESS_BPS_CAP);
        }

        /// @notice Property: is_valid_min_contribution returns true when min_contrib <= goal.
        #[test]
        fn prop_min_contribution_valid_when_in_range(
            min_contrib in MIN_CONTRIBUTION_FLOOR..=GOAL_MAX,
            goal in GOAL_MIN..=GOAL_MAX
        ) {
            let (_env, client) = setup();
            if min_contrib <= goal {
                prop_assert!(client.is_valid_min_contribution(&min_contrib, &goal));
            }
        }

        /// @notice Property: is_valid_contribution_amount returns true when amount >= min_contrib.
        #[test]
        fn prop_contribution_amount_valid_when_meets_minimum(
            amount in MIN_CONTRIBUTION_FLOOR..=1_000_000i128,
            min_contrib in MIN_CONTRIBUTION_FLOOR..=1_000_000i128
        ) {
            let (_env, client) = setup();
            if amount >= min_contrib {
                prop_assert!(client.is_valid_contribution_amount(&amount, &min_contrib));
            }
        }

        /// @notice Property: is_valid_fee_bps returns true for all values in [0, FEE_BPS_CAP].
        #[test]
        fn prop_fee_bps_valid_when_within_cap(fee_bps in 0u32..=FEE_BPS_CAP) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_fee_bps(&fee_bps));
        }

        /// @notice Property: is_valid_generator_batch_size returns true for all values in [1, MAX].
        #[test]
        fn prop_batch_size_valid_when_in_range(batch_size in 1u32..=GENERATOR_BATCH_MAX) {
            let (_env, client) = setup();
            prop_assert!(client.is_valid_generator_batch_size(&batch_size));
        }
    }

    // ── Regression Tests ──────────────────────────────────────────────────────
    /// @notice Regression tests capture known problematic values from CI failures.

    /// @notice Verifies that a 100-second offset (previously accepted) is now rejected.
    /// @dev    Previously caused flaky tests due to timing races.
    #[test]
    fn regression_deadline_offset_100_seconds_now_invalid() {
        let (_env, client) = setup();
        assert!(!client.is_valid_deadline_offset(&100));
    }

    /// @notice Verifies that a zero goal is always rejected.
    /// @dev    Zero goal causes division-by-zero in progress calculations.
    #[test]
    fn regression_goal_zero_always_invalid() {
        let (_env, client) = setup();
        assert!(!client.is_valid_goal(&0));
    }

    /// @notice Verifies that progress never exceeds PROGRESS_BPS_CAP even with extreme inputs.
    #[test]
    fn regression_progress_bps_never_exceeds_cap() {
        let (_env, client) = setup();
        assert_eq!(client.compute_progress_bps(&i128::MAX, &1), PROGRESS_BPS_CAP);
    }

    /// @notice Verifies that fee amount is never negative even with negative inputs.
    #[test]
    fn regression_fee_amount_never_negative() {
        let (_env, client) = setup();
        assert!(client.compute_fee_amount(&-1_000_000, &5000) >= 0);
    }
}
