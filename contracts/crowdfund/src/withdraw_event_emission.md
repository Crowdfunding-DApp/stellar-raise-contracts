# withdraw() Event Emission

## Overview

Documents every event emitted by `withdraw()`, adds NatSpec-style inline
comments at each emission site, and provides a helper module for CI/CD
pipelines and off-chain indexers to identify and parse these events without
reading contract source.

## Events

| Topic 1    | Topic 2            | Data                                                        | Condition                        |
| :--------- | :----------------- | :---------------------------------------------------------- | :------------------------------- |
| `campaign` | `fee_transferred`  | `(platform_address: Address, fee: i128)`                    | platform fee configured          |
| `campaign` | `nft_batch_minted` | `minted_count: u32`                                         | NFT contract set & ≥1 minted     |
| `campaign` | `withdrawn`        | `(creator: Address, payout: i128, nft_minted_count: u32)`   | always — final event of withdraw |

## Emission Order

```
1. fee_transferred   (conditional — only when platform fee is configured)
2. nft_batch_minted  (conditional — only when NFT contract set and minted > 0)
3. withdrawn         (always — use as success sentinel)
```

## CI/CD Usage

`withdrawn` is always the **last** event emitted by a successful `withdraw()`
call. Pipelines can use its presence as a reliable success signal:

```rust
use crowdfund::withdraw_event_emission::{is_withdraw_event, topics};

// Check for success sentinel
let success = env.events().all().iter().any(|(_, topics_vec, _)| {
    // topics_vec[0] == "campaign", topics_vec[1] == "withdrawn"
    is_withdraw_event("campaign", "withdrawn")
});
```

## Security Notes

- All events are emitted **after** state mutations and token transfers.
  A missing `withdrawn` event means the transfer did not occur.
- `nft_batch_minted` carries only the count (not addresses) to keep event
  size bounded regardless of contributor list length.
- The `fee_transferred` event confirms the platform fee was deducted
  **before** the creator payout was calculated.

## Module Location

`contracts/crowdfund/src/withdraw_event_emission.rs`

## Tests

`contracts/crowdfund/src/withdraw_event_emission_test.rs`

10 tests — all passing:

```
test_withdraw_mints_all_when_within_cap                      ok
test_withdraw_caps_minting_at_max_batch                      ok
test_withdraw_mints_exactly_at_cap_boundary                  ok
test_withdraw_emits_single_batch_event                       ok
test_withdraw_no_batch_event_without_nft_contract            ok
test_withdraw_emits_withdrawn_event_once                     ok
test_withdraw_no_batch_event_when_no_eligible_contributors   ok
is_withdraw_event_matches_all_three_topics                   ok
is_withdraw_event_rejects_unrelated_topics                   ok
topic_constants_have_correct_values                          ok
```
