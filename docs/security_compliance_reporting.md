# Security Compliance Reporting

## Overview

`scripts/security_compliance_reporting.sh` is an automated security compliance
scanner for the Stellar Raise CI/CD pipeline. It runs a suite of checks against
the repository and emits a structured JSON report that can be consumed by CI
systems, dashboards, or auditors.

---

## Checks Performed

| Check | Tool | Blocking |
|---|---|---|
| Rust dependency vulnerabilities | `cargo audit` | Yes (when `FAIL_ON_HIGH=true`) |
| Secret / credential leaks | `gitleaks` | Yes |
| Hardcoded secret patterns | `git grep` (built-in) | Yes |
| World-writable shell scripts | `find` (built-in) | No (warn only) |
| Licence file presence & type | file scan (built-in) | No (warn only) |
| Unpinned GitHub Actions | `grep` (built-in) | No (warn only) |

Checks that depend on optional tools (`cargo-audit`, `gitleaks`) are
**gracefully skipped** with a `"status":"skipped"` entry in the report when
the tool is not installed.

---

## Usage

```bash
# Basic run (exits 2 if blocking findings are found)
bash scripts/security_compliance_reporting.sh

# Non-blocking mode (always exits 0, useful for reporting-only pipelines)
FAIL_ON_HIGH=false bash scripts/security_compliance_reporting.sh

# Custom output paths
REPORT_FILE=reports/compliance.json \
REPORT_LOG=reports/compliance.log \
bash scripts/security_compliance_reporting.sh
```

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `REPORT_FILE` | `security_compliance_report.json` | Path for the JSON report output |
| `REPORT_LOG` | `security_compliance.log` | Path for the human-readable run log |
| `FAIL_ON_HIGH` | `true` | Exit with code 2 when blocking findings exist |

---

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Report generated, no blocking findings |
| `1` | Internal script error or missing required tool (`git`) |
| `2` | Blocking HIGH/CRITICAL findings detected (only when `FAIL_ON_HIGH=true`) |

---

## Report Format

The script writes a JSON file with the following top-level structure:

```json
{
  "report_version": "1.0.0",
  "generated_at": "2026-03-27T10:00:00Z",
  "git_sha": "abc1234",
  "git_branch": "feature/add-automated-security-compliance-reporting-for-cicd",
  "overall_status": "pass",
  "checks": {
    "cargo_audit":        { "status": "pass", "vulnerability_count": 0, "findings": [] },
    "secret_leaks":       { "status": "skipped", "reason": "gitleaks not installed", "leak_count": 0, "findings": [] },
    "hardcoded_secrets":  { "status": "pass", "match_count": 0, "findings": [] },
    "file_permissions":   { "status": "pass", "world_writable_scripts": 0, "findings": [] },
    "licence_compliance": { "status": "pass", "licence_file": "LICENSE", "licence_type": "Apache-2.0" },
    "cicd_pinning":       { "status": "pass", "unpinned_count": 0, "findings": [] }
  }
}
```

`overall_status` is derived from the worst individual check status:
`fail` > `warn` > `skipped` / `pass`.

---

## CI/CD Integration

Add the following step to any GitHub Actions workflow:

```yaml
- name: Security compliance report
  run: |
    FAIL_ON_HIGH=true bash scripts/security_compliance_reporting.sh
  env:
    REPORT_FILE: security_compliance_report.json

- name: Upload compliance report
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: security-compliance-report
    path: security_compliance_report.json
```

---

## Running Tests

```bash
bash scripts/security_compliance_reporting.test.sh
```

Expected output ends with a summary line:

```
Results: N passed, 0 failed
Coverage proxy: 100% (N/N assertions passing)
```

All tests are self-contained, use only bash built-ins and `mktemp`, and clean
up all temporary files on exit. No network access or real credentials are
required.

---

## Security Assumptions & Threat Model

1. **Secret scanning scope** — `check_hardcoded_secrets` scans only tracked
   files via `git grep`. Untracked files are not scanned; use `gitleaks` for
   full coverage including git history.

2. **cargo audit advisory lag** — The Rust advisory database is fetched at
   scan time. Advisories published after the last `cargo audit` database sync
   will not appear until the next sync.

3. **gitleaks false positives** — Secret pattern matching may produce false
   positives on test fixtures or example values. Review findings before
   treating them as confirmed leaks.

4. **CI/CD pinning** — The script flags actions pinned to branch names
   (`main`, `master`, `develop`, `HEAD`) as supply-chain risks. Pinning to a
   full commit SHA is the recommended practice.

5. **Script integrity** — This script should itself be pinned to a known-good
   commit SHA in CI. Running an unpinned version of this script from an
   untrusted branch defeats its purpose.

6. **No credentials stored** — The script never writes secrets, tokens, or
   private keys to disk. All temporary files contain only scan metadata and
   are deleted on exit via the `_cleanup` trap.

---

## Adding New Checks

1. Write a `check_<name>()` function that prints a JSON fragment with at
   minimum a `"status"` field (`pass` | `warn` | `fail` | `skipped`).
2. Call it in `main()` and capture the output into a variable.
3. Extract the status string and pass it to `aggregate_severity`.
4. Add the JSON fragment to `assemble_report`.
5. Write corresponding tests in `security_compliance_reporting.test.sh`.

---

## Dependencies

| Tool | Required | Install |
|---|---|---|
| `bash` ≥ 4.2 | Yes | system |
| `git` | Yes | system |
| `cargo-audit` | No | `cargo install cargo-audit` |
| `gitleaks` | No | https://github.com/gitleaks/gitleaks/releases |
| `jq` | No (enhances output) | `apt install jq` / `brew install jq` |
