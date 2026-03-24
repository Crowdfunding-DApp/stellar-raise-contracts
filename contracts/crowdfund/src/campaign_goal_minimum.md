# `campaign_goal_minimum` — Campaign goal & minimum threshold enforcement

## Overview

`campaign_goal_minimum` centralizes **on-chain** magic numbers and thresholds used
during `initialize()` and basis-point progress math. It exposes pure validation
helpers and constants that can be imported from the crate, from tests, and
referenced in off-chain tooling.

**Deprecated pattern:** scattering literals such as `10_000` and ad-hoc deadline
checks across `lib.rs`. All such paths now go through this module and
[`compute_progress_bps`](./campaign_goal_minimum.rs) so reviewers and frontends
have a single source of truth.

## Why extract constants?

| Literal / pattern | Previous risk | Mitigation |
|-------------------|---------------|------------|
| `10_000` (bps scale / fee divisor) | Inconsistent updates, audit noise | `PROGRESS_BPS_SCALE`, `MAX_PLATFORM_FEE_BPS` |
| Goal / min contribution `0` | Zero-goal drain, useless contributions | `validate_goal`, `validate_min_contribution` |
| Short deadline | Dead-on-arrival campaigns | `validate_deadline` + `MIN_DEADLINE_OFFSET` |

Named constants are resolved at **compile time** (no extra gas for the constants
themselves). View functions on the contract (see below) expose the same values
for Soroban clients that cannot import Rust.

## Constants

| Constant | Type | Value | Description |
|----------|------|-------|-------------|
| `MIN_GOAL_AMOUNT` | `i128` | `1` | Minimum campaign goal in token units |
| `MIN_CONTRIBUTION_AMOUNT` | `i128` | `1` | Minimum value for the `min_contribution` parameter at init |
| `MAX_PLATFORM_FEE_BPS` | `u32` | `10_000` | Maximum platform fee (100 % in basis points) |
| `PROGRESS_BPS_SCALE` | `i128` | `10_000` | Scale factor for all basis-point progress calculations |
| `MIN_DEADLINE_OFFSET` | `u64` | `60` | Minimum seconds the deadline must be in the future at init |
| `MAX_PROGRESS_BPS` | `u32` | `10_000` | Cap on progress value returned to callers |

## Validation helpers

### `validate_goal(goal: i128) -> Result<(), &'static str>`

Ensures `goal >= MIN_GOAL_AMOUNT`. A zero goal would allow the creator to treat
the campaign as “funded” after arbitrary dust.

### `validate_min_contribution(min_contribution: i128) -> Result<(), &'static str>`

Ensures `min_contribution >= MIN_CONTRIBUTION_AMOUNT`.

### `validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str>`

Ensures `deadline >= now + MIN_DEADLINE_OFFSET` using `saturating_add` on `now`
so `u64::MAX` does not wrap.

### `validate_platform_fee(fee_bps: u32) -> Result<(), &'static str>`

Ensures `fee_bps <= MAX_PLATFORM_FEE_BPS`.

### `compute_progress_bps(total_raised: i128, goal: i128) -> u32`

Computes `(total_raised * PROGRESS_BPS_SCALE) / goal`, capped at
`MAX_PROGRESS_BPS`. Returns `0` if `goal <= 0` (division-by-zero guard). Uses
`checked_mul` on the product: if `total_raised * PROGRESS_BPS_SCALE` overflows
`i128`, returns `MAX_PROGRESS_BPS` instead of wrapping.

## On-chain policy view functions (frontend / scalability)

Integrations should **call these** instead of hardcoding thresholds so UI
validation stays aligned after WASM upgrades:

| Method | Returns |
|--------|---------|
| `policy_min_goal_amount` | `MIN_GOAL_AMOUNT` |
| `policy_min_contribution_floor` | `MIN_CONTRIBUTION_AMOUNT` |
| `policy_min_deadline_offset_secs` | `MIN_DEADLINE_OFFSET` |
| `policy_max_platform_fee_bps` | `MAX_PLATFORM_FEE_BPS` |
| `policy_progress_bps_scale` | `PROGRESS_BPS_SCALE` |

**Note:** `contracts/crowdfund/src/proptest_generator_boundary.rs` uses **stricter
test-only ranges** (e.g. larger minimum goal for proptest). That module is for
property-test stability and UI *suggested* bounds — it does not override
on-chain enforcement.

## Security assumptions

1. **`MIN_GOAL_AMOUNT`** — Prevents zero-goal campaigns that could be abused for
   immediate “success” semantics after minimal funding.
2. **`MIN_CONTRIBUTION_AMOUNT`** — Prevents zero-amount contributions that waste
   gas and grow contributor storage without economic meaning.
3. **`MAX_PLATFORM_FEE_BPS`** — Caps the fee at 100 % so fee math never implies
   taking more than the raised total.
4. **`PROGRESS_BPS_SCALE`** — Single scale for fee divisor (`withdraw`) and
   progress views (`get_stats`, `bonus_goal_progress_bps`), avoiding mismatched
   literals.
5. **`MIN_DEADLINE_OFFSET`** — Ensures the campaign is not initialized already
   expired relative to practical submission latency.
6. **`compute_progress_bps` overflow** — `checked_mul` avoids silent `i128`
   wrap on extreme inputs; capped result is safe for downstream percentage UI.

## Integration with `lib.rs`

`initialize` runs `validate_goal`, `validate_min_contribution`, and
`validate_deadline` before persisting campaign fields. Optional `bonus_goal`
values also pass `validate_goal`. Platform fees use `validate_platform_fee`
(with a stable panic message for the 100 % cap). `get_stats`,
`bonus_goal_progress_bps`, and platform fee division in `withdraw` use
`PROGRESS_BPS_SCALE` / `compute_progress_bps`.

## Test coverage

See [`campaign_goal_minimum.test.rs`](./campaign_goal_minimum.test.rs) (wired as
`campaign_goal_minimum_test` in `lib.rs` via `#[path = "..."]`). Tests cover:

- Constant stability and `PROGRESS_BPS_SCALE == MAX_PROGRESS_BPS` invariant
- All validation helpers: boundaries, negatives, `u64` / `i128` edge cases
- `compute_progress_bps`: fractional progress, cap when over goal, zero/negative
  goal, **overflow on multiply**

Integration tests in `test.rs` assert `initialize` panics on invalid goal, min
contribution, or deadline, and that policy view functions match module
constants.

### Measuring coverage (target ≥ 95 % for this module)

```bash
cargo install cargo-llvm-cov --locked  # once
cd /path/to/stellar-raise-contracts
cargo llvm-cov -p crowdfund --lcov --output-path /tmp/crowdfund.lcov
```

As of the latest run on this branch, **line coverage (LCOV `DA` records) for
`campaign_goal_minimum.rs` is 100 %** (35/35 instrumented lines hit), exceeding
the 95 % target for this module.

For HTML output: `cargo llvm-cov -p crowdfund --html --output-dir target/cov`

## Off-chain TypeScript example

```typescript
const PROGRESS_BPS_SCALE = 10_000n;
const MAX_PROGRESS_BPS = 10_000n;

function computeProgressBps(totalRaised: bigint, goal: bigint): number {
  if (goal <= 0n) return 0;
  const raw = (totalRaised * PROGRESS_BPS_SCALE) / goal;
  return Number(raw > MAX_PROGRESS_BPS ? MAX_PROGRESS_BPS : raw);
}
```

Prefer fetching `policy_*` from the contract for thresholds. For amounts near
`i128::MAX`, mirror Rust’s `checked_mul` (cap at `MAX_PROGRESS_BPS` if multiply
would overflow a bounded integer type).
