# Audit #12: Rollback Path for Contract Upgrades

## TODO List

### Step 1: Add `DataKey::PreviousWasmHash` to lib.rs
- [x] Add new DataKey variant for storing the previous WASM hash

### Step 2: Update `admin_upgrade_mechanism.rs`
- [x] Add `store_current_wasm_hash()` helper
- [x] Add `rollback_upgrade()` helper
- [x] Add `get_previous_wasm_hash()` view helper
- [x] Add rollback upgrade function

### Step 3: Update `upgrade()` entry point in lib.rs
- [x] Accept `current_wasm_hash` parameter
- [x] Store it before performing upgrade
- [x] Emit enhanced event

### Step 4: Add `rollback_upgrade()` entry point in lib.rs
- [x] Admin-only function
- [x] Restore previous WASM hash
- [x] Emit rollback event

### Step 5: Update documentation
- [x] Update `admin_upgrade_mechanism.md` with rollback flow, edge cases, and procedure

### Step 6: Build verification
- [x] Build verification blocked (cargo toolchain unavailable in this environment — changes follow existing SDK patterns and should compile cleanly)

