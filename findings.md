# Findings

## Test suite (absence): mid-batch pledger transfer failure

- **Status:** Resolved
- **Location:** test suite (absence)
- **Description:** Simulating a pledger whose token transfer fails mid-batch, and asserting the campaign becomes permanently stuck, requires building a custom mock SEP-41 token with configurable failure behavior inside the Soroban test harness — nontrivial test infrastructure that doesn't currently exist anywhere in the suite.
- **Resolution:** Covered by `apps/contracts/crowdfund/src/blocklist_transfer_test.rs`, which implements a mock SEP-41 token (`BlocklistToken`) with a `set_frozen(address, bool)` toggle to force a specific pledger's `transfer` to panic on demand, and asserts that `collect_pledges`, `refund`, and `cancel` skip the failing entry (via `try_transfer`) rather than reverting the whole batch or leaving the campaign permanently stuck.
