# Security Compliance Automation

Automated security and compliance checks for the Stellar Raise CI/CD pipeline.

## Overview

`scripts/security_compliance_automation.sh` runs six targeted checks on every
CI run (and locally on demand) to catch vulnerabilities, credential leaks, and
policy violations before they reach production.

| # | Check | Tool / Method |
|---|-------|---------------|
| 1 | npm dependency audit | `npm audit --audit-level=high` |
| 2 | Cargo dependency audit | `cargo audit` |
| 3 | Secret / credential leak scan | `grep` over git-tracked files |
| 4 | Workflow least-privilege permissions | `grep` over `.github/workflows/` |
| 5 | WASM binary size gate | `stat` — must be ≤ 256 KB after `wasm-opt` |
| 6 | Required security policy files | `SECURITY.md` and `LICENSE` non-empty |

## Usage

```bash
# Run all checks (from repo root)
bash scripts/security_compliance_automation.sh

# Override WASM path or size limit
WASM_PATH=target/wasm32-unknown-unknown/release/crowdfund.opt.wasm \
WASM_MAX_KB=128 \
bash scripts/security_compliance_automation.sh
```

Exit code `0` means all checks passed. Exit code `1` means one or more checks
failed; details are printed to stderr.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WASM_PATH` | `target/wasm32-unknown-unknown/release/crowdfund.opt.wasm` | Path to the optimised WASM binary |
| `WASM_MAX_KB` | `256` | Maximum allowed WASM size in kilobytes |

## CI Integration

Add the following step to `.github/workflows/rust_ci.yml` after the
`wasm-opt` step:

```yaml
- name: Run security compliance checks
  run: bash scripts/security_compliance_automation.sh
```

The script skips checks gracefully when optional tools (`npm`, `cargo-audit`)
are not present, so it is safe to add to any job without extra setup.

## Checks in Detail

### 1 — npm Audit

Runs `npm audit --audit-level=high`. Any high or critical CVE causes the check
to fail. Skipped when `npm` is not on `PATH`.

### 2 — Cargo Audit

Runs `cargo audit` against the RustSec advisory database. Skipped when
`cargo-audit` is not installed.

### 3 — Secret Leak Detection

Scans all git-tracked source files (`.sh`, `.yml`, `.yaml`, `.ts`, `.tsx`,
`.js`, `.rs`, `.toml`, `.json`) for patterns that resemble hardcoded secrets:

- AWS access key IDs (`AKIA…`)
- Stellar secret keys (`S` + 55 base32 characters)
- PEM private key headers (`-----BEGIN … PRIVATE KEY-----`)

Matching file *counts* are reported; no secret values are ever printed.

### 4 — Workflow Least-Privilege Permissions

Every file under `.github/workflows/` must contain a `permissions:` block.
Missing blocks are reported as failures. This enforces the GitHub Actions
[least-privilege principle](https://docs.github.com/en/actions/security-guides/automatic-token-authentication#permissions-for-the-github_token).

### 5 — WASM Binary Size Gate

The optimised WASM binary must not exceed `WASM_MAX_KB` kilobytes (default
256 KB). The check is skipped when the binary does not exist, allowing the
script to run in pre-build environments.

### 6 — Required Security Policy Files

`SECURITY.md` and `LICENSE` must exist and be non-empty. These files are
required so contributors know how to report vulnerabilities and understand the
licence terms.

## Running the Tests

```bash
bash scripts/security_compliance_automation.test.sh
```

The test suite creates isolated temporary git repositories for each failure
path so tests are hermetic and do not modify the working tree.

## Security Assumptions

- The script reads files only; it never writes, executes, or transmits data.
- Secret patterns use anchored regexes to minimise false positives.
- Temporary directories created by the test suite are removed on exit via a
  `trap … EXIT` handler.
- The script must be run from the repository root (standard for CI jobs).
