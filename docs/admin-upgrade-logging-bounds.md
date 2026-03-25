# Admin Upgrade Mechanism — Logging Bounds & Validation

## Overview

The `upgrade` function allows a designated admin to replace the contract's WASM binary in-place. Without proper logging and input bounds, upgrades are silent and unauditable.

## Validation Bounds

| Check | Rule | Behavior |
|---|---|---|
| Zero-hash guard | `new_wasm_hash` must not be all-zero bytes | Panic — rejects null/unset hashes |
| Admin auth | Caller must be the stored `Admin` address | `require_auth()` — rejects unauthorized callers |

## Audit Event

Every successful upgrade attempt emits an on-chain event **before** the WASM swap executes:

```
topic:  ("admin", "upgrade_initiated")
data:   (admin: Address, new_wasm_hash: BytesN<32>)
```

This ensures indexers and monitoring tools can detect upgrades even if the new WASM changes the contract's event schema.

## Security Rationale

- **Zero-hash guard** — a zeroed hash is never a valid WASM binary reference. Accepting it would silently brick the contract.
- **Pre-upgrade event** — emitting before `update_current_contract_wasm` guarantees the event is recorded even if the new code alters or removes the upgrade path.
- **Admin-only auth** — `require_auth()` enforces that only the on-chain registered admin can trigger an upgrade, preventing privilege escalation.

## Example (Rust)

```rust
pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // Bounds: reject zero hash
    let zero_hash = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    if new_wasm_hash == zero_hash {
        panic!("upgrade: wasm hash must not be zero");
    }

    // Audit log before upgrade
    env.events().publish(
        (Symbol::new(&env, "admin"), Symbol::new(&env, "upgrade_initiated")),
        (admin.clone(), new_wasm_hash.clone()),
    );

    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
```
