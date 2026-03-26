# withdraw_event_emission

Bounded `withdraw()` event emission for the Stellar Raise crowdfund contract.

## Overview

This module improves the security, testability, and performance of `withdraw()` by:

1. **Bounding NFT minting** — caps mints at `MAX_NFT_MINT_BATCH` (50) per call, preventing unbounded gas consumption with large contributor lists.
2. **Single summary event** — emits one `nft_batch_minted` event instead of one per contributor (O(1) vs O(n)).
3. **Security-guarded emit helpers** — each helper asserts its inputs are valid before publishing, making invalid states impossible to emit silently.
4. **Deprecated old logic** — `emit_withdrawal_event` is kept as a thin wrapper for call-site compatibility but delegates to `emit_withdrawn`. New code should call `emit_withdrawn` directly.

## Public API

### `mint_nfts_in_batch(env, nft_contract) -> u32`

Mints NFTs to eligible contributors up to `MAX_NFT_MINT_BATCH`. Returns the count minted.
Emits `("campaign", "nft_batch_minted")` only when count > 0.

### `emit_fee_transferred(env, platform, fee)`

Publishes `("campaign", "fee_transferred")` with `(Address, i128)`.
**Panics** if `fee <= 0`.

### `emit_nft_batch_minted(env, minted_count)`

Publishes `("campaign", "nft_batch_minted")` with `u32` count.
**Panics** if `minted_count == 0`.

### `emit_withdrawn(env, creator, creator_payout, nft_minted_count)`

Publishes `("campaign", "withdrawn")` with `(Address, i128, u32)`.
**Panics** if `creator_payout <= 0`.

### `emit_withdrawal_event(env, creator, payout, nft_minted_count)` _(deprecated)_

Thin wrapper around `emit_withdrawn`. Kept for backwards compatibility.
Prefer `emit_withdrawn` in new code.

## Events Reference

| Topic 1    | Topic 2             | Data                        | When emitted                        |
|------------|---------------------|-----------------------------|-------------------------------------|
| `campaign` | `withdrawn`         | `(Address, i128, u32)`      | Every successful `withdraw()` call  |
| `campaign` | `fee_transferred`   | `(Address, i128)`           | When platform fee is configured     |
| `campaign` | `nft_batch_minted`  | `u32`                       | When ≥1 NFT is minted               |

> **Breaking change**: The `withdrawn` event now carries a third field (`nft_minted_count: u32`).
> Off-chain indexers decoding the old `(Address, i128)` tuple must be updated.

## Security Considerations

- **Reentrancy**: `TotalRaised` is zeroed before NFT minting and event emission, following checks-effects-interactions.
- **Overflow**: Fee calculation uses `checked_mul` / `checked_div`; payout uses `checked_sub`.
- **Authorization**: `creator.require_auth()` is called before any transfer.
- **Batch cap**: Contributors beyond `MAX_NFT_MINT_BATCH` are not permanently skipped — a subsequent call (if the contract is upgraded to allow it) can mint the remainder.
- **Input guards**: All emit helpers assert positive values before publishing. A zero or negative fee/payout indicates a logic error upstream and must not reach indexers.

## Deprecation: Old Logic Removed

The previous implementation scattered `env.events().publish()` calls inline within `withdraw()` and emitted one `nft_minted` event per contributor. This has been replaced by:

- Centralised helpers in this module (easier to unit-test in isolation).
- A single `nft_batch_minted` summary event (O(1) gas regardless of contributor count).
- `emit_withdrawal_event` retained as a deprecated shim for any existing call sites.

## Usage

```rust
use crate::withdraw_event_emission::{emit_fee_transferred, emit_withdrawn, mint_nfts_in_batch};

// Inside withdraw():
let nft_contract: Option<Address> = env.storage().instance().get(&DataKey::NFTContract);
let nft_minted_count = mint_nfts_in_batch(&env, &nft_contract);
emit_withdrawn(&env, &creator, creator_payout, nft_minted_count);
```

## Test Coverage

See `withdraw_event_emission_test.rs` for the full suite. Key scenarios:

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_mints_all_when_within_cap` | All NFTs minted when count < cap |
| `test_withdraw_caps_minting_at_max_batch` | Minting stops at `MAX_NFT_MINT_BATCH` |
| `test_withdraw_mints_exactly_at_cap_boundary` | Boundary condition at exactly cap |
| `test_withdraw_emits_single_batch_event` | Only one `nft_batch_minted` event emitted |
| `test_withdraw_no_batch_event_without_nft_contract` | No event when NFT contract absent |
| `test_withdraw_emits_withdrawn_event_once` | Exactly one `withdrawn` event |
| `test_withdrawn_event_payout_reflects_fee_deduction` | Payout = total - fee |
| `test_fee_transferred_event_data_includes_fee_amount` | Fee amount correct in event data |
| `test_double_withdraw_panics` | Second withdraw blocked by status guard |
| `test_emit_fee_transferred_panics_on_zero_fee` | Security: zero fee rejected |
| `test_emit_withdrawn_panics_on_zero_payout` | Security: zero payout rejected |
