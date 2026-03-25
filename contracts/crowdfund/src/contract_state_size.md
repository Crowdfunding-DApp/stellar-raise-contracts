# `contract_state_size` — Bounded Contract State for Reviewability and Reliability

## Overview

`contract_state_size` centralizes the limits for every crowdfund state field
whose size can grow from user input. The goal is to make worst-case storage
growth explicit, auditable, and enforceable in both local development and
CI/CD.

The module introduces pure validation helpers and wires them into the
contract's state-mutating entrypoints so oversize writes are rejected before
they are persisted.

## Why this matters

Without explicit bounds, a campaign can accumulate:

- Very large metadata strings
- Extremely long contributor or pledger indexes
- Unbounded roadmap entries
- Unbounded stretch-goal lists

That makes the contract harder to review, increases state- and payload-size
variance across environments, and weakens our confidence in worst-case
behavior during CI.

## Limits

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_CONTRIBUTORS` | `128` | Max indexed contributor addresses |
| `MAX_PLEDGERS` | `128` | Max indexed pledger addresses |
| `MAX_ROADMAP_ITEMS` | `32` | Max roadmap entries |
| `MAX_STRETCH_GOALS` | `32` | Max stretch-goal milestones |
| `MAX_TITLE_LENGTH` | `128` bytes | Max campaign title size |
| `MAX_DESCRIPTION_LENGTH` | `2048` bytes | Max campaign description size |
| `MAX_SOCIAL_LINKS_LENGTH` | `512` bytes | Max social-links field size |
| `MAX_BONUS_GOAL_DESCRIPTION_LENGTH` | `280` bytes | Max bonus-goal description size |
| `MAX_ROADMAP_DESCRIPTION_LENGTH` | `280` bytes | Max roadmap-item description size |
| `MAX_METADATA_TOTAL_LENGTH` | `2304` bytes | Combined title + description + socials budget |

## Validation helpers

The module exposes small pure helpers so both contract code and tests can
reuse the same rules:

- `validate_title`
- `validate_description`
- `validate_social_links`
- `validate_bonus_goal_description`
- `validate_roadmap_description`
- `validate_metadata_total_length`
- `validate_contributor_capacity`
- `validate_pledger_capacity`
- `validate_roadmap_capacity`
- `validate_stretch_goal_capacity`

Each helper returns `Result<(), &'static str>` and uses a stable error string
that makes failures easy to assert in tests and easy to spot in logs.

## Contract integration

The following entrypoints now enforce state-size limits:

### `initialize`

- Validates `bonus_goal_description` before storing it.

### `contribute`

- Rejects a contribution that would add a new address beyond
  `MAX_CONTRIBUTORS`.
- Existing contributors can still contribute even when the contributor index
  is already full.

### `pledge`

- Rejects a pledge that would add a new address beyond `MAX_PLEDGERS`.

### `update_metadata`

- Validates individual field lengths for `title`, `description`, and
  `socials`.
- Validates the combined metadata footprint using the existing stored values
  for fields that are not being updated in the current call.

### `add_roadmap_item`

- Rejects new entries once `MAX_ROADMAP_ITEMS` is reached.
- Rejects oversized roadmap descriptions.

### `add_stretch_goal`

- Rejects new milestones once `MAX_STRETCH_GOALS` is reached.

## Security assumptions

1. Bounding state growth makes worst-case storage usage reviewable and stable.
2. Rejecting oversize writes before persistence prevents silent storage bloat.
3. Limiting indexed address lists reduces risk in flows that later iterate
   over those lists.
4. A combined metadata budget prevents campaigns from storing several
   individually-valid but collectively excessive fields at once.
5. The contributor and pledger limits apply only to new index growth, so
   existing participants are not locked out from follow-up actions.

## NatSpec-style documentation

The Rust source includes NatSpec-style comments on:

- Every public constant
- Every public validation helper
- The module-level security assumptions and rationale

This keeps the rules close to the code and helps future reviews stay fast.

## Test coverage

See [`contract_state_size.test.rs`](./contract_state_size.test.rs).

The dedicated suite covers:

- Constant stability
- Exact-boundary acceptance for string limits
- Rejection one byte over the limit
- Aggregate metadata budget acceptance and rejection
- Overflow-safe handling for aggregate-length calculations
- Collection-capacity acceptance and rejection
- Contract-level rejection of oversize metadata and full collections
- Contract-level acceptance at valid boundaries

## Review notes

This implementation is intentionally small:

- Limits live in one file
- Enforcement points are narrow and explicit
- Tests exercise both pure helpers and real contract calls

That keeps the change efficient to review while still improving reliability
and reducing unbounded-state risk.
