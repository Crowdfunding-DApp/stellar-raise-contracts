# Notes

## How to Implement a New Edge Case for Soroban SDK Minor Version Bump — Gas Efficiency

This guide walks through adding a gas-efficiency edge case to the `soroban_sdk_minor` module
in `contracts/crowdfund/src/soroban_sdk_minor.rs`.

---

### 1. Understand the existing pattern

The module lives at:
- `contracts/crowdfund/src/soroban_sdk_minor.rs` — helpers
- `contracts/crowdfund/src/soroban_sdk_minor.test.rs` — tests

Existing helpers follow this shape:
- A pure function with clear inputs/outputs
- A `/// @notice` / `/// @dev` / `/// # Security` doc block
- A matching test in `soroban_sdk_minor.test.rs`

---

### 2. Add the gas-efficiency helper

Open `soroban_sdk_minor.rs` and add a constant + function. Example for a TTL-extension
gas guard (a common gas-efficiency edge case in minor bumps):

```rust
/// Maximum ledger TTL extension allowed per call to stay within gas budget.
pub const MAX_TTL_EXTENSION: u32 = 500_000;

/// @notice Clamp a requested TTL extension to the gas-safe maximum.
/// @dev    Soroban charges per-ledger for TTL extensions; unbounded values
///         can exhaust the gas budget silently. Clamp before calling
///         `env.storage().instance().extend_ttl(...)`.
/// @param  requested – The caller-supplied extension in ledgers.
/// @return The clamped, gas-safe extension value.
pub fn clamp_ttl_extension(requested: u32) -> u32 {
    requested.min(MAX_TTL_EXTENSION)
}
```

If the edge case involves the `Env`, follow the same signature style as
`assess_compatibility` — take `env: &Env` as the first argument.

---

### 3. Export it (if needed)

If `lib.rs` re-exports from this module, add your new symbol there. Check
`contracts/crowdfund/src/lib.rs` for any explicit `pub use` lines.

---

### 4. Write the test

In `soroban_sdk_minor.test.rs`, import your new symbol and add tests for:
- The happy path (value within bounds)
- The boundary value (exactly at the limit)
- The over-limit case (value exceeds the cap)

```rust
use crate::soroban_sdk_minor::{clamp_ttl_extension, MAX_TTL_EXTENSION};

#[test]
fn ttl_extension_clamps_to_max() {
    assert_eq!(clamp_ttl_extension(MAX_TTL_EXTENSION + 1), MAX_TTL_EXTENSION);
}

#[test]
fn ttl_extension_allows_values_within_limit() {
    assert_eq!(clamp_ttl_extension(1_000), 1_000);
}

#[test]
fn ttl_extension_boundary_is_inclusive() {
    assert_eq!(clamp_ttl_extension(MAX_TTL_EXTENSION), MAX_TTL_EXTENSION);
}
```

---

### 5. Run the tests

```bash
cargo test --package crowdfund soroban_sdk_minor -- --nocapture
```

All existing tests must still pass — the module has no breaking changes between
same-major SDK bumps (`assess_compatibility` returns `Compatible` for same-major).

---

### 6. Update the spec doc

Add a line to `contracts/crowdfund/soroban_sdk_minor.md` under **Implemented updates**:

```
- Added gas-efficiency TTL extension guard:
  - `MAX_TTL_EXTENSION`
  - `clamp_ttl_extension(...)`
```

---

### Key rules to follow

- Keep helpers pure where possible (no `env` mutation, no storage writes).
- Use `saturating_*` or `.min()` / `.clamp()` — never raw arithmetic that can overflow.
- Every public function needs a `@notice` doc comment.
- Same-major SDK bumps must not change storage keys or ABI — verify with `assess_compatibility`.
- Gas-sensitive paths (TTL, event payloads, page sizes) must always be bounded.
