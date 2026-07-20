# Admin Upgrade Mechanism

## Overview

The admin upgrade mechanism allows the contract admin to replace the deployed
WASM binary without changing the contract address or losing stored state.
Only the address stored as `Admin` during `initialize()` may call `upgrade()`.

## Security Assumptions

1. **Admin auth required** — `upgrade()` calls `require_auth()` on the stored
   admin address. Any transaction not signed by that address is rejected.
2. **Single admin** — The admin is set once at `initialize()` and cannot be
   changed without a separate governance mechanism.
3. **State preserved** — `update_current_contract_wasm` replaces only the
   executable code; all instance storage (goal, deadline, contributions, etc.)
   persists across upgrades.
4. **Irreversible** — Once a new WASM hash is applied the previous binary is
   no longer active. Test the new binary on testnet before upgrading mainnet.
5. **WASM hash integrity** — The 32-byte hash must correspond to a binary
   already uploaded via `stellar contract install`. Passing an unknown hash
   will cause the host to reject the call.

## API

### `validate_admin_upgrade(env) -> Address`

Reads the `Admin` key from instance storage and calls `require_auth()`.
Panics with `"Admin not initialized"` if called before `initialize()`.

### `perform_upgrade(env, new_wasm_hash)`

Delegates to `env.deployer().update_current_contract_wasm(new_wasm_hash)`.

### `upgrade(env, new_wasm_hash)` _(contract entry point)_

Calls `validate_admin_upgrade`, then `perform_upgrade`, then emits an
`("upgrade", admin)` event with the new WASM hash as the event data.

## Upgrade Procedure

```bash
# 1. Build the new binary
cargo build --release --target wasm32-unknown-unknown -p crowdfund

# 2. Upload and get the WASM hash
stellar contract install \
  --wasm target/wasm32-unknown-unknown/release/crowdfund.wasm \
  --source <ADMIN_SECRET> \
  --network testnet

# 3. Invoke upgrade
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET> \
  --network testnet \
  -- upgrade \
  --new_wasm_hash <WASM_HASH>
```

## Recommendation

Require at least two reviewers to approve upgrade PRs before merging to
production. The admin key for mainnet should be a multisig account.

# admin_upgrade_mechanism

Admin-gated WASM upgrade validation for the Stellar Raise crowdfund contract.

## Overview

This module handles the two validation steps that must pass before a contract upgrade is executed:

1. **Admin authorization** — only the address stored as `Admin` during `initialize()` may call `upgrade()`.
2. **WASM hash validation** — the supplied hash must be non-zero; an all-zero hash indicates a missing or malformed upload.

## Public API

### `validate_admin_upgrade(env) -> Address`

Reads `DataKey::Admin` from instance storage and calls `require_auth()` on it.
**Panics** if no admin is stored (contract not initialized).

### `validate_wasm_hash(new_wasm_hash)`

Asserts the 32-byte hash is not all zeros.
**Panics** with `"upgrade: wasm_hash must not be zero"` if the hash is `[0u8; 32]`.

### `perform_upgrade(env, new_wasm_hash)`

Calls `env.deployer().update_current_contract_wasm(new_wasm_hash)` to swap the WASM.
Only called after both validation steps pass.

## Upgrade Flow

```
upgrade(env, new_wasm_hash, current_wasm_hash)
  │
  ├─ validate_admin_upgrade(env)       → panics if not admin
  ├─ store_current_wasm_hash(env, cur) → persists current hash as rollback point
  ├─ perform_upgrade(env, new)         → swaps WASM
  └─ env.events().publish(...)         → emits upgrade event (old hash, new hash)
```

## Rollback Flow

```
rollback_upgrade(env)
  │
  ├─ validate_admin_upgrade(env)           → panics if not admin
  ├─ rollback_upgrade(env)                 → reads PreviousWasmHash, restores it
  └─ env.events().publish(...)             → emits rollback event with restored hash
```

## Edge Cases

| Input                                | Outcome                                                           |
| ------------------------------------ | ---------------------------------------------------------------- |
| Non-admin caller                     | Rejected by `require_auth()`                                     |
| Creator (≠ admin)                    | Rejected by `require_auth()`                                     |
| No admin stored (pre-init)           | Panics on `expect("Admin not initialized")`                      |
| All-zero WASM hash                   | Panics with `"upgrade: wasm_hash must not be zero"`              |
| Valid hash, valid admin              | Upgrade proceeds                                                  |
| **Rollback with no previous hash**   | **Panics with `"No previous WASM hash available for rollback"`** |
| **Rollback after a bad upgrade**     | **Restores the previous working WASM — contract is un-bricked**  |

## Security Considerations

- **Irreversibility (mitigated)**: Upgrades can now be **rolled back** to the previously stored WASM hash via `rollback_upgrade()`. The previous hash is stored atomically before the new WASM is applied.
- **Admin key custody**: the admin address is set once at `initialize()` and cannot be changed without an upgrade.
- **State persistence**: all contract storage survives a WASM swap — the upgrade only replaces executable code.
- **Rollback prerequisite**: A `current_wasm_hash` must be provided to `upgrade()` to establish a rollback point. Without it, no rollback is possible.
- **Recommendation**: require at least two reviewers to approve upgrade PRs before merging.

## Rollback Procedure

```bash
# 1. If a bad upgrade was applied, invoke rollback
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET> \
  --network testnet \
  -- rollback_upgrade
```

## API Changes

### `upgrade(env, new_wasm_hash, current_wasm_hash)`

**New parameter**: `current_wasm_hash` — The SHA-256 hash of the currently deployed WASM binary. This is stored as `DataKey::PreviousWasmHash` in instance storage **before** the new WASM is applied, creating a rollback point.

### `rollback_upgrade(env) -> BytesN<32>` _(new entry point)_

Reads `DataKey::PreviousWasmHash` from instance storage and restores it via `update_current_contract_wasm()`. Emits a `("crowdfund", "rollback")` event with the restored hash. Only callable by the admin.

Panics with `"No previous WASM hash available for rollback"` if called before any `upgrade()` has been performed.

### `store_current_wasm_hash(env, hash)` _(internal helper)_

Persists the given hash under `DataKey::PreviousWasmHash` in instance storage. Called by `upgrade()` before applying the new WASM.

### `get_previous_wasm_hash(env) -> Option<BytesN<32>>` _(internal view)_

Returns the stored previous WASM hash, or `None` if no upgrade has been performed yet.
# Admin Upgrade Mechanism

## Overview

The admin upgrade mechanism allows the contract admin to replace the deployed
WASM binary without changing the contract address or losing stored state.
Only the address stored as `Admin` during `initialize()` may call `upgrade()`.

## Security Assumptions

1. **Admin auth required** — `upgrade()` calls `require_auth()` on the stored
   admin address. Any transaction not signed by that address is rejected.
2. **Single admin** — The admin is set once at `initialize()` and cannot be
   changed without a separate governance mechanism.
3. **State preserved** — `update_current_contract_wasm` replaces only the
   executable code; all instance storage (goal, deadline, contributions, etc.)
   persists across upgrades.
4. **Irreversible** — Once a new WASM hash is applied the previous binary is
   no longer active. Test the new binary on testnet before upgrading mainnet.
5. **WASM hash integrity** — The 32-byte hash must correspond to a binary
   already uploaded via `stellar contract install`. Passing an unknown hash
   will cause the host to reject the call.

## API

### `validate_admin_upgrade(env) -> Address`

Reads the `Admin` key from instance storage and calls `require_auth()`.
Panics with `"Admin not initialized"` if called before `initialize()`.

### `perform_upgrade(env, new_wasm_hash)`

Delegates to `env.deployer().update_current_contract_wasm(new_wasm_hash)`.

### `upgrade(env, new_wasm_hash)` _(contract entry point)_

Calls `validate_admin_upgrade`, then `perform_upgrade`, then emits an
`("upgrade", admin)` event with the new WASM hash as the event data.

## Upgrade Procedure

```bash
# 1. Build the new binary
cargo build --release --target wasm32-unknown-unknown -p crowdfund

# 2. Upload and get the WASM hash
stellar contract install \
  --wasm target/wasm32-unknown-unknown/release/crowdfund.wasm \
  --source <ADMIN_SECRET> \
  --network testnet

# 3. Invoke upgrade
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET> \
  --network testnet \
  -- upgrade \
  --new_wasm_hash <WASM_HASH>
```

## Recommendation

Require at least two reviewers to approve upgrade PRs before merging to
production. The admin key for mainnet should be a multisig account.
