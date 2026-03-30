# Performance Profiling

Lightweight profiling primitives for measuring and reporting execution metrics
in the Stellar Raise crowdfund contract test suite.

## Overview

`performance_profiling.rs` provides a `ProfileSample` / `ProfileReport` pair
for collecting instruction and memory measurements during contract testing.
Results can be emitted as on-chain events so CI pipelines can track
performance regressions over time.

## Security Assumptions

1. **Read-only** — No function writes to contract storage.
2. **No auth required** — Profiling is permissionless.
3. **Deterministic** — Same inputs always produce the same output.
4. **Overflow-safe** — All arithmetic uses `saturating_*` operations.
5. **Bounded** — Sample counts are bounded by `MAX_SAMPLES` (1 000).

## Constants

| Constant | Value | Description |
|:---------|:------|:------------|
| `MAX_SAMPLES` | `1_000` | Maximum samples per `ProfileReport` |
| `BUDGET_INSTRUCTION_LIMIT` | `100_000_000` | Soroban instruction budget ceiling |

## Types

### `ProfileSample`

```rust
pub struct ProfileSample {
    pub label: &'static str,
    pub instructions: u64,
    pub memory_bytes: u64,
}
```

`is_within_budget()` returns `true` when `instructions <= BUDGET_INSTRUCTION_LIMIT`.

### `ProfileReport`

```rust
pub struct ProfileReport {
    pub samples: Vec<ProfileSample>,
    pub total_instructions: u64,
    pub peak_memory_bytes: u64,
}
```

Key methods:

| Method | Description |
|:-------|:------------|
| `add_sample(sample)` | Appends a sample (no-op when at `MAX_SAMPLES`) |
| `sample_count()` | Number of stored samples |
| `average_instructions()` | Mean instruction count; `0` for empty report |
| `budget_utilization_bps()` | Utilization in bps (0–10 000), capped at 10 000 |

## Functions

| Function | Description |
|:---------|:------------|
| `profile_operation(label, instructions, memory_bytes)` | Constructs a `ProfileSample` |
| `check_budget(sample)` | Returns `true` when sample is within budget |
| `emit_profile_event(env, label, instructions)` | Emits a `"profile"` event on-chain |

## Budget Utilization

```
utilization_bps = (total_instructions * 10_000) / BUDGET_INSTRUCTION_LIMIT
```

Capped at `10_000` bps (100%) even when total instructions exceed the limit.

## Test Coverage

The test suite (`performance_profiling.test.rs`) covers:

- `ProfileSample::new` — field values
- `is_within_budget` — at limit (pass), over limit (fail), zero
- `ProfileReport::new` — empty state
- `add_sample` — single sample, MAX_SAMPLES cap, peak memory tracking
- `average_instructions` — empty (0), two-sample average
- `budget_utilization_bps` — zero, full, capped, half
- `profile_operation` — correct fields
- `check_budget` — pass and fail
- `emit_profile_event` — no panic

Target: ≥ 95% of all code paths covered.

## Related Files

- [`contracts/crowdfund/src/performance_profiling.rs`](../contracts/crowdfund/src/performance_profiling.rs) — Module
- [`contracts/crowdfund/src/performance_profiling.test.rs`](../contracts/crowdfund/src/performance_profiling.test.rs) — Tests
- [`contracts/crowdfund/src/lib.rs`](../contracts/crowdfund/src/lib.rs) — Module registration
