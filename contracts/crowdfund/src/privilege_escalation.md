# Privilege Escalation Module

## Overview

`privilege_escalation.rs` implements a **two-step, time-locked role promotion model** for the Stellar crowdfund smart contract. It prevents single-transaction privilege grabs by requiring an explicit nomination followed by a separate acceptance within a bounded time window.

---

## Motivation

The existing `access_control` module stores roles but provides no safe path for *promoting* an address to a higher role. Without a controlled escalation mechanism, an admin key compromise could silently reassign the top role in a single transaction. This module closes that gap.

---

## Escalation Paths

| From role          | To role              | Gated by              | Prerequisite           |
|--------------------|----------------------|-----------------------|------------------------|
| (any)              | `PAUSER_ROLE`        | `DEFAULT_ADMIN_ROLE`  | none                   |
| (any)              | `GovernanceAddress`  | `DEFAULT_ADMIN_ROLE`  | none                   |
| `PAUSER_ROLE`      | `DEFAULT_ADMIN_ROLE` | current `DEFAULT_ADMIN` | nominee must hold `PAUSER_ROLE` |

The prerequisite chain for `DEFAULT_ADMIN_ROLE` ensures no cold wallet can be promoted to the highest role in a single step.

---

## Two-Step Flow

```
Admin                          Nominee
  │                               │
  │── nominate_pauser(nominee) ──▶│  (step 1: nomination stored on-chain)
  │                               │
  │                               │── accept_role_pauser() ──▶ role committed
  │                               │   (must happen within 24 h)
```

1. **Nominate** — `DEFAULT_ADMIN_ROLE` calls `nominate_pauser`, `nominate_governance`, or `nominate_default_admin`. A `PendingNomination` struct is written to instance storage.
2. **Accept** — The nominee calls the corresponding `accept_role_*` function. The module validates the nominee identity, the acceptance window, and that the nominating admin has not been replaced.

---

## Security Properties

| Property | Mechanism |
|---|---|
| No single-tx escalation | Two separate transactions required |
| Time-lock | `ESCALATION_ACCEPTANCE_WINDOW = 86 400 s` (24 h) |
| Prerequisite chain | `DEFAULT_ADMIN` nomination requires nominee to hold `PAUSER_ROLE` |
| Admin rotation guard | Acceptance fails if the nominating admin was replaced before acceptance |
| Replay prevention | Pending nomination is cleared from storage on acceptance |
| Revocability | Admin can cancel any pending nomination before acceptance |
| Auditability | Every nomination, acceptance, and revocation emits an on-chain event |

---

## Public API

### Nomination functions (admin only)

```rust
/// Nominate `nominee` for PAUSER_ROLE.
pub fn nominate_pauser(env: &Env, caller: &Address, nominee: &Address)

/// Nominate `nominee` for GovernanceAddress.
pub fn nominate_governance(env: &Env, caller: &Address, nominee: &Address)

/// Nominate `nominee` for DEFAULT_ADMIN_ROLE.
/// Requires nominee to already hold PAUSER_ROLE.
pub fn nominate_default_admin(env: &Env, caller: &Address, nominee: &Address)
```

### Acceptance functions (nominee only)

```rust
pub fn accept_role_pauser(env: &Env, caller: &Address) -> Result<(), ContractError>
pub fn accept_role_governance(env: &Env, caller: &Address) -> Result<(), ContractError>
pub fn accept_role_default_admin(env: &Env, caller: &Address) -> Result<(), ContractError>
```

### Revocation (admin only)

```rust
/// Cancel a pending nomination before it is accepted.
pub fn revoke_nomination(env: &Env, caller: &Address, role_tag: &str) -> Result<(), ContractError>
```

### Query helpers

```rust
/// Returns the pending nomination for role_tag, or None.
pub fn get_pending_nomination(env: &Env, role_tag: &str) -> Option<PendingNomination>

/// Returns true if addr currently holds role_tag.
pub fn has_role(env: &Env, addr: &Address, role_tag: &str) -> bool
```

---

## Events

All events are published under the `privilege` topic:

| Event symbol          | Payload                          | Trigger                        |
|-----------------------|----------------------------------|--------------------------------|
| `nominated_pauser`    | `(admin, nominee)`               | `nominate_pauser`              |
| `nominated_governance`| `(admin, nominee)`               | `nominate_governance`          |
| `nominated_admin`     | `(admin, nominee)`               | `nominate_default_admin`       |
| `role_accepted`       | `(nominee, role_tag)`            | any `accept_role_*`            |
| `nomination_revoked`  | `(admin, role_tag)`              | `revoke_nomination`            |

---

## Storage Layout

Pending nominations are stored as `Symbol`-keyed instance storage entries:

| Key              | Type                | Description                        |
|------------------|---------------------|------------------------------------|
| `pending_PAUSER` | `PendingNomination` | Pending pauser nomination          |
| `pending_GOVERNANCE` | `PendingNomination` | Pending governance nomination  |
| `pending_DEFAULT_ADMIN` | `PendingNomination` | Pending admin nomination      |

Instance storage shares the contract's TTL and is automatically cleaned up on upgrade.

---

## Constants

| Constant                        | Value    | Description                          |
|---------------------------------|----------|--------------------------------------|
| `ESCALATION_ACCEPTANCE_WINDOW`  | `86 400` | Seconds nominee has to accept (24 h) |

---

## Security Assumptions

1. Only `DEFAULT_ADMIN_ROLE` may initiate any escalation nomination.
2. Nominees must call `accept_role_*` within `ESCALATION_ACCEPTANCE_WINDOW` ledger-seconds.
3. A pending nomination is invalidated if the nominating admin is replaced before acceptance.
4. Escalation to `DEFAULT_ADMIN_ROLE` requires the nominee to already hold `PAUSER_ROLE`.
5. All escalation events are emitted for off-chain monitoring and audit.
6. No token transfers occur in this module — re-entrancy is not a concern.

---

## Test Coverage

Run the test suite with:

```bash
cargo test -p crowdfund privilege_escalation -- --nocapture
```

| Test | Scenario |
|---|---|
| `test_nominate_pauser_by_admin_succeeds` | Happy path nomination |
| `test_nominate_pauser_by_non_admin_panics` | Unauthorized nomination rejected |
| `test_accept_role_pauser_succeeds` | Happy path acceptance |
| `test_accept_role_pauser_wrong_caller_panics` | Impostor acceptance rejected |
| `test_accept_role_pauser_expired_panics` | Expired window rejected |
| `test_accept_role_pauser_no_nomination_returns_error` | No pending nomination |
| `test_accept_role_pauser_replay_prevention` | Double-accept rejected |
| `test_nominate_and_accept_governance_succeeds` | Governance happy path |
| `test_nominate_governance_by_non_admin_panics` | Unauthorized governance nomination |
| `test_nominate_and_accept_default_admin_succeeds` | Admin promotion happy path |
| `test_nominate_default_admin_without_pauser_role_panics` | Prerequisite chain enforced |
| `test_nominate_default_admin_by_non_admin_panics` | Unauthorized admin nomination |
| `test_accept_pauser_fails_if_nominator_replaced` | Admin rotation guard |
| `test_revoke_nomination_by_admin_succeeds` | Revocation happy path |
| `test_revoke_nomination_by_non_admin_panics` | Unauthorized revocation |
| `test_revoke_nomination_no_pending_returns_error` | Revoke when nothing pending |
| `test_revoked_nomination_cannot_be_accepted` | Post-revocation acceptance blocked |
| `test_has_role_unknown_tag_returns_false` | Unknown role tag |
| `test_get_pending_nomination_none_when_absent` | Query when nothing pending |
| `test_nomination_metadata_stored_correctly` | Metadata integrity |
| `test_overwrite_pending_nomination` | Latest nomination wins |
| `test_accept_at_window_boundary_succeeds` | Boundary condition |
