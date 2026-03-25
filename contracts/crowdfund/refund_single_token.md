# `refund_single_token` Notes

The authoritative implementation and security write-up now live in
[`src/refund_single_token.md`](./src/refund_single_token.md).

## Summary

`refund_single()` is the preferred pull-based refund path for the crowdfund
contract. Its core guarantees are:

- contributor authentication is required
- refunds are blocked until after the deadline and only when the goal was missed
- arithmetic is checked before state mutation
- the contributor record is zeroed before the token transfer
- one `("campaign", "refund_single")` event is emitted on success

See the source-level documentation for the full flow, test references, and the
distinction between validated contract invariants and broader Soroban runtime
assumptions.
