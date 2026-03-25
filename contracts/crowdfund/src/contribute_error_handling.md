# contribute() Error Handling

## Overview

Deprecates old panic-based guards in `contribute()` and replaces them with
typed `ContractError` variants, enabling scripts and CI/CD pipelines to handle
all error paths programmatically.

## Deprecation Notice

The following untyped panics have been **removed** and replaced:

| Old behaviour                        | New typed error              |
| :----------------------------------- | :--------------------------- |
| `panic!("amount below minimum")`     | `ContractError::BelowMinimum` (code 9) |
| zero-amount pass-through (no guard)  | `ContractError::ZeroAmount` (code 8)   |
| no campaign-status guard             | `ContractError::CampaignNotActive` (code 10) |

## Error Reference

| Code | Variant               | Trigger                                          | Retryable |
| :--- | :-------------------- | :----------------------------------------------- | :-------- |
| 2    | `CampaignEnded`       | `ledger.timestamp > deadline`                    | No        |
| 6    | `Overflow`            | `checked_add` would wrap on contribution totals  | No        |
| 8    | `ZeroAmount`          | `amount == 0`                                    | No        |
| 9    | `BelowMinimum`        | `amount < min_contribution`                      | No        |
| 10   | `CampaignNotActive`   | campaign status is not `Active`                  | No        |

## Security Assumptions

- `contributor.require_auth()` is called before any state mutation.
- Campaign status is checked first — cancelled/successful campaigns are
  rejected before any other validation.
- Token transfer happens before storage writes; failures roll back atomically.
- Overflow is caught with `checked_add` on both per-contributor and global totals.
- The deadline check uses strict `>`, so a contribution at exactly the deadline
  timestamp is **accepted**. Scripts should account for this boundary.

## Usage in Scripts

```rust
use crowdfund::contribute_error_handling::{describe_error, error_codes};

match client.try_contribute(&contributor, &amount) {
    Ok(_) => println!("contributed"),
    Err(Ok(e)) => eprintln!("contract error {}: {}", e as u32, describe_error(e as u32)),
    Err(Err(e)) => eprintln!("host error: {:?}", e),
}
```

## Module Location

`contracts/crowdfund/src/contribute_error_handling.rs`

## Tests

`contracts/crowdfund/src/contribute_error_handling_tests.rs`

17 tests — all passing:

```
contribute_happy_path                                    ok
contribute_accumulates_multiple_contributions            ok
contribute_after_deadline_returns_campaign_ended         ok
contribute_exactly_at_deadline_is_accepted               ok
contribute_below_minimum_returns_typed_error             ok
contribute_one_below_minimum_returns_below_minimum       ok
contribute_zero_amount_returns_typed_error               ok
contribute_to_cancelled_campaign_returns_not_active      ok
contribute_to_successful_campaign_returns_not_active     ok
overflow_error_code_is_correct                           ok
describe_error_campaign_ended                            ok
describe_error_overflow                                  ok
describe_error_zero_amount                               ok
describe_error_below_minimum                             ok
describe_error_campaign_not_active                       ok
describe_error_unknown                                   ok
is_retryable_returns_false_for_all_known_errors          ok
```
