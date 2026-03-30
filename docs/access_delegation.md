# Access Delegation Security Module

## Overview

`access_delegation.rs` adds a scoped and time-bounded delegation model for contracts that need to let a trusted delegate perform limited operations on behalf of a delegator.

The design emphasizes least privilege:

- delegates are limited to explicit scopes
- delegations have hard expiry timestamps
- only active delegations can authorize calls
- only the delegator can revoke an active delegation

## Files

- Contract logic: `contracts/security/src/access_delegation.rs`
- Test suite: `contracts/security/src/access_delegation.test.rs`
- Module registration: `contracts/security/src/lib.rs`

## Core Types

- `AccessDelegation`: delegation record (`delegator`, `delegate`, `scopes`, `expires_at`, `active`)
- `DelegationCheck`: authorization result (`Authorized` or `Denied { reason }`)

## Security Controls

### Input Validation

`validate_delegation_input()` enforces:

- no self-delegation
- non-empty scope list
- bounded scope list (`MAX_SCOPE_COUNT`)
- no duplicate scopes
- non-zero current timestamp
- strictly future expiry

### Runtime Authorization

`can_delegate_call()` verifies:

- delegation is active
- caller equals delegated address
- delegation is not expired
- requested scope is included

### Revocation

`validate_revocation()` only allows the delegator to revoke, and prevents revoking an already inactive delegation.

### Capacity Limits

`validate_delegation_capacity()` limits active delegations per delegator (`MAX_ACTIVE_DELEGATIONS_PER_DELEGATOR`) to reduce abuse and storage growth risk.

### Audit Events

Event helpers are provided for observability:

- `emit_delegation_created_event`
- `emit_delegation_revoked_event`

## Test Coverage

`access_delegation.test.rs` includes:

- happy paths for delegation creation and authorized calls
- edge cases (boundary expiry, inactive delegation, wrong caller)
- failure paths for malformed input and unauthorized revocation
- active delegation counting and capacity-limit checks
- event helper smoke tests
- property tests for expiry validation and authorization invariants

## Security Assumptions

- Integrating contracts enforce authentication for who can create/revoke delegations.
- Ledger timestamp is trusted as monotonic enough for expiry checks.
- Scope identifiers are stable and unambiguous across contract versions.
- Storage persistence and retrieval of delegation records are correct in integration code.

## Performance Notes

- Scope lookup and duplicate detection are linear/quadratic in small bounded vectors.
- Active delegation count is linear in delegation records scanned.
- Bounded constants keep execution predictable and reviewable.
