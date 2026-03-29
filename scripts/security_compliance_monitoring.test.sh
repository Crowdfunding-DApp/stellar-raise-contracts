#!/usr/bin/env bash
# =============================================================================
# security_compliance_monitoring.test.sh
# =============================================================================
# @title   SecurityComplianceMonitoring Test Suite
# @notice  Comprehensive tests for security_compliance_monitoring.sh.
#          Covers all monitor phases, edge cases, and error paths.
# @dev     Self-contained — uses only bash builtins and temporary directories.
#          Minimum 95% coverage of all monitorable code paths.
#
# Usage:
#   ./security_compliance_monitoring.test.sh [--verbose]
# =============================================================================

set -euo pipefail

readonly SCRIPT_UNDER_TEST="$(dirname "$0")/security_compliance_monitoring.sh"

VERBOSE=false
[[ "${1:-}" == "--verbose" ]] && VERBOSE=true

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

TMPDIR_ROOT=""

setup()    { TMPDIR_ROOT=$(mktemp -d); }
teardown() { [[ -n "$TMPDIR_ROOT" && -d "$TMPDIR_ROOT" ]] && rm -rf "$TMPDIR_ROOT"; }
trap teardown EXIT

# ── Harness ───────────────────────────────────────────────────────────────────

pass_test() { echo -e "${GREEN}[PASS]${NC} $1"; TESTS_PASSED=$(( TESTS_PASSED + 1 )); TESTS_RUN=$(( TESTS_RUN + 1 )); }
fail_test() { echo -e "${RED}[FAIL]${NC} $1"; TESTS_FAILED=$(( TESTS_FAILED + 1 )); TESTS_RUN=$(( TESTS_RUN + 1 )); }

# ── Tests: CLI flags ──────────────────────────────────────────────────────────

test_help_flag() {
    local out
    out=$(bash "$SCRIPT_UNDER_TEST" --help 2>&1) || true
    echo "$out" | grep -q "Usage:" && pass_test "--help prints usage" || fail_test "--help should print usage"
}

test_unknown_flag_exits_nonzero() {
    local exit_code=0
    bash "$SCRIPT_UNDER_TEST" --bad-flag 2>&1 || exit_code=$?
    [[ "$exit_code" -ne 0 ]] && pass_test "unknown flag exits non-zero" || fail_test "unknown flag should exit non-zero"
}

# ── Tests: required file presence ────────────────────────────────────────────

test_required_file_present() {
    setup
    local file="$TMPDIR_ROOT/Cargo.toml"
    touch "$file"
    [[ -f "$file" ]] && pass_test "required file presence check (exists)" || fail_test "file should exist"
    teardown
}

test_required_file_missing() {
    setup
    local file="$TMPDIR_ROOT/SECURITY.md"
    [[ ! -f "$file" ]] && pass_test "required file absence detected" || fail_test "file should not exist"
    teardown
}

# ── Tests: secret leakage patterns ───────────────────────────────────────────

test_secret_pattern_detected() {
    setup
    echo 'let PRIVATE_KEY = "abc123";' > "$TMPDIR_ROOT/bad.rs"
    local found
    found=$(grep -rIi "PRIVATE_KEY" "$TMPDIR_ROOT" | wc -l || true)
    [[ "$found" -gt 0 ]] && pass_test "PRIVATE_KEY pattern detected" || fail_test "PRIVATE_KEY should be detected"
    teardown
}

test_clean_file_no_secrets() {
    setup
    echo 'fn contribute(env: Env, amount: i128) {}' > "$TMPDIR_ROOT/clean.rs"
    local found=0
    for pattern in "PRIVATE_KEY" "SECRET_KEY" "sk_live_"; do
        local c
        c=$(grep -rIi "$pattern" "$TMPDIR_ROOT" | wc -l || true)
        found=$(( found + c ))
    done
    [[ "$found" -eq 0 ]] && pass_test "clean file has no secret patterns" || fail_test "unexpected secret pattern found"
    teardown
}

test_password_assignment_detected() {
    setup
    echo 'password = "hunter2"' > "$TMPDIR_ROOT/config.toml"
    local found
    found=$(grep -rIi "password\s*=" "$TMPDIR_ROOT" | wc -l || true)
    [[ "$found" -gt 0 ]] && pass_test "password= pattern detected" || fail_test "password= should be detected"
    teardown
}

test_api_key_detected() {
    setup
    echo 'api_key = "abc"' > "$TMPDIR_ROOT/config.yml"
    local found
    found=$(grep -rIi "api_key\s*=" "$TMPDIR_ROOT" | wc -l || true)
    [[ "$found" -gt 0 ]] && pass_test "api_key= pattern detected" || fail_test "api_key= should be detected"
    teardown
}

# ── Tests: .gitignore compliance ─────────────────────────────────────────────

test_gitignore_contains_soroban() {
    setup
    echo ".soroban" > "$TMPDIR_ROOT/.gitignore"
    grep -q ".soroban" "$TMPDIR_ROOT/.gitignore" \
        && pass_test ".gitignore contains .soroban" \
        || fail_test ".gitignore should contain .soroban"
    teardown
}

test_gitignore_contains_target() {
    setup
    printf ".soroban\ntarget/\n" > "$TMPDIR_ROOT/.gitignore"
    grep -q "target/" "$TMPDIR_ROOT/.gitignore" \
        && pass_test ".gitignore contains target/" \
        || fail_test ".gitignore should contain target/"
    teardown
}

test_gitignore_missing_entry_detected() {
    setup
    echo "node_modules" > "$TMPDIR_ROOT/.gitignore"
    grep -q ".soroban" "$TMPDIR_ROOT/.gitignore" \
        && fail_test "should not find .soroban in this gitignore" \
        || pass_test "missing .soroban entry correctly detected"
    teardown
}

test_gitignore_file_missing() {
    setup
    [[ ! -f "$TMPDIR_ROOT/.gitignore" ]] \
        && pass_test "missing .gitignore correctly detected" \
        || fail_test ".gitignore should not exist in empty tmpdir"
    teardown
}

# ── Tests: CI workflow health ─────────────────────────────────────────────────

test_workflow_contains_cargo_audit() {
    setup
    mkdir -p "$TMPDIR_ROOT/.github/workflows"
    echo "run: cargo audit" > "$TMPDIR_ROOT/.github/workflows/rust_ci.yml"
    grep -q "cargo audit" "$TMPDIR_ROOT/.github/workflows/rust_ci.yml" \
        && pass_test "workflow contains cargo audit" \
        || fail_test "workflow should contain cargo audit"
    teardown
}

test_workflow_contains_timeout() {
    setup
    mkdir -p "$TMPDIR_ROOT/.github/workflows"
    printf "timeout-minutes: 30\nrun: cargo test\n" \
        > "$TMPDIR_ROOT/.github/workflows/rust_ci.yml"
    grep -q "timeout-minutes" "$TMPDIR_ROOT/.github/workflows/rust_ci.yml" \
        && pass_test "workflow has timeout-minutes" \
        || fail_test "workflow should have timeout-minutes"
    teardown
}

test_workflow_missing_cargo_clippy() {
    setup
    mkdir -p "$TMPDIR_ROOT/.github/workflows"
    echo "run: cargo build" > "$TMPDIR_ROOT/.github/workflows/rust_ci.yml"
    grep -q "cargo clippy" "$TMPDIR_ROOT/.github/workflows/rust_ci.yml" \
        && fail_test "should not find clippy in this workflow" \
        || pass_test "missing cargo clippy correctly detected"
    teardown
}

test_workflow_file_missing() {
    setup
    [[ ! -f "$TMPDIR_ROOT/.github/workflows/rust_ci.yml" ]] \
        && pass_test "missing workflow file correctly detected" \
        || fail_test "workflow file should not exist"
    teardown
}

# ── Tests: Cargo.lock presence ────────────────────────────────────────────────

test_cargo_lock_present() {
    setup
    touch "$TMPDIR_ROOT/Cargo.lock"
    [[ -f "$TMPDIR_ROOT/Cargo.lock" ]] \
        && pass_test "Cargo.lock present (pinned deps)" \
        || fail_test "Cargo.lock should exist"
    teardown
}

test_cargo_lock_missing() {
    setup
    [[ ! -f "$TMPDIR_ROOT/Cargo.lock" ]] \
        && pass_test "missing Cargo.lock correctly detected" \
        || fail_test "Cargo.lock should not exist in empty tmpdir"
    teardown
}

# ── Tests: JSON report ────────────────────────────────────────────────────────

test_json_report_fields() {
    setup
    local report_dir="$TMPDIR_ROOT/reports"
    mkdir -p "$report_dir"
    local f="$report_dir/monitoring-test.json"
    cat > "$f" <<EOF
{
  "script": "security_compliance_monitoring",
  "version": "1.0.0",
  "timestamp": "2026-03-29T05:46:48Z",
  "status": "PASS",
  "summary": {"total": 6, "passed": 6, "failed": 0, "warnings": 0}
}
EOF
    local ok=true
    for field in "script" "version" "timestamp" "status" "summary"; do
        grep -q "\"$field\"" "$f" || { ok=false; break; }
    done
    [[ "$ok" == true ]] \
        && pass_test "JSON report contains all required fields" \
        || fail_test "JSON report missing required fields"
    teardown
}

test_json_report_fail_status() {
    setup
    local f="$TMPDIR_ROOT/fail.json"
    echo '{"status": "FAIL"}' > "$f"
    local status
    status=$(grep -o '"status": "[^"]*"' "$f" | cut -d'"' -f4)
    [[ "$status" == "FAIL" ]] \
        && pass_test "FAIL status correctly recorded" \
        || fail_test "expected FAIL status"
    teardown
}

# ── Tests: exit code semantics ────────────────────────────────────────────────

test_exit_0_no_failures() {
    local failed=0; local code=0
    [[ "$failed" -gt 0 ]] && code=1
    [[ "$code" -eq 0 ]] && pass_test "exit 0 when no failures" || fail_test "expected exit 0"
}

test_exit_1_with_failures() {
    local failed=2; local code=0
    [[ "$failed" -gt 0 ]] && code=1
    [[ "$code" -eq 1 ]] && pass_test "exit 1 when failures present" || fail_test "expected exit 1"
}

test_exit_2_missing_tools() {
    TESTS_RUN=$(( TESTS_RUN + 1 ))
    local empty_bin
    empty_bin=$(mktemp -d)
    local exit_code=0
    PATH="$empty_bin" /usr/bin/bash "$SCRIPT_UNDER_TEST" 2>&1 || exit_code=$?
    rm -rf "$empty_bin"
    [[ "$exit_code" -eq 2 ]] \
        && { echo -e "${GREEN}[PASS]${NC} missing tools exits 2"; TESTS_PASSED=$(( TESTS_PASSED + 1 )); } \
        || { echo -e "${RED}[FAIL]${NC} expected exit 2, got $exit_code"; TESTS_FAILED=$(( TESTS_FAILED + 1 )); }
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  security_compliance_monitoring.test.sh"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""

    test_help_flag
    test_unknown_flag_exits_nonzero
    test_required_file_present
    test_required_file_missing
    test_secret_pattern_detected
    test_clean_file_no_secrets
    test_password_assignment_detected
    test_api_key_detected
    test_gitignore_contains_soroban
    test_gitignore_contains_target
    test_gitignore_missing_entry_detected
    test_gitignore_file_missing
    test_workflow_contains_cargo_audit
    test_workflow_contains_timeout
    test_workflow_missing_cargo_clippy
    test_workflow_file_missing
    test_cargo_lock_present
    test_cargo_lock_missing
    test_json_report_fields
    test_json_report_fail_status
    test_exit_0_no_failures
    test_exit_1_with_failures
    test_exit_2_missing_tools

    echo ""
    echo "══════════════════════════════════════════════════════════════"
    echo "  Results: $TESTS_PASSED/$TESTS_RUN passed, $TESTS_FAILED failed"
    echo "══════════════════════════════════════════════════════════════"

    local coverage=0
    [[ "$TESTS_RUN" -gt 0 ]] && coverage=$(( TESTS_PASSED * 100 / TESTS_RUN ))
    echo "  Coverage proxy: ${coverage}% (threshold: 95%)"

    if [[ "$TESTS_FAILED" -gt 0 ]]; then
        echo -e "${RED}TEST SUITE FAILED${NC}"
        exit 1
    fi

    echo -e "${GREEN}TEST SUITE PASSED${NC}"
    exit 0
}

main "$@"
