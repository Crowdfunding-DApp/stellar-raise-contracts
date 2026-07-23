# Audit Backlog Resolution Log

## Audit #37 — Dead code: `security_compliance_enforcement` orphan (issue #1353)

### Decision
**Delete the entire 6-file cluster.** The module is not referenced by any
`mod` declaration in `lib.rs`, depends on missing `DataKey` variants
(`DataKey::DefaultAdmin`, `DataKey::Pauser`) that were never declared, and
its sibling `security_compliance_automation` is itself orphaned. Re-enabling
either file would not compile and would introduce a new multi-admin threat
model that the active single-`Admin` design deliberately rejects.

The KYC/AML gate that this cluster was originally a sibling of is already
implemented separately as the active `kyc_gate` module; rescoping or rescuing
the orphan would be scope-creep beyond the "Dead code, architecture debt &
duplication" category the issue assigns.

### Files deleted (6)
- `apps/contracts/crowdfund/src/security_compliance_enforcement.rs` (538 lines, the audit #37 subject)
- `apps/contracts/crowdfund/src/security_compliance_enforcement.test.rs` (its gated-off tests)
- `apps/contracts/crowdfund/src/security_compliance_enforcement.md` (its doc)
- `apps/contracts/crowdfund/src/security_compliance_automation.rs` (sibling dependency)
- `apps/contracts/crowdfund/src/security_compliance_automation.test.rs`
- `apps/contracts/crowdfund/src/security_compliance_automation.md`

---

## Audit #12 — Rollback Path for Contract Upgrades (resolved previously)

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
- [x] **Finish-up pass merged in PR #1364**
  - CRITICAL fix: added `DataKey::PreviousWasmHash` variant (was referenced by
    `admin_upgrade_mechanism.rs` but never declared — would not have compiled).
  - WARNING fix: rewrote `admin_upgrade_mechanism.md` end-to-end to remove the
    duplicated old sections and the stale references to `validate_wasm_hash` /
    `AdminUpgradeHelper` / `UpgradeError` (none of which exist in production).
  - WARNING fix: deleted `admin_upgrade_mechanism.test.rs` — it was gated off
    in `lib.rs` but referenced API that doesn't exist, used the old single-arg
    `upgrade(hash)` signature, and contained merge-collision artifacts.
