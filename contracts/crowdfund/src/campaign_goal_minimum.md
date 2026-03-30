# Campaign Goal Minimum Threshold Enforcement

## Overview
Implements pure validation helpers in `campaign_goal_minimum.rs` for campaign goal (>=1 token unit), min_contribution (>=1), deadline (now+60s), platform_fee (<=10000bps), bonus_goal (>goal). Used in `initialize()` via `crowdfund_initialize_function::execute_initialize`. Computes `progress_bps` for `get_stats()`.

## Purpose
- **Prevent Spam/Abuse**: Blocks zero-goal (instant \"success\" drain) or dust contributions (storage waste).
- **UX**: Clear typed errors (InvalidGoal=8, etc.).
- **Efficiency**: Pure fns (no Env/storage), safe for off-chain/proptests.
- **Security**: Single-pass validation before storage writes.

## Key Constants
| Constant | Value | Purpose |
|----------|-------|---------|
| `MIN_GOAL_AMOUNT` | `1i128` | Prevents zero-goal campaigns |
| `MIN_CONTRIBUTION_AMOUNT` | `1i128` | Blocks dust attacks |
| `MIN_DEADLINE_OFFSET` | `60u64` | Ensures ~1 ledger live time |
| `MAX_PLATFORM_FEE_BPS` | `10_000u32` | Caps fee at 100% |
| `MAX_PROGRESS_BPS` | `10_000u32` | Progress capped at 100.00% |

## Usage
In `initialize()`:
```rust
campaign_goal_minimum::validate_goal_amount(&env, goal)?;  // InvalidGoal if <1
// ... other validates
```

Progress: `get_stats().progress_bps = compute_progress_bps(total_raised, goal);`

## Security Considerations
- **Pure Fns**: No storage/arithmetic overflow risk.
- **Typed Errors**: `ContractError::InvalidGoal` etc. for precise handling.
- **Reinit Guard**: Checks `has(DataKey::Creator)` before auth/validates.
- **Auth**: `creator.require_auth()` post-guard, pre-writes.

## Tests (campaign_goal_minimum.test.rs)
- Constants exact values.
- validate_goal/min_contrib/deadline/platform_fee: happy/exact/below/overflow.
- compute_progress_bps: 0%, partial, 100%, over-cap, zero-goal guard.
- validate_goal_amount typed ContractError.
>95% coverage, incl. security boundaries.

## Integration
- lib.rs: `initialize` → `execute_initialize` → validates → store.
- contribute: Re-enforces min_contrib/deadline/status.
- Edge: bonus_goal > goal or InvalidBonusGoal.

