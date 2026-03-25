# admin_upgrade_mechanism

Security validation and audit logging for the admin upgrade mechanism.

## Motivation

The original `upgrade` entry-point in `lib.rs` had two gaps:

1. `DataKey::Admin` was never written during `initialize`, so `upgrade` would
   always panic with an unwrap error — making upgrades impossible in practice.
2. No events were emitted around upgrades, leaving no on-chain audit trail for
   indexers or security monitors.

This module closes both gaps and adds admin rotation so the upgrade key can be
transferred without redeploying the contract.

## Public API

### Admin registration

| Function | Description |
|---|---|
| `set_admin(env, admin)` | One-time registration of the admin address. Panics if already set. |
| `get_admin(env)` | Returns the stored admin address. Panics if not set. |
| `admin_is_set(env)` | Returns `true` when an admin has been registered. |
| `is_admin(env, candidate)` | Returns `true` when `candidate` matches the stored admin. |

### Upgrade lifecycle

| Function | Description |
|---|---|
| `validate_upgrade(env, new_wasm_hash)` | Validates admin auth and non-zero hash; emits `("upgrade", "pre")` event. |
| `log_upgrade(env, new_wasm_hash)` | Emits `("upgrade", "done")` event after the WASM swap. |

### Admin rotation

| Function | Description |
|---|---|
| `rotate_admin(env, new_admin)` | Transfers admin rights to `new_admin`; requires current admin auth; emits `("admin", "rotated")` event. |

## Upgrade Flow

```
1. Admin calls upgrade(new_wasm_hash)
   └─ validate_upgrade()        ← checks admin set, hash non-zero, require_auth
   └─ update_current_contract_wasm()
   └─ log_upgrade()             ← emits post-upgrade event
```

## Security Notes

- `set_admin` is a one-time operation. A second call panics, preventing silent
  privilege escalation after deployment.
- A zero WASM hash (`[0u8; 32]`) is rejected by `validate_upgrade` as a likely
  mistake or griefing attempt.
- `rotate_admin` requires the *current* admin to authorise the rotation, so a
  compromised key cannot be silently replaced without the current holder's
  signature.
- All three state-changing operations emit events, giving indexers a complete
  audit trail: admin set → (optional rotations) → pre-upgrade → post-upgrade.
- No new trust assumptions are introduced beyond those in `lib.rs`.

## Test Coverage

Tests live in `admin_upgrade_mechanism_test.rs` and cover:

- `admin_is_set`: absent, present.
- `set_admin`: stores address, panics on second call.
- `get_admin`: panics when absent, returns stored value.
- `is_admin`: no admin set, correct address, wrong address.
- `validate_upgrade`: no admin, zero hash, valid inputs, max hash, single non-zero byte.
- `log_upgrade`: non-zero hash, zero hash (no validation in log path).
- `rotate_admin`: no admin, updates address, old admin no longer matches, same address, twice in sequence.
- Integration: new admin can validate after rotation.
