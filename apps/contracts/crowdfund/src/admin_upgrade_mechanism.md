# admin_upgrade_mechanism

Admin-gated WASM upgrade and rollback validation for the Stellar Raise
crowdfund contract.

## Overview

The admin upgrade mechanism lets the contract admin replace the deployed
WASM binary **without changing the contract address or losing stored state**.
Each successful `upgrade()` records the hash it is replacing under
`DataKey::PreviousWasmHash` so the admin can restore it via `rollback_upgrade()`
if the new WASM turns out to be broken, mis-laid-out, or otherwise bad.

The mechanism performs one validation step on every call:

1. **Admin authorization** — only the address stored as `Admin` during
   `initialize()` may call `upgrade()` or `rollback_upgrade()`.

## Public API

### `validate_admin_upgrade(env) -> Address`

Reads `DataKey::Admin` from instance storage and calls `require_auth()` on
it. **Panics** with `"Admin not initialized"` if no admin is stored (contract
not initialized). Returns the authenticated admin address.

### `store_current_wasm_hash(env, current_wasm_hash)`

Persists `current_wasm_hash` under `DataKey::PreviousWasmHash` in instance
storage. Called by `upgrade()` *before* the new WASM is applied so that
`rollback_upgrade()` always has a valid rollback point.

### `get_previous_wasm_hash(env) -> Option<BytesN<32>>`

Returns the stored rollback hash, or `None` if no `upgrade()` has ever been
performed. Read-only view helper.

### `perform_upgrade(env, new_wasm_hash)`

Calls `env.deployer().update_current_contract_wasm(new_wasm_hash)` to swap
the WASM. Only called after `validate_admin_upgrade()` succeeds.

### `rollback_upgrade(env) -> BytesN<32>`

Reads `DataKey::PreviousWasmHash` and restores it via
`update_current_contract_wasm()`. Only callable by the admin. **Panics** with
`"No previous WASM hash available for rollback"` if called before any
`upgrade()` has been performed.

## Upgrade Flow

```
upgrade(env, new_wasm_hash, current_wasm_hash)
  │
  ├─ validate_admin_upgrade(env)       → panics if not admin
  ├─ store_current_wasm_hash(env, cur) → persists current hash as rollback point
  ├─ perform_upgrade(env, new)         → swaps WASM
  └─ env.events().publish(...)         → emits ("crowdfund","upgrade") with
                                            (admin, current, new)
```

## Rollback Flow

```
rollback_upgrade(env)
  │
  ├─ validate_admin_upgrade(env)           → panics if not admin
  ├─ rollback_upgrade(env)                 → reads PreviousWasmHash, restores it
  └─ env.events().publish(...)             → emits ("crowdfund","rollback") with
                                                (admin, restored_hash)
```

## Edge Cases

| Input                                | Outcome                                                           |
| ------------------------------------ | ------------------------------------------------------------------ |
| Non-admin caller                     | Rejected by `require_auth()`                                       |
| Creator (≠ admin)                    | Rejected by `require_auth()`                                       |
| No admin stored (pre-init)           | Panics on `"Admin not initialized"`                                |
| Valid hash, valid admin              | Upgrade proceeds; `current` is stored as rollback point            |
| `rollback_upgrade` with no prior upgrade | Panics on `"No previous WASM hash available for rollback"`     |
| `rollback_upgrade` after a bad upgrade   | Restores the previous working WASM — contract is un-bricked     |

> **Note** The contract does **not** validate the `new_wasm_hash` argument
> itself. Robust hashing, host-side rejection of unknown hashes, and
> all-zero checks are the host's responsibility once the WASM reaches
> `update_current_contract_wasm`. Operators should verify hashes off-chain
> before invoking `upgrade()`.

## Security Considerations

- **Irreversibility (mitigated)**: Upgrades can be rolled back to the
  previously stored WASM hash via `rollback_upgrade()`. The previous hash
  is stored atomically *before* the new WASM is applied.
- **Admin key custody**: the admin address is set once at `initialize()`
  and cannot be changed without an upgrade.
- **State persistence**: all contract storage survives a WASM swap —
  the upgrade only replaces executable code.
- **Rollback prerequisite**: A `current_wasm_hash` must be provided to
  `upgrade()` to establish a rollback point. Without it, no rollback is
  possible.
- **Single rollback hop**: `PreviousWasmHash` is overwritten on every
  `upgrade()`. Once a second upgrade has been applied, the hash that was
  live *before the first* upgrade is no longer recoverable from chain.
- **Recommendation**: require at least two reviewers to approve upgrade
  PRs before merging, and deploy upgrade candidates to testnet first.

## Procedures

### Upgrade

```bash
# 1. Build the new binary
cargo build --release --target wasm32-unknown-unknown -p crowdfund

# 2. Upload and capture the WASM hash
stellar contract install \
  --wasm target/wasm32-unknown-unknown/release/crowdfund.wasm \
  --source <ADMIN_SECRET> \
  --network testnet

# 3. Invoke upgrade, supplying both the new hash AND the currently
#    deployed hash (the latter is recorded as the rollback point).
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET> \
  --network testnet \
  -- upgrade \
  --new_wasm_hash <NEW_WASM_HASH> \
  --current_wasm_hash <CURRENT_WASM_HASH>
```

### Rollback

```bash
# If a bad upgrade was applied, restore the previous WASM.
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET> \
  --network testnet \
  -- rollback_upgrade
```

## API Reference

### `upgrade(env, new_wasm_hash, current_wasm_hash)` _(contract entry point)_

Admin-only. Stores `current_wasm_hash` under `DataKey::PreviousWasmHash`,
then calls `perform_upgrade(env, new_wasm_hash)`. Emits a
`("crowdfund", "upgrade")` event with payload
`(admin, current_wasm_hash, new_wasm_hash)`.

### `rollback_upgrade(env) -> BytesN<32>` _(contract entry point)_

Admin-only. Reads `DataKey::PreviousWasmHash` and restores it via
`update_current_contract_wasm()`. Emits a `("crowdfund", "rollback")` event
with payload `(admin, restored_hash)`. Returns the restored hash. **Panics**
with `"No previous WASM hash available for rollback"` if called before any
`upgrade()` has been performed.
