# Property-Based Testing

## What is property-based testing?

Property-based testing generates hundreds of random inputs and checks that a
stated invariant holds for all of them. Unlike unit tests — which verify a
handful of hand-picked examples — property tests explore the full input space
and are especially good at finding edge cases near integer boundaries, zero
values, and maximum values.

This project uses the [`proptest`](https://crates.io/crates/proptest) crate
(version 1.x) for all property tests.

## Why use it here?

The crowdfund contract handles real token transfers. A single arithmetic bug
(e.g. an unchecked addition that wraps near `i128::MAX`) could allow an
attacker to drain funds or claim a refund they are not entitled to. Property
tests give high confidence that:

- Overflow guards fire before any state is written.
- Validation functions reject every invalid input, not just the ones we thought
  of when writing unit tests.
- Pure helper functions (progress calculation, fee deduction) are deterministic
  and bounded.

## How to run

```bash
# Run all tests in the workspace (includes property tests)
cargo test --workspace

# Run only the property-based tests
cargo test --package crowdfund property_based_testing

# Run with more cases (slower but more thorough)
PROPTEST_CASES=1024 cargo test --package crowdfund property_based_testing
```

## Properties tested

| Property | Invariant | Security assumption |
|---|---|---|
| `prop_pledge_accumulation_exact` | `prior + amount` equals the new total for all valid i128 pairs | Arithmetic must be exact; truncation lets a contributor under-pay |
| `prop_past_deadline_always_rejected` | Any deadline < `now + MIN_DEADLINE_OFFSET` is rejected | Prevents instant-expiry drain attacks |
| `prop_future_deadline_always_accepted` | Any deadline ≥ `now + MIN_DEADLINE_OFFSET` is accepted | Valid campaigns must not be blocked |
| `prop_refund_requires_failed_campaign` | Refund is allowed iff deadline passed AND goal not met | Contributors must not refund during an active or successful campaign |
| `prop_no_refund_when_goal_met` | Goal met → refund never allowed | Prevents double-spend after successful withdrawal |
| `prop_no_refund_while_active` | Deadline not passed → refund never allowed | Campaign funds must stay locked while active |
| `prop_overflow_detected_near_max` | `checked_add` returns `None` when result would exceed `i128::MAX` | Attacker cannot wrap `total_raised` to a small value |
| `prop_zero_amount_always_invalid` | Amount = 0 is always rejected | Zero-amount transfers waste gas and pollute storage |
| `prop_negative_amount_always_invalid` | Negative amounts are always rejected | Prevents negative-transfer exploits |
| `prop_progress_bps_idempotent` | `compute_progress_bps(r, g)` returns the same value on every call | View functions must have no hidden side effects |
| `prop_clamp_idempotent` | `clamp(clamp(x)) == clamp(x)` | Clamping must be stable |
| `prop_goal_below_minimum_rejected` | `goal < MIN_GOAL_AMOUNT` is always rejected | Zero-goal campaigns could be trivially "succeeded" |
| `prop_valid_goal_accepted` | `goal >= MIN_GOAL_AMOUNT` is always accepted | Valid campaigns must not be blocked |
| `prop_below_min_contribution_rejected` | `amount < min_contribution` is always rejected | Prevents spam attacks that bloat the contributor list |
| `prop_at_or_above_min_contribution_accepted` | `amount >= min_contribution` is always accepted | Valid contributions must not be blocked |
| `prop_fee_above_cap_rejected` | `fee_bps > 10 000` is always rejected | Fee > 100% would drain more than the contract holds |
| `prop_valid_fee_accepted` | `fee_bps <= 10 000` is always accepted | Valid fee configurations must not be blocked |
| `prop_progress_bps_never_exceeds_cap` | Progress is always ≤ 10 000 bps | Frontend must never display > 100% funded |
| `prop_clamp_always_within_cap` | `clamp_progress_bps` always returns ≤ 10 000 | Clamping must enforce the cap unconditionally |
| `prop_net_payout_never_exceeds_total` | Creator payout ≤ total raised for all valid fee values | Fee arithmetic must not produce a payout larger than the contract holds |
| `prop_zero_fee_payout_equals_total` | 0% fee → full payout | No fee must mean the creator receives everything |
| `prop_full_fee_payout_is_zero` | 100% fee → zero payout | 100% fee must leave the creator with nothing |

## Known limitations

- **No Soroban `Env` integration** — the property tests exercise pure helper
  functions and validation logic. They do not spin up a full Soroban ledger
  environment, so they cannot test token transfer side effects, event emission,
  or storage TTL behaviour. Those paths are covered by the unit tests in
  `test.rs`.

- **`contribute` / `withdraw` / `refund_single` entry points** — these require
  a live `Env` with a mock token contract. Property testing them end-to-end
  would require a custom proptest strategy that generates valid `Address`
  values, which is not yet supported without significant test-harness work.

- **Boundary constants are fixed** — `MIN_GOAL_AMOUNT`, `MIN_DEADLINE_OFFSET`,
  and `FEE_BPS_CAP` are compile-time constants. If they change, the property
  tests automatically adapt because they import the constants directly.

- **Case count** — each `proptest!` block runs 256 cases by default. Increase
  with `PROPTEST_CASES=N` for deeper exploration at the cost of slower CI.
