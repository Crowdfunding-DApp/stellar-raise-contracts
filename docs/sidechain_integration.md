# Sidechain Integration

Lightweight cross-chain message verification and relay primitives for the
Stellar Raise crowdfund contract, focused on gas efficiency and
interoperability.

## Overview

`sidechain_integration.rs` provides bounded, read-only primitives for
verifying cross-chain messages and estimating gas savings from sidechain
offloading. Results can be emitted as on-chain events so off-chain monitors
and CI pipelines can react without privileged access.

## Security Assumptions

1. **Read-only** — No function writes to contract storage.
2. **No auth required** — Verification is permissionless.
3. **Deterministic** — Same inputs always produce the same output.
4. **Overflow-safe** — All arithmetic uses `saturating_*` operations.
5. **Bounded** — Message payloads are bounded by `MAX_PAYLOAD_BYTES` (256).

## Constants

| Constant | Value | Description |
|:---------|:------|:------------|
| `MAX_PAYLOAD_BYTES` | `256` | Maximum message payload size in bytes |
| `MAX_CHAIN_ID` | `65535` | Maximum valid chain identifier |

## Types

### `ChainStatus`

```rust
pub enum ChainStatus { Active, Inactive, Suspended }
```

`is_active()` returns `true` only for `Active`.

### `SidechainMessage`

```rust
pub struct SidechainMessage {
    pub chain_id: u32,
    pub sequence: u64,
    pub payload_len: usize,
}
```

`SidechainMessage::new(chain_id, sequence, payload_len)` returns `None` if
`chain_id > MAX_CHAIN_ID` or `payload_len > MAX_PAYLOAD_BYTES`.

### `RelayResult`

```rust
pub struct RelayResult {
    pub success: bool,
    pub gas_saved_bps: u32,  // basis points
}
```

`gas_efficiency_pct()` returns `gas_saved_bps / 100`.

## Functions

| Function | Description |
|:---------|:------------|
| `verify_message(msg)` | Returns `true` when the message passes all validity checks |
| `estimate_gas_savings(payload_len)` | Returns gas savings in bps; shorter payloads yield higher savings |
| `emit_relay_event(env, chain_id, sequence)` | Emits a `"relay"` event on-chain |

## Gas Efficiency

Gas savings are estimated as:

```
savings_bps = (MAX_PAYLOAD_BYTES - payload_len) * 10
```

A zero-length payload achieves the maximum saving of `2560` bps (25.6%).
Payloads at or beyond `MAX_PAYLOAD_BYTES` yield `0` bps.

## Test Coverage

The test suite (`sidechain_integration.test.rs`) covers:

- `SidechainMessage::new` — valid, invalid chain ID, invalid payload, boundary
- `is_valid` — true and false paths
- `verify_message` — valid and invalid messages
- `estimate_gas_savings` — zero, max, half, overflow-safe
- `RelayResult::gas_efficiency_pct` — zero, 50%, 100%
- `ChainStatus::is_active` — all three variants
- `emit_relay_event` — no panic

Target: ≥ 95% of all code paths covered.

## Related Files

- [`contracts/crowdfund/src/sidechain_integration.rs`](../contracts/crowdfund/src/sidechain_integration.rs) — Module
- [`contracts/crowdfund/src/sidechain_integration.test.rs`](../contracts/crowdfund/src/sidechain_integration.test.rs) — Tests
- [`contracts/crowdfund/src/lib.rs`](../contracts/crowdfund/src/lib.rs) — Module registration
- [`docs/dependency_vulnerability_scanning.md`](dependency_vulnerability_scanning.md) — Related security module
