# `stellar_token_minter` — Bounded NFT reward minting

## Overview

The [`stellar_token_minter.rs`](./src/stellar_token_minter.rs) module defines **constants and pure helpers** for the crowdfund contract’s optional post-`withdraw` NFT reward flow. After a successful campaign, `withdraw` may invoke an external NFT contract’s `mint` entrypoint for each contributor with a positive balance, **up to [`MAX_NFT_MINT_BATCH`](./src/stellar_token_minter.rs)** per call.

`lib.rs` re-exports `MAX_NFT_MINT_BATCH` for backward compatibility (`use crate::MAX_NFT_MINT_BATCH`).

## Issue fixed (tests)

Previously, two test files existed on disk but were **not declared in `lib.rs`**:

- `stellar_token_minter_test.rs`
- `withdraw_event_emission_test.rs`

As a result, **their tests never ran** in `cargo test`. All coverage for NFT batch caps, events, and related edge cases is now consolidated in **[`stellar_token_minter.test.rs`](./src/stellar_token_minter.test.rs)** and wired with:

```rust
#[cfg(test)]
#[path = "stellar_token_minter.test.rs"]
mod stellar_token_minter_test;
```

## API (Rust module)

| Item | Description |
|------|-------------|
| `MAX_NFT_MINT_BATCH` | `u32 = 50` — max `mint` invocations per `withdraw` |
| `NFT_MINT_FN_NAME` | `"mint"` — must match `Symbol::new` in `lib.rs` |
| `mint_batch_is_full(minted_so_far)` | `true` when the loop must stop |
| `bump_token_id_after_mint(current)` | Next sequential `token_id` (saturating) |

## Cross-contract `mint` ABI

`withdraw` uses `invoke_contract` with **two arguments**: `(to, token_id)`. The minimal `NftContract` trait in this crate only types a one-argument `mint` for convenience; **integrations must implement the two-argument ABI** expected by `lib.rs`.

## Security assumptions

1. **`MAX_NFT_MINT_BATCH`** bounds cross-contract calls and loop work per `withdraw`.
2. **Sequential `token_id`** starts at `1`; `bump_token_id_after_mint` uses `saturating_add` on `u64`.
3. **External NFT contract** is untrusted; a malicious contract can still revert or burn gas — only use audited WASM.

## Tests

See [`stellar_token_minter.test.rs`](./src/stellar_token_minter.test.rs):

- Unit tests for helpers and constants.
- Integration tests for mint cap, batch vs summary events, and `withdrawn` event count (formerly in `withdraw_event_emission_test.rs`).
- Crowdfund guards: `collect_pledges`, bonus goal bps, `get_stats`, upgrade auth (formerly in `stellar_token_minter_test.rs`).

### Coverage

```bash
cargo llvm-cov -p crowdfund --lcov --output-path /tmp/crowdfund.lcov
```

On a representative run, **`stellar_token_minter.rs` line coverage (LCOV DA) was 100%**, meeting the ≥ 95% target for this module.

## Related docs

- [Bounded `withdraw()` event emission](./withdraw_event_emission.md) — historical design notes for batch events.
