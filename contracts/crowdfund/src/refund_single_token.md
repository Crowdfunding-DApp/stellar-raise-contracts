# `refund_single_token` Module

## Overview

`refund_single_token.rs` is the security-sensitive core behind the public
`refund_single()` contract method. It splits the flow into three helpers:

- `validate_refund_preconditions()`: read-only eligibility checks
- `execute_refund_single()`: state mutation plus transfer
- `refund_single_transfer()`: fixed-direction token transfer wrapper

The public entrypoint in [`lib.rs`](./lib.rs) remains intentionally small:

```rust
pub fn refund_single(env: Env, contributor: Address) -> Result<(), ContractError> {
    contributor.require_auth();
    let amount = validate_refund_preconditions(&env, &contributor)?;
    execute_refund_single(&env, &contributor, amount)
}
```

## Why this split exists

Separating validation from execution makes the critical refund path easier to
reason about and easier to test:

- preconditions can be exercised without mutating storage
- arithmetic can be validated before any effects are committed
- the token transfer direction is centralized in one helper

## Refund lifecycle

1. The contributor authenticates through `refund_single()`.
2. `validate_refund_preconditions()` checks:
   - campaign is not `Successful` or `Cancelled`
   - deadline has passed
   - funding goal was not reached
   - contributor has a non-zero recorded balance
3. `execute_refund_single()`:
   - rejects `amount > total_raised` and then computes `new_total_raised`
   - zeroes the contributor record
   - stores the reduced `total_raised`
   - transfers tokens from contract to contributor
   - emits `("campaign", "refund_single")`

## Validated security assumptions

The following assumptions are explicitly covered by
[`refund_single_token.test.rs`](./refund_single_token.test.rs):

| Assumption | Enforcement | Representative test |
|------------|-------------|---------------------|
| Only the contributor can claim | `contributor.require_auth()` in the public entrypoint | `test_refund_single_requires_contributor_auth` |
| Refunds are unavailable while active | `timestamp <= deadline` returns `CampaignStillActive` | `test_validate_before_deadline_returns_campaign_still_active` |
| Refunds are unavailable after success | `total_raised >= goal` returns `GoalReached` | `test_validate_goal_met_returns_goal_reached` |
| No double-claim | contribution storage is zeroed before transfer | `test_refund_single_double_claim_returns_nothing_to_refund` |
| No partial mutation on arithmetic failure | `amount > total_raised` is rejected before writes | `test_execute_overflow_preserves_state_and_balance` |
| Contributor isolation | only the targeted balance and `total_raised` change | `test_execute_transfers_tokens_and_updates_state` |
| Off-chain observability | one refund event per successful claim | `test_execute_emits_refund_event_once` |

## Runtime assumptions

This module still relies on normal Soroban call semantics:

- if the downstream token transfer fails, the enclosing invocation reverts
- the token configured at initialization is a trustworthy Stellar asset contract

Those are environment-level assumptions rather than logic inside this module.

## Error behavior

| Outcome | Meaning |
|---------|---------|
| `CampaignStillActive` | deadline has not passed yet |
| `GoalReached` | campaign succeeded, so refunds are not allowed |
| `NothingToRefund` | no stored contribution remains for that contributor |
| `Overflow` | internal accounting is inconsistent with the contribution amount |

`Successful` and `Cancelled` status states intentionally panic with
`"campaign is not active"` to match the broader contract behavior.

## Event

Successful refunds emit:

```text
topics: ("campaign", "refund_single")
data:   (contributor: Address, amount: i128)
```

Indexers should prefer this event over storage polling.

## Storage touched

| Key | Storage class | Behavior |
|-----|---------------|----------|
| `DataKey::Contribution(addr)` | persistent | set to `0` after a successful claim |
| `DataKey::TotalRaised` | instance | decremented by the claimed amount |
| `DataKey::Token` | instance | read to construct the transfer client |

## Relationship to deprecated `refund()`

The batch `refund()` method remains for backward compatibility, but
`refund_single()` is the preferred path because it avoids unbounded iteration
across the contributor list.
