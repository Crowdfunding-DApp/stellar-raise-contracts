# proptest_generator_boundary — Boundary Conditions Contract

The `ProptestGeneratorBoundary` contract is the single source of truth for all boundary conditions and validation constants used by the Stellar Raise crowdfunding platform's property-based tests and input validation logic.

---

## Rationale

Property-based tests (proptests) can generate a wide range of inputs. Without strictly defined boundaries, tests may:

- Encounter division-by-zero in progress calculations (zero goal).
- Generate unrealistic campaign durations that don't reflect frontend UI behaviour.
- Trigger integer overflow if boundary values are not capped.

By exposing these constants via a contract, off-chain scripts can dynamically retrieve the current platform limits, ensuring consistency between tests and deployment environments.

---

## NatSpec Comment Style

All public functions and section headers use NatSpec-style tags:

| Tag | Meaning |
| :--- | :--- |
| `@title` | Human-readable contract/module name |
| `@notice` | What the function does (user-facing) |
| `@dev` | Implementation detail (developer-facing) |
| `@param` | Parameter description |
| `@return` | Return value description |

Section headers inside `impl` blocks use `///` doc comments (not `//` plain comments) so they appear in generated documentation.

---

## Constants

| Constant | Value | Description |
| :--- | ---: | :--- |
| `DEADLINE_OFFSET_MIN` | `1_000` | Minimum deadline offset in seconds (~17 min) |
| `DEADLINE_OFFSET_MAX` | `1_000_000` | Maximum deadline offset in seconds (~11.5 days) |
| `GOAL_MIN` | `1_000` | Minimum valid goal amount |
| `GOAL_MAX` | `100_000_000` | Maximum goal amount for test generation |
| `MIN_CONTRIBUTION_FLOOR` | `1` | Absolute floor for contribution amounts |
| `PROGRESS_BPS_CAP` | `10_000` | Maximum progress in basis points (100%) |
| `FEE_BPS_CAP` | `10_000` | Maximum fee in basis points (100%) |
| `PROPTEST_CASES_MIN` | `32` | Minimum proptest case count |
| `PROPTEST_CASES_MAX` | `256` | Maximum proptest case count |
| `GENERATOR_BATCH_MAX` | `512` | Maximum batch size for generator operations |

---

## Contract Functions

### Getter Functions

```rust
fn deadline_offset_min(_env) -> u64   // DEADLINE_OFFSET_MIN
fn deadline_offset_max(_env) -> u64   // DEADLINE_OFFSET_MAX
fn goal_min(_env) -> i128             // GOAL_MIN
fn goal_max(_env) -> i128             // GOAL_MAX
fn min_contribution_floor(_env) -> i128  // MIN_CONTRIBUTION_FLOOR
fn progress_bps_cap(_env) -> u32      // PROGRESS_BPS_CAP
fn fee_bps_cap(_env) -> u32           // FEE_BPS_CAP
fn proptest_cases_min(_env) -> u32    // PROPTEST_CASES_MIN
fn proptest_cases_max(_env) -> u32    // PROPTEST_CASES_MAX
fn generator_batch_max(_env) -> u32   // GENERATOR_BATCH_MAX
```

### Validation Functions

```rust
/// Returns true if offset is in [DEADLINE_OFFSET_MIN, DEADLINE_OFFSET_MAX].
fn is_valid_deadline_offset(_env, offset: u64) -> bool

/// Returns true if goal is in [GOAL_MIN, GOAL_MAX].
fn is_valid_goal(_env, goal: i128) -> bool

/// Returns true if min_contribution is in [MIN_CONTRIBUTION_FLOOR, goal].
fn is_valid_min_contribution(_env, min_contribution: i128, goal: i128) -> bool

/// Returns true if amount >= min_contribution.
fn is_valid_contribution_amount(_env, amount: i128, min_contribution: i128) -> bool

/// Returns true if fee_bps <= FEE_BPS_CAP.
fn is_valid_fee_bps(_env, fee_bps: u32) -> bool

/// Returns true if batch_size is in [1, GENERATOR_BATCH_MAX].
fn is_valid_generator_batch_size(_env, batch_size: u32) -> bool
```

### Clamping Functions

```rust
/// Clamps requested to [PROPTEST_CASES_MIN, PROPTEST_CASES_MAX].
fn clamp_proptest_cases(_env, requested: u32) -> u32

/// Clamps raw to [0, PROGRESS_BPS_CAP]. Negative values floor to 0.
fn clamp_progress_bps(_env, raw: i128) -> u32
```

### Derived Calculation Functions

```rust
/// Computes progress in basis points, capped at PROGRESS_BPS_CAP.
/// Returns 0 when goal <= 0 or raised < 0.
/// Uses saturating_mul to prevent overflow.
fn compute_progress_bps(_env, raised: i128, goal: i128) -> u32

/// Computes fee amount from contribution and fee basis points.
/// Returns 0 when amount <= 0 or fee_bps == 0.
fn compute_fee_amount(_env, amount: i128, fee_bps: u32) -> i128

/// Returns Symbol "boundary" for off-chain event filtering.
fn log_tag(_env) -> Symbol
```

---

## Security Model

- **Overflow Protection**: `compute_progress_bps` uses `saturating_mul` before division, preventing i128 overflow on large raised values.
- **Division by Zero**: `compute_progress_bps` guards with `if goal <= 0 { return 0 }` before dividing.
- **Basis Points Capping**: Both `clamp_progress_bps` and `compute_progress_bps` cap at `PROGRESS_BPS_CAP` (10,000) to prevent frontend display errors.
- **Timestamp Validity**: `DEADLINE_OFFSET_MIN` (1,000 s) prevents flaky tests from timing races; `DEADLINE_OFFSET_MAX` (1,000,000 s) prevents u64 overflow when added to ledger timestamps.
- **Resource Bounds**: `PROPTEST_CASES_MAX` and `GENERATOR_BATCH_MAX` prevent accidental CI stress scenarios.

---

## Test Coverage

| Category | Tests | Notes |
| :--- | ---: | :--- |
| Constant sanity checks | 2 | All getters + ordering |
| Deadline offset validation | 2 | Boundary values + edge cases |
| Goal validation | 2 | Boundary values + edge cases |
| Min contribution validation | 2 | Standard + min-goal boundary |
| Contribution amount validation | 1 | |
| Fee bps validation | 1 | |
| Batch size validation | 1 | |
| Clamping functions | 2 | proptest_cases + progress_bps |
| compute_progress_bps | 6 | Basic, edge, overflow, negative, partial, full, over |
| compute_fee_amount | 7 | Basic, edge, floor division, zero/negative, large |
| log_tag | 1 | |
| Property-based tests | 18 | 256 cases each |
| Regression tests | 4 | Known CI failures |
| **Total** | **51** | **≥ 95 % coverage** |

---

## Running Tests

```bash
cargo test --package crowdfund proptest_generator_boundary
```
