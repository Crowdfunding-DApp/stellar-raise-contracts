# PR: Add Logging Bounds to Admin Upgrade Mechanism Validation for Security

## Branch
`feature/add-logging-bounds-to-admin-upgrade-mechanism-validation-for-security`

## Summary

The existing `upgrade()` function in `contracts/crowdfund/src/lib.rs` performs no input validation and emits no on-chain events. This means any admin-triggered WASM swap is completely silent — undetectable by indexers, monitoring tools, or auditors after the fact. This PR closes that gap by adding input bounds validation and structured audit logging to the upgrade path.

## Problem

```rust
// Before — no validation, no logging
pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
```

- A zeroed `BytesN<32>` is silently accepted, which would brick the contract.
- No event is emitted, so there is no on-chain record of who triggered the upgrade or what hash was deployed.

## Changes

### `contracts/crowdfund/src/admin_upgrade_mechanism.rs` (new)
Isolated module containing the hardened upgrade logic:
- Zero-hash guard — panics with `"upgrade: wasm hash must not be zero"` if `new_wasm_hash == [0u8; 32]`.
- Pre-upgrade audit event emitted via `env.events().publish(("admin", "upgrade_initiated"), (admin, new_wasm_hash))` before `update_current_contract_wasm` is called, ensuring the event is always recorded regardless of what the new WASM does.

### `contracts/crowdfund/src/admin_upgrade_mechanism.test.rs` (new)
Comprehensive test coverage:
- `test_upgrade_emits_audit_event` — verifies the `upgrade_initiated` event is published with the correct admin address and wasm hash.
- `test_upgrade_rejects_zero_hash` — asserts the zero-hash guard panics as expected.
- `test_upgrade_requires_admin_auth` — confirms `require_auth()` is enforced and non-admin callers are rejected.

### `docs/admin-upgrade-logging-bounds.md` (new)
Developer-facing reference covering:
- Validation bounds table (zero-hash guard, admin auth).
- Audit event schema (`topic`, `data` fields).
- Security rationale for each control.
- Annotated code example.

## Security Controls Added

| Control | Location | Behaviour |
|---|---|---|
| Zero-hash guard | `upgrade()` | Panics — rejects null/unset WASM hashes |
| Admin `require_auth()` | `upgrade()` | Rejects any caller that is not the stored `Admin` address |
| Pre-upgrade audit event | `upgrade()` | Emits `("admin", "upgrade_initiated")` with admin address + hash before swap |

## Testing

```bash
cargo test -p crowdfund admin_upgrade
```

All three new tests pass. Existing auth tests in `auth_tests.rs` are unaffected.

## Checklist

- [x] Input bounds validated (zero-hash guard)
- [x] Audit event emitted before upgrade executes
- [x] Admin `require_auth()` enforced
- [x] Unit tests cover happy path, zero-hash rejection, and unauthorized caller
- [x] Documentation added in `docs/admin-upgrade-logging-bounds.md`
- [x] No breaking changes to existing contract interface
