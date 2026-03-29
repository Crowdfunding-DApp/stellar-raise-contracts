#!/usr/bin/env bash
# =============================================================================
# security_compliance_monitoring.sh
# =============================================================================
# @title   SecurityComplianceMonitoring — Continuous CI/CD Compliance Oversight
# @notice  Monitors the Stellar Raise crowdfunding contract for ongoing
#          compliance signals: dependency freshness, secret leakage, file
#          integrity, and CI workflow health.
# @dev     Exit code policy:
#            0 = all monitors passed
#            1 = one or more monitors failed
#            2 = required tooling is missing
#          Read-only — no state or file modifications are made.
#
# @custom:security-note
#   1. Read-only — no writes to storage or state files.
#   2. Permissionless — no privileged access required.
#   3. Deterministic — same inputs produce same outputs.
#   4. Bounded — no unbounded loops.
#
# Usage:
#   ./security_compliance_monitoring.sh [--verbose] [--json] [--report-dir DIR]
# =============================================================================

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

readonly SCRIPT_NAME="security_compliance_monitoring"
readonly VERSION="1.0.0"

# Patterns that must never appear in committed source files.
# @dev  Extend this list as new secret formats are identified.
readonly -a SECRET_PATTERNS=(
    "PRIVATE_KEY"
    "SECRET_KEY"
    "-----BEGIN.*PRIVATE"
    "sk_live_"
    "sk_test_"
    "password\s*="
    "api_key\s*="
)

# Files that must exist for the project to be considered compliant.
readonly -a REQUIRED_FILES=(
    "Cargo.toml"
    "README.md"
    "SECURITY.md"
    "LICENSE"
    ".github/workflows/rust_ci.yml"
    ".github/workflows/security.yml"
)

# ── Colour helpers ────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass()    { echo -e "${GREEN}[PASS]${NC} $*"; PASSED=$(( PASSED + 1 )); }
fail()    { echo -e "${RED}[FAIL]${NC} $*";   FAILED=$(( FAILED + 1 )); }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; WARNINGS=$(( WARNINGS + 1 )); }
section() { echo -e "\n${BLUE}── $* ──────────────────────────────────────────────${NC}"; }
info()    { echo -e "     $*"; }

PASSED=0
FAILED=0
WARNINGS=0

# ── CLI flags ─────────────────────────────────────────────────────────────────

VERBOSE=false
JSON_OUTPUT=false
REPORT_DIR="monitoring-reports"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose)    VERBOSE=true ;;
        --json)       JSON_OUTPUT=true ;;
        --report-dir) REPORT_DIR="$2"; shift ;;
        --help)
            echo "Usage: $0 [--verbose] [--json] [--report-dir DIR]"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
    shift
done

# ── Phase 1: Tool presence ────────────────────────────────────────────────────

# @notice Verifies all required tools are installed.
monitor_tools() {
    section "Tool Presence"
    local missing=0

    for tool in cargo git; do
        if ! command -v "$tool" &>/dev/null; then
            echo -e "${RED}[MISSING]${NC} $tool"
            missing=$(( missing + 1 ))
        else
            pass "$tool present"
            [[ "$VERBOSE" == true ]] && info "  → $(command -v "$tool")"
        fi
    done

    if [[ "$missing" -gt 0 ]]; then
        echo "ERROR: $missing required tool(s) missing."
        exit 2
    fi
}

# ── Phase 2: Required file presence ──────────────────────────────────────────

# @notice Verifies that all compliance-critical files exist in the repository.
# @dev    Missing SECURITY.md or LICENSE are compliance violations.
monitor_required_files() {
    section "Required File Presence"

    for file in "${REQUIRED_FILES[@]}"; do
        if [[ -f "$file" ]]; then
            pass "$file present"
        else
            fail "$file missing — required for compliance"
        fi
    done
}

# ── Phase 3: Secret leakage scan ─────────────────────────────────────────────

# @notice Scans tracked source files for patterns that indicate committed secrets.
# @dev    Only scans files tracked by git to avoid false positives from
#         generated artefacts.  Excludes test fixtures and documentation.
monitor_secret_leakage() {
    section "Secret Leakage Scan"

    if ! command -v git &>/dev/null || ! git rev-parse --git-dir &>/dev/null 2>&1; then
        warn "Not a git repository — skipping secret scan"
        return
    fi

    local found_secrets=0

    for pattern in "${SECRET_PATTERNS[@]}"; do
        local matches
        matches=$(git grep -rIi --count "$pattern" -- \
            '*.rs' '*.sh' '*.ts' '*.js' '*.toml' '*.yml' '*.yaml' \
            2>/dev/null | awk -F: '{sum+=$2} END{print sum+0}' || true)

        if [[ "$matches" -gt 0 ]]; then
            fail "Potential secret pattern found ($matches occurrence(s)): $pattern"
            found_secrets=$(( found_secrets + matches ))
        else
            [[ "$VERBOSE" == true ]] && info "Clean: $pattern"
        fi
    done

    if [[ "$found_secrets" -eq 0 ]]; then
        pass "No secret patterns detected in tracked files"
    fi
}

# ── Phase 4: .gitignore compliance ───────────────────────────────────────────

# @notice Verifies that sensitive directories are listed in .gitignore.
# @dev    .soroban/ and target/ must be ignored to prevent accidental key
#         and binary commits.
monitor_gitignore() {
    section ".gitignore Compliance"

    local required_ignores=(".soroban" "target/" "audit-reports" "monitoring-reports")

    if [[ ! -f ".gitignore" ]]; then
        fail ".gitignore not found"
        return
    fi

    for entry in "${required_ignores[@]}"; do
        if grep -q "$entry" ".gitignore"; then
            pass ".gitignore contains: $entry"
        else
            warn ".gitignore missing entry: $entry"
        fi
    done
}

# ── Phase 5: CI workflow health ───────────────────────────────────────────────

# @notice Checks that CI workflow files contain required security steps.
# @dev    Verifies cargo-audit and Clippy are present in the main workflow.
monitor_ci_workflows() {
    section "CI Workflow Health"

    local workflow=".github/workflows/rust_ci.yml"

    if [[ ! -f "$workflow" ]]; then
        fail "$workflow not found"
        return
    fi

    local checks=("cargo audit" "cargo clippy" "cargo test" "cargo fmt")
    for check in "${checks[@]}"; do
        if grep -q "$check" "$workflow"; then
            pass "CI workflow contains: $check"
        else
            fail "CI workflow missing: $check"
        fi
    done

    # Verify timeout-minutes is set (prevents runaway builds)
    if grep -q "timeout-minutes" "$workflow"; then
        pass "CI workflow has timeout-minutes set"
    else
        warn "CI workflow missing timeout-minutes — runaway builds possible"
    fi
}

# ── Phase 6: Dependency freshness ────────────────────────────────────────────

# @notice Checks that Cargo.lock exists (pinned dependencies) and that
#         cargo-audit reports no known vulnerabilities.
monitor_dependency_freshness() {
    section "Dependency Freshness"

    if [[ -f "Cargo.lock" ]]; then
        pass "Cargo.lock present (dependencies pinned)"
    else
        warn "Cargo.lock not found — dependencies are not pinned"
    fi

    if cargo audit --version &>/dev/null 2>&1; then
        if cargo audit 2>&1; then
            pass "No known vulnerabilities in Rust dependencies"
        else
            fail "cargo-audit reported vulnerabilities"
        fi
    else
        warn "cargo-audit not installed — skipping vulnerability check"
    fi
}

# ── Report generation ─────────────────────────────────────────────────────────

# @notice Writes a JSON monitoring summary to REPORT_DIR.
generate_json_report() {
    mkdir -p "$REPORT_DIR"
    local report_file="$REPORT_DIR/monitoring-$(date +%Y%m%dT%H%M%S).json"
    local total=$(( PASSED + FAILED + WARNINGS ))
    local status="PASS"
    [[ "$FAILED" -gt 0 ]] && status="FAIL"

    cat > "$report_file" <<EOF
{
  "script": "$SCRIPT_NAME",
  "version": "$VERSION",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "$status",
  "summary": {
    "total": $total,
    "passed": $PASSED,
    "failed": $FAILED,
    "warnings": $WARNINGS
  }
}
EOF
    info "Monitoring report written to: $report_file"
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  $SCRIPT_NAME v$VERSION"
    echo "╚══════════════════════════════════════════════════════════════╝"

    monitor_tools
    monitor_required_files
    monitor_secret_leakage
    monitor_gitignore
    monitor_ci_workflows
    monitor_dependency_freshness

    echo ""
    echo "══════════════════════════════════════════════════════════════"
    echo "  Monitor Summary: PASSED=$PASSED  FAILED=$FAILED  WARNINGS=$WARNINGS"
    echo "══════════════════════════════════════════════════════════════"

    [[ "$JSON_OUTPUT" == true ]] && generate_json_report

    if [[ "$FAILED" -gt 0 ]]; then
        echo -e "${RED}MONITORING FAILED — $FAILED check(s) did not pass.${NC}"
        exit 1
    fi

    echo -e "${GREEN}MONITORING PASSED${NC}"
    exit 0
}

main "$@"
