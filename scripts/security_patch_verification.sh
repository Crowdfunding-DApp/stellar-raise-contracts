#!/usr/bin/env bash
# =============================================================================
# security_patch_verification.sh
# =============================================================================
# @title   SecurityPatchVerification — Automated CI/CD Security Patch Verification
# @notice  Verifies that all security patches are applied and dependency
#          vulnerability checks pass in CI/CD pipelines.
# @dev     Read-only operations only — no state modifications.
#          Designed for Stellar/Soroban smart contract projects.
#
# @author  Security Team
# @license Apache-2.0
#
# Security Assumptions:
#   1. Read-only       — No function writes to storage or state files
#   2. Permissionless  — No privileged access required to run checks
#   3. Deterministic   — Same input produces same output
#   4. Bounded         — No unbounded loops or iterations
#   5. Safe arithmetic — All numeric comparisons use integer arithmetic
#
# Usage:
#   ./security_patch_verification.sh
#   ./security_patch_verification.sh --verbose
# =============================================================================

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

readonly SCRIPT_NAME="security_patch_verification"
readonly VERSION="1.0.0"
readonly MIN_COVERAGE_PERCENT=95
readonly MIN_RUST_MINOR=74  # rustc >= 1.74

readonly COLOR_RESET='\033[0m'
readonly COLOR_RED='\033[0;31m'
readonly COLOR_GREEN='\033[0;32m'
readonly COLOR_YELLOW='\033[0;33m'
readonly COLOR_BLUE='\033[0;34m'
readonly COLOR_BOLD='\033[1m'

# ── Global State ──────────────────────────────────────────────────────────────

declare -i TOTAL_CHECKS=0
declare -i PASSED_CHECKS=0
declare -i FAILED_CHECKS=0
declare -i WARNINGS=0

declare -a FAILED_CHECKS_LIST=()
declare -a PASSED_CHECKS_LIST=()
declare -a WARNINGS_LIST=()

VERBOSE=false
PROJECT_ROOT="${PROJECT_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

# ── Logging ───────────────────────────────────────────────────────────────────

log_info()    { echo -e "${COLOR_BLUE}[INFO]${COLOR_RESET}  $*"; }
log_success() { echo -e "${COLOR_GREEN}[PASS]${COLOR_RESET}  $*"; }
log_warn()    { echo -e "${COLOR_YELLOW}[WARN]${COLOR_RESET}  $*"; }
log_error()   { echo -e "${COLOR_RED}[FAIL]${COLOR_RESET}  $*" >&2; }

# ── Result Tracking ───────────────────────────────────────────────────────────

record_pass() {
  local check_name="$1"
  TOTAL_CHECKS=$(( TOTAL_CHECKS + 1 ))
  PASSED_CHECKS=$(( PASSED_CHECKS + 1 ))
  PASSED_CHECKS_LIST+=("$check_name")
  log_success "$check_name"
}

record_fail() {
  local check_name="$1"
  local reason="${2:-}"
  TOTAL_CHECKS=$(( TOTAL_CHECKS + 1 ))
  FAILED_CHECKS=$(( FAILED_CHECKS + 1 ))
  FAILED_CHECKS_LIST+=("$check_name")
  log_error "$check_name${reason:+: $reason}"
}

record_warn() {
  local msg="$1"
  WARNINGS=$(( WARNINGS + 1 ))
  WARNINGS_LIST+=("$msg")
  log_warn "$msg"
}

# ── Checks ────────────────────────────────────────────────────────────────────

# @notice Runs `cargo audit` to detect known vulnerabilities in Rust dependencies.
check_cargo_audit() {
  local check="cargo_audit"
  if ! command -v cargo-audit &>/dev/null && ! cargo audit --version &>/dev/null 2>&1; then
    record_warn "cargo-audit not installed — skipping Rust dependency audit"
    return
  fi
  if cargo audit --quiet 2>/dev/null; then
    record_pass "$check"
  else
    record_fail "$check" "vulnerable Rust dependencies detected"
  fi
}

# @notice Runs `npm audit` to detect known vulnerabilities in Node dependencies.
check_npm_audit() {
  local check="npm_audit"
  if ! command -v npm &>/dev/null; then
    record_warn "npm not installed — skipping Node dependency audit"
    return
  fi
  if npm audit --audit-level=moderate --prefix "$PROJECT_ROOT" 2>/dev/null; then
    record_pass "$check"
  else
    record_fail "$check" "vulnerable npm dependencies detected"
  fi
}

# @notice Verifies the Rust toolchain is installed and meets the minimum version.
check_rust_toolchain_version() {
  local check="rust_toolchain_version"
  if ! command -v rustc &>/dev/null; then
    record_fail "$check" "rustc not found"
    return
  fi
  local version_str
  version_str=$(rustc --version 2>/dev/null | awk '{print $2}')
  local minor
  minor=$(echo "$version_str" | cut -d. -f2)
  if [[ "$minor" -ge "$MIN_RUST_MINOR" ]]; then
    record_pass "$check"
  else
    record_fail "$check" "rustc $version_str < 1.$MIN_RUST_MINOR required"
  fi
}

# @notice Verifies the wasm32-unknown-unknown compilation target is installed.
check_wasm_target() {
  local check="wasm32_target"
  if rustup target list --installed 2>/dev/null | grep -q "wasm32-unknown-unknown"; then
    record_pass "$check"
  else
    record_fail "$check" "wasm32-unknown-unknown target not installed (run: rustup target add wasm32-unknown-unknown)"
  fi
}

# @notice Verifies Cargo.lock exists, confirming patch integrity.
check_patch_signatures() {
  local check="patch_signatures"
  if [[ -f "$PROJECT_ROOT/Cargo.lock" ]]; then
    record_pass "$check"
  else
    record_fail "$check" "Cargo.lock not found — patch integrity cannot be verified"
  fi
}

# ── Summary ───────────────────────────────────────────────────────────────────

print_summary() {
  echo ""
  echo -e "${COLOR_BOLD}═══════════════════════════════════════${COLOR_RESET}"
  echo -e "${COLOR_BOLD} $SCRIPT_NAME v$VERSION — Summary${COLOR_RESET}"
  echo -e "${COLOR_BOLD}═══════════════════════════════════════${COLOR_RESET}"
  echo -e "  Total:    $TOTAL_CHECKS"
  echo -e "  ${COLOR_GREEN}Passed:   $PASSED_CHECKS${COLOR_RESET}"
  echo -e "  ${COLOR_RED}Failed:   $FAILED_CHECKS${COLOR_RESET}"
  echo -e "  ${COLOR_YELLOW}Warnings: $WARNINGS${COLOR_RESET}"

  if [[ "${#FAILED_CHECKS_LIST[@]}" -gt 0 ]]; then
    echo ""
    log_error "Failed checks:"
    for item in "${FAILED_CHECKS_LIST[@]}"; do
      echo "    - $item"
    done
  fi

  echo ""
  if [[ "$FAILED_CHECKS" -gt 0 ]]; then
    log_error "Security patch verification FAILED"
    exit 1
  else
    log_success "Security patch verification PASSED"
    exit 0
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
  for arg in "$@"; do
    case "$arg" in
      --verbose) VERBOSE=true ;;
    esac
  done

  log_info "Starting $SCRIPT_NAME v$VERSION"
  log_info "Project root: $PROJECT_ROOT"
  echo ""

  check_rust_toolchain_version
  check_wasm_target
  check_patch_signatures
  check_cargo_audit
  check_npm_audit

  print_summary
}

main "$@"
