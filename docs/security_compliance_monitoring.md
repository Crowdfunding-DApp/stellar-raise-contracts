# security_compliance_monitoring

Continuous CI/CD compliance oversight for the Stellar Raise crowdfunding contract.

## Overview

`security_compliance_monitoring.sh` monitors the repository for ongoing compliance signals:
dependency freshness, secret leakage, file integrity, and CI workflow health.
It complements the auditing script by focusing on *continuous* oversight rather than
point-in-time deep audits.

## Monitor Phases

| Phase | Name | What it checks |
|-------|------|----------------|
| 1 | Tool Presence | `cargo`, `git` are installed |
| 2 | Required File Presence | `Cargo.toml`, `README.md`, `SECURITY.md`, `LICENSE`, CI workflows |
| 3 | Secret Leakage Scan | Scans tracked files for `PRIVATE_KEY`, `SECRET_KEY`, `password =`, etc. |
| 4 | `.gitignore` Compliance | `.soroban/`, `target/`, report dirs are ignored |
| 5 | CI Workflow Health | `cargo audit`, `cargo clippy`, `cargo test`, `cargo fmt`, `timeout-minutes` present |
| 6 | Dependency Freshness | `Cargo.lock` exists; `cargo audit` reports no vulnerabilities |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All monitors passed |
| `1` | One or more monitors failed |
| `2` | Required tooling is missing |

## Usage

```bash
# Basic run
./scripts/security_compliance_monitoring.sh

# Verbose output
./scripts/security_compliance_monitoring.sh --verbose

# Write JSON report
./scripts/security_compliance_monitoring.sh --json --report-dir monitoring-reports

# Help
./scripts/security_compliance_monitoring.sh --help
```

## Security Assumptions

1. **Read-only** — The script never writes to source files or contract storage.
2. **Permissionless** — No privileged credentials are required.
3. **Deterministic** — Same repository state → same result.
4. **Bounded** — All iterations are over fixed-size constant arrays.

## Secret Patterns Scanned

The following patterns are checked across all git-tracked source files:

- `PRIVATE_KEY`
- `SECRET_KEY`
- `-----BEGIN.*PRIVATE`
- `sk_live_`
- `sk_test_`
- `password\s*=`
- `api_key\s*=`

To add new patterns, extend the `SECRET_PATTERNS` array in the script.

## JSON Report Format

```json
{
  "script": "security_compliance_monitoring",
  "version": "1.0.0",
  "timestamp": "2026-03-29T05:46:48Z",
  "status": "PASS",
  "summary": {
    "total": 6,
    "passed": 6,
    "failed": 0,
    "warnings": 0
  }
}
```

## CI Integration

```yaml
- name: Make monitoring script executable
  run: chmod +x scripts/security_compliance_monitoring.sh

- name: Run security compliance monitoring
  run: ./scripts/security_compliance_monitoring.sh --json --report-dir monitoring-reports

- name: Upload monitoring report
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: monitoring-report
    path: monitoring-reports/
```

## Running Tests

```bash
chmod +x scripts/security_compliance_monitoring.test.sh
./scripts/security_compliance_monitoring.test.sh
# or with verbose output:
./scripts/security_compliance_monitoring.test.sh --verbose
```

Test coverage target: ≥ 95 % of all monitorable code paths.
