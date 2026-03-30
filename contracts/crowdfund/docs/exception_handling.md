# Exception Handling Module

## Overview

Centralized error handling replaces all `panic!()` with `Result<T, Error>`. Benefits:

- **No unwinds**: Contract fails gracefully, no unsafe state.
- **Auditability**: Every failure path logged via events.
- **Monitoring**: Off-chain tools match error codes to alerts.
- **Reusability**: Shared across crowdfund/factory/security.

## Error Enum

| Code | Variant            | Trigger           | Example                    |
| ---- | ------------------ | ----------------- | -------------------------- |
| 1    | Unauthorized       | Missing auth/role | Wrong caller for upgrade   |
| 2    | InvalidInput       | Bad params        | amount < min, date < now   |
| 3    | NotFound           | Missing storage   | Uninit admin               |
| 4    | Overflow           | Math fail         | total + amount > i128::MAX |
| 5    | AlreadyInitialized | Re-init           | Second initialize()        |
| 6    | StateLimitExceeded | Size limits       | Contributors > 1000        |
| 7    | InvalidState       | Wrong status      | withdraw() on Active       |
| 8    | ZeroValue          | amount == 0       | Zero contribution          |
| 9    | BelowMinimum       | amount < min      | $0.01 < $1 min             |
| 10   | CampaignInactive   | !Active status    | Contribute after deadline  |

## Usage

```rust
use crate::exception_handling::{ensure_auth, invalid_input};

pub fn contribute(...) -> Result<(), Error> {
    contributor.require_auth(); // Keep + helper for clarity
    ensure_auth(&env, &creator)?; // Replaces panic!("not authorized");

    if amount < min {
        return invalid_input(&env, "below min contribution");
    }
    // ...
}
```

## Security Considerations

- **Fail-fast**: Errors prevent partial state changes.
- **Events**: All helpers emit for alerting (e.g. rate-limit InvalidInput).
- **CEI**: Checks before effects; errors revert transfers.
- **No silent fails**: Always explicit Err.

## Testing

95%+ coverage in exception_handling.test.rs. Edge cases: overflows, zero inputs, auth fails.

## Migration from Panic

- "not authorized" → `ensure_auth()?`
- "state size limit exceeded" → `state_limit_exceeded()?`
- Custom → `invalid_state(&env, "reason")?`
