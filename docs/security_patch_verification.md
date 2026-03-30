# Security Patch Verification

Automated CI/CD script that verifies security patches are applied and all
dependency vulnerability checks pass for the Stellar Raise crowdfund project.

## Overview

`security_patch_verification.sh` runs a suite of read-only checks against the
local environment and dependency manifests. It is designed to run in CI/CD
pipelines (GitHub Actions) and locally before merging security-sensitive PRs.

Each check records a pass or fail result. The script exits with code `0` when
all checks pass and `1` when any check fails, making it suitable as a required
status check on pull requests.

## Security Assumptions

1. **Read-only** — No function writes to storage or state files.
2. **Permissionless** — No privileged access required to run checks.
3. **Deterministic** — Same environment produces the same result.
4. **Bounded execution** — No unbounded loops; checks are finite.
5. **Safe arithmetic** — All numeric comparisons use integer arithmetic.

## Usage

```bash
# Make executable (first time only)
chmod +x scripts/security_patch_verification.sh

# Run all checks
./scripts/security_patch_verification.sh

# Verbose output
./scripts/security_patch_verification.sh --verbose

# Run the test suite
chmod +x scripts/security_patch_verification.test.sh
./scripts/security_patch_verification.test.sh
```

## CI/CD Integration

Add to `.github/workflows/security.yml`:

```yaml
- name: Make patch verification script executable
  run: chmod +x scripts/security_patch_verification.sh

- name: Run security patch verification
  run: ./scripts/security_patch_verification.sh
```

## Checks Performed

| Check | Description | Failure Condition |
|:------|:------------|:------------------|
| `rust_toolchain_version` | Verifies `rustc` >= 1.74 is installed | `rustc` missing or too old |
| `wasm32_target` | Verifies `wasm32-unknown-unknown` target is installed | Target not in `rustup target list` |
| `patch_signatures` | Verifies `Cargo.lock` exists for patch integrity | `Cargo.lock` absent |
| `cargo_audit` | Runs `cargo audit` for known Rust CVEs | Any advisory found |
| `npm_audit` | Runs `npm audit --audit-level=moderate` | Moderate+ vulnerability found |

Checks for `cargo-audit` and `npm` are skipped with a warning if the tools are
not installed, so the script can run in minimal environments.

## Test Coverage

The test suite (`security_patch_verification.test.sh`) covers:

- Script existence and executability
- `VERSION` and `MIN_COVERAGE_PERCENT` constants
- All logging and helper functions are defined
- `check_patch_signatures` pass path (Cargo.lock present)
- `check_patch_signatures` fail path (Cargo.lock absent)
- `print_summary` exits `0` when no failures
- `print_summary` exits `1` when failures exist

Target: ≥ 95% of all code paths covered.

## Related Files

- [`scripts/security_patch_verification.sh`](../scripts/security_patch_verification.sh) — Main script
- [`scripts/security_patch_verification.test.sh`](../scripts/security_patch_verification.test.sh) — Test suite
- [`scripts/security_compliance_automation.sh`](../scripts/security_compliance_automation.sh) — Related compliance script
- [`.github/workflows/security.yml`](../.github/workflows/security.yml) — CI/CD integration
