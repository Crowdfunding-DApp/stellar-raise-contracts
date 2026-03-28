#!/usr/bin/env bash
# =============================================================================
# @file    security_compliance_automation.sh
# @brief   Automated security compliance checks for the Stellar Raise CI/CD
#          pipeline.
#
# @description
#   Runs a suite of security and compliance checks against the repository:
#     1. Dependency vulnerability audit (npm + cargo)
#     2. Secret / credential leak detection in tracked files
#     3. Workflow least-privilege permissions enforcement
#     4. WASM binary size gate (must not exceed 256 KB after wasm-opt)
#     5. Required security policy files present (SECURITY.md, LICENSE)
#
# @security
#   - Reads files only; never writes, executes, or transmits data.
#   - set -euo pipefail ensures unset variables and pipeline errors are fatal.
#   - Secret patterns use grep -E with anchored regexes to minimise false
#     positives; no actual secret values are printed.
#
# @usage
#   bash scripts/security_compliance_automation.sh [--wasm-path <path>]
#
#   Environment overrides:
#     WASM_PATH   Path to the optimised WASM binary (default: see below)
#     WASM_MAX_KB Maximum allowed WASM size in KB (default: 256)
#
# @exitcodes
#   0  All checks passed.
#   1  One or more checks failed (details printed to stderr).
#
# @author  stellar-raise-contracts contributors
# @version 1.0.0
# =============================================================================

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

WASM_PATH="${WASM_PATH:-target/wasm32-unknown-unknown/release/crowdfund.opt.wasm}"
WASM_MAX_KB="${WASM_MAX_KB:-256}"
WORKFLOWS_DIR=".github/workflows"

errors=0

# ── Helpers ───────────────────────────────────────────────────────────────────

# @function pass — prints a passing check message
# @param $1  description
pass() { echo "  PASS  $1"; }

# @function fail — prints a failing check message and increments error counter
# @param $1  description
fail() {
  echo "  FAIL  $1" >&2
  errors=$((errors + 1))
}

# =============================================================================
# Check 1 — npm dependency audit
# @notice  Runs `npm audit --audit-level=high` to surface high/critical CVEs.
#          Skipped gracefully when npm is not installed (e.g. Rust-only CI).
# =============================================================================
check_npm_audit() {
  if ! command -v npm &>/dev/null; then
    echo "  SKIP  npm audit (npm not found)"
    return
  fi
  if npm audit --audit-level=high --prefix . &>/dev/null; then
    pass "npm audit: no high/critical vulnerabilities"
  else
    fail "npm audit: high or critical vulnerabilities detected (run 'npm audit' for details)"
  fi
}

# =============================================================================
# Check 2 — Cargo dependency audit
# @notice  Runs `cargo audit` when the binary is available.
#          Skipped gracefully when cargo-audit is not installed.
# =============================================================================
check_cargo_audit() {
  if ! command -v cargo-audit &>/dev/null && ! cargo audit --version &>/dev/null 2>&1; then
    echo "  SKIP  cargo audit (cargo-audit not installed)"
    return
  fi
  if cargo audit &>/dev/null; then
    pass "cargo audit: no known vulnerabilities"
  else
    fail "cargo audit: vulnerabilities detected (run 'cargo audit' for details)"
  fi
}

# =============================================================================
# Check 3 — Secret / credential leak detection
# @notice  Scans git-tracked files for patterns that resemble secrets.
#          Patterns: bare private keys, AWS key prefixes, generic tokens.
#          Only tracked files are scanned (git ls-files) to avoid false
#          positives from build artefacts or node_modules.
# @security
#   Matching lines are counted but never printed to avoid leaking values.
# =============================================================================
check_secret_leaks() {
  local tracked_files
  tracked_files="$(git ls-files -- '*.sh' '*.yml' '*.yaml' '*.ts' '*.tsx' '*.js' '*.rs' '*.toml' '*.json' 2>/dev/null || true)"

  if [[ -z "$tracked_files" ]]; then
    pass "secret scan: no tracked source files to scan"
    return
  fi

  # Patterns that indicate a hardcoded secret
  local secret_pattern='(AKIA[0-9A-Z]{16}|S[0-9A-Z]{55}|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY)'

  local hits
  hits="$(echo "$tracked_files" | xargs grep -rlE -- "$secret_pattern" 2>/dev/null | wc -l || true)"

  if [[ "$hits" -eq 0 ]]; then
    pass "secret scan: no hardcoded credentials detected"
  else
    fail "secret scan: potential secrets found in $hits file(s) — review and rotate immediately"
  fi
}

# =============================================================================
# Check 4 — Workflow least-privilege permissions
# @notice  Every workflow file must declare `permissions: contents: read` (or
#          a more restrictive top-level permissions block) to follow the
#          principle of least privilege.
# =============================================================================
check_workflow_permissions() {
  local missing=0
  for wf in "$WORKFLOWS_DIR"/*.yml "$WORKFLOWS_DIR"/*.yaml; do
    [[ -f "$wf" ]] || continue
    if ! grep -q "permissions:" "$wf"; then
      echo "  WARN  $wf: no 'permissions:' block found" >&2
      missing=$((missing + 1))
    fi
  done
  if [[ "$missing" -eq 0 ]]; then
    pass "workflow permissions: all workflow files declare permissions"
  else
    fail "workflow permissions: $missing workflow file(s) missing a 'permissions:' block"
  fi
}

# =============================================================================
# Check 5 — WASM binary size gate
# @notice  The optimised WASM must not exceed WASM_MAX_KB kilobytes.
#          Skipped when the binary does not exist (pre-build environments).
# =============================================================================
check_wasm_size() {
  if [[ ! -f "$WASM_PATH" ]]; then
    echo "  SKIP  WASM size gate ($WASM_PATH not found — run a release build first)"
    return
  fi
  local size_bytes max_bytes
  size_bytes="$(stat -c%s "$WASM_PATH")"
  max_bytes=$(( WASM_MAX_KB * 1024 ))
  if [[ "$size_bytes" -le "$max_bytes" ]]; then
    pass "WASM size gate: ${size_bytes} bytes ≤ $((max_bytes)) bytes (${WASM_MAX_KB} KB limit)"
  else
    fail "WASM size gate: ${size_bytes} bytes exceeds ${max_bytes} bytes (${WASM_MAX_KB} KB limit)"
  fi
}

# =============================================================================
# Check 6 — Required security policy files
# @notice  SECURITY.md and LICENSE must exist and be non-empty so contributors
#          know how to report vulnerabilities and understand the licence terms.
# =============================================================================
check_required_files() {
  local required_files=("SECURITY.md" "LICENSE")
  local missing=0
  for f in "${required_files[@]}"; do
    if [[ -s "$f" ]]; then
      pass "required file present: $f"
    else
      fail "required file missing or empty: $f"
      missing=$((missing + 1))
    fi
  done
}

# =============================================================================
# Main
# =============================================================================
main() {
  echo "=== Security Compliance Automation ==="
  echo ""

  check_npm_audit
  check_cargo_audit
  check_secret_leaks
  check_workflow_permissions
  check_wasm_size
  check_required_files

  echo ""
  if [[ "$errors" -eq 0 ]]; then
    echo "All security compliance checks passed."
    exit 0
  else
    echo "$errors check(s) failed." >&2
    exit 1
  fi
}

main "$@"
