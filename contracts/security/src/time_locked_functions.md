# time_locked_functions

## Overview

`time_locked_functions.rs` adds a reusable on-chain timelock for
security-sensitive operations in Stellar Raise.  The module lets the configured
admin queue privileged actions, wait out a mandatory delay, and then execute or
cancel them within a bounded grace window.  This creates review time for human
operators and automated monitors before impactful actions take effect.

The companion test file (`time_locked_functions.test.rs`) covers configuration
validation, authorization, scheduling, status transitions, cancellation,
execution, event emission, and property-based invariants.  It is written to
target ≥ 95 % line coverage for the module.

---

## Security Assumptions

| Assumption | Enforcement |
|---|---|
| Only the configured admin may queue/cancel/execute | `validate_admin_caller` and `require_auth()` |
| Every privileged action waits at least the configured minimum delay | `validate_delay` and `compute_execute_after` |
| Actions cannot remain executable forever | `derive_status` marks actions `Expired` after `grace_period` |
| Zero or placeholder payload hashes are unsafe | `validate_payload_hash` rejects `[0; 32]` |
| Actions are single-use | `executed_at` and `cancelled_at` are one-way state transitions |
| Misconfiguration must not silently weaken security | `validate_config` rejects unsafe bounds |
| Timestamp arithmetic must not overflow | `compute_execute_after` uses `checked_add` |

---

## Constants

| Constant | Value | Description |
|---|---|---|
| `MIN_ALLOWED_DELAY` | `3_600` | Smallest supported timelock delay (1 hour) |
| `MAX_ALLOWED_DELAY` | `2_592_000` | Largest supported timelock delay (30 days) |
| `MIN_ALLOWED_GRACE_PERIOD` | `3_600` | Smallest supported execution window (1 hour) |
| `MAX_ALLOWED_GRACE_PERIOD` | `604_800` | Largest supported execution window (7 days) |
| `MAX_ACTION_NAME_LEN` | `64` | Maximum allowed action label length |

---

## Data Types

### `TimeLockStatus`
```rust
pub enum TimeLockStatus {
    Pending,
    Ready,
    Executed,
    Cancelled,
    Expired,
}
```

### `TimeLockConfig`
Stores the timelock administrator and the allowed delay / grace-period bounds.

### `TimeLockedAction`
Stores a queued action's ID, proposer, human-readable action name, payload
hash, creation timestamp, execution timestamp, requested delay, and one-way
execution/cancellation timestamps.

---

## Pure Helper Functions

### `validate_config(config) -> Result<(), &'static str>`
Rejects unsafe bounds such as a delay below 1 hour or a grace period above
7 days.

### `validate_admin_caller(config, caller) -> Result<(), &'static str>`
Verifies that the caller matches the configured timelock admin.

### `validate_action_name(action_name) -> Result<(), &'static str>`
Rejects empty or oversized human-readable action labels.

### `validate_payload_hash(payload_hash) -> Result<(), &'static str>`
Rejects an all-zero payload hash.

### `validate_delay(delay, config) -> Result<(), &'static str>`
Ensures a requested delay falls within the configured min/max range.

### `compute_execute_after(now, delay) -> Result<u64, &'static str>`
Returns `now + delay` using checked arithmetic.

### `validate_schedule_request(...) -> Result<u64, &'static str>`
Combines admin, name, hash, delay, and timestamp validation into one gate and
returns the computed ETA.

### `derive_status(action, now, grace_period) -> TimeLockStatus`
Derives the current status of a queued action from its timestamps.

### `validate_cancellation(...) -> Result<(), &'static str>`
Rejects cancellation of already executed or already cancelled actions.

### `validate_execution(...) -> Result<(), &'static str>`
Only permits execution when the action is `Ready`.

---

## Contract Functions

### `initialize(env, admin, min_delay, max_delay, grace_period) -> TimeLockConfig`
Single-use setup for the timelock module.  Requires `admin.require_auth()`.

### `get_config(env) -> Option<TimeLockConfig>`
Returns the stored configuration.

### `action_count(env) -> u64`
Returns the total number of queued actions ever created.

### `schedule_action(env, caller, action_name, payload_hash, delay) -> u64`
Queues a new time-locked action and emits a `(timelock, queued, action_id)`
event with `execute_after` as event data.

### `cancel_action(env, caller, action_id) -> TimeLockedAction`
Cancels a queued action and emits `(timelock, cancel, action_id)`.

### `execute_action(env, caller, action_id) -> TimeLockedAction`
Executes a ready action and emits `(timelock, exec, action_id)`.

### `get_action(env, action_id) -> Option<TimeLockedAction>`
Retrieves a stored queued action.

### `get_status(env, action_id) -> Option<TimeLockStatus>`
Returns the derived runtime status for a queued action.

### `is_action_ready(env, action_id) -> bool`
Convenience helper for readiness checks.

---

## Usage

### Initialize the timelock

```rust
let cfg = SecurityTimeLockedFunctions::initialize(
    env.clone(),
    admin.clone(),
    3_600,   // 1 hour minimum delay
    86_400,  // 24 hour maximum delay
    7_200,   // 2 hour grace period
);
```

### Queue an upgrade

```rust
let action_id = SecurityTimeLockedFunctions::schedule_action(
    env.clone(),
    admin.clone(),
    String::from_str(&env, "upgrade_contract"),
    new_wasm_hash,
    7_200,
);
```

### Execute after the delay

```rust
if SecurityTimeLockedFunctions::is_action_ready(env.clone(), action_id) {
    let action = SecurityTimeLockedFunctions::execute_action(
        env.clone(),
        admin.clone(),
        action_id,
    );
    // apply the real privileged action associated with action.payload_hash
}
```

### Cancel a queued action

```rust
let cancelled = SecurityTimeLockedFunctions::cancel_action(
    env.clone(),
    admin.clone(),
    action_id,
);
assert!(cancelled.cancelled_at > 0);
```

---

## Review Notes

1. The module is intentionally generic: it records `payload_hash` instead of
   executing arbitrary call data directly.  This keeps the timelock small and
   reviewable while still providing strong sequencing guarantees.
2. Event topics are short and stable for indexers: `queued`, `cancel`, `exec`.
3. Status derivation is timestamp-only and deterministic, which simplifies both
   auditing and unit testing.

---

## Suggested Test Commands

```bash
cargo test -p security time_locked_functions
cargo test -p security
```
