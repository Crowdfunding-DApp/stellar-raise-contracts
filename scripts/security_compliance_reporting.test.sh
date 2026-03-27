#!/usr/bin/env bash
# @title   security_compliance_reporting.test.sh
# @notice  Comprehensive unit tests for security_compliance_reporting.sh.
#          Uses a lightweight bash harness with no external test framework.
# @dev     Run: bash scripts/security_compliance_reporting.test.sh
#          Exit 0 = all tests passed.
#          Coverage target: ≥95% of functions and branches.
#
# @security Tests do NOT write real secrets to disk.
#           All temporary files are cleaned up on EXIT.

set -euo pipefail

SCRIPT="$(dirname "$0")/security_compliance_reporting.sh"
[[ -f "$SCRIPT" ]] || { echo "ERROR: script not found at $SCRIPT"; exit 1; }

PASS=0
FAIL=0
_TEST_TMPFILES=()

# ── Harness cleanup ───────────────────────────────────────────────────────────

_harness_cleanup() {
  for f in "${_TEST_TMPFILES[@]:-}"; do
    [[ -f "$f" ]] && rm -f "$f"
    [[ -d "$f" ]] && rm -rf "$f"
  done
}
trap _harness_cleanup EXIT

make_test_tmp() {
  local f; f="$(mktemp)"
  _TEST_TMPFILES+=("$f")
  echo "$f"
}

make_test_dir() {
  local d; d="$(mktemp -d)"
  _TEST_TMPFILES+=("$d")
  echo "$d"
}

# ── Assertion helpers ─────────────────────────────────────────────────────────

assert_exit() {
  local desc="$1" expected="$2"; shift 2
  local actual=0
  "$@" &>/dev/null || actual=$?
  if [[ "$actual" -eq "$expected" ]]; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (expected exit $expected, got $actual)"
    (( FAIL++ )) || true
  fi
}

assert_output_contains() {
  local desc="$1" pattern="$2"; shift 2
  local out actual=0
  out="$("$@" 2>&1)" || actual=$?
  if echo "$out" | grep -q "$pattern"; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (pattern '$pattern' not found)"
    (( FAIL++ )) || true
  fi
}

assert_output_not_contains() {
  local desc="$1" pattern="$2"; shift 2
  local out actual=0
  out="$("$@" 2>&1)" || actual=$?
  if echo "$out" | grep -q "$pattern"; then
    echo "  FAIL  $desc  (pattern '$pattern' unexpectedly found)"
    (( FAIL++ )) || true
  else
    echo "  PASS  $desc"
    (( PASS++ )) || true
  fi
}

assert_file_contains() {
  local desc="$1" file="$2" pattern="$3"
  if grep -q "$pattern" "$file" 2>/dev/null; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (pattern '$pattern' not in $file)"
    (( FAIL++ )) || true
  fi
}

assert_equals() {
  local desc="$1" expected="$2" actual="$3"
  if [[ "$expected" == "$actual" ]]; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (expected '$expected', got '$actual')"
    (( FAIL++ )) || true
  fi
}

# ── Source helpers (stub main) ────────────────────────────────────────────────

# shellcheck source=/dev/null
eval "$(sed 's/^main "\$@"$/: # main stubbed/' "$SCRIPT")"

# ── Helper: inline function env for subshell tests ────────────────────────────

COMMON_FUNCS="$(declare -f log die tool_available require_tool json_escape make_tmp \
  check_cargo_audit check_secret_leaks check_hardcoded_secrets check_file_permissions \
  check_licence check_cicd_pinning aggregate_severity assemble_report _cleanup 2>/dev/null || true)"

# ── Tests: tool_available ─────────────────────────────────────────────────────

echo ""
echo "=== tool_available ==="

assert_exit "returns 0 for bash (always present)" 0 \
  bash -c "$COMMON_FUNCS; tool_available bash"

assert_exit "returns 1 for __nonexistent_tool__" 1 \
  bash -c "$COMMON_FUNCS; tool_available __nonexistent_tool__"

# ── Tests: require_tool ───────────────────────────────────────────────────────

echo ""
echo "=== require_tool ==="

assert_exit "passes for present tool" 0 \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; require_tool bash"

assert_exit "exits 1 for missing tool" 1 \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; require_tool __no_such_tool__"

assert_output_contains "error message mentions tool name" "__no_such_tool__" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; require_tool __no_such_tool__" || true

# ── Tests: json_escape ────────────────────────────────────────────────────────

echo ""
echo "=== json_escape ==="

assert_equals "escapes double quotes" \
  'say \"hello\"' \
  "$(bash -c "$COMMON_FUNCS; json_escape 'say \"hello\"'")"

assert_equals "escapes backslash" \
  'a\\b' \
  "$(bash -c "$COMMON_FUNCS; json_escape 'a\\b'")"

assert_equals "plain string unchanged" \
  "hello" \
  "$(bash -c "$COMMON_FUNCS; json_escape 'hello'")"

assert_equals "empty string returns empty" \
  "" \
  "$(bash -c "$COMMON_FUNCS; json_escape ''")"

# ── Tests: log ────────────────────────────────────────────────────────────────

echo ""
echo "=== log ==="

assert_output_contains "log includes level tag [INFO]" "\[INFO\]" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; log INFO 'test message'"

assert_output_contains "log includes message text" "test message" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; log INFO 'test message'"

assert_output_contains "log includes [WARN] level" "\[WARN\]" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; log WARN 'something'"

assert_output_contains "log includes [ERROR] level" "\[ERROR\]" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; log ERROR 'bad thing'"

assert_exit "log appends to REPORT_LOG file" 0 \
  bash -c "$COMMON_FUNCS
    TMP=\$(mktemp); REPORT_LOG=\"\$TMP\"
    log INFO 'written to file'
    grep -q 'written to file' \"\$TMP\"
    rm -f \"\$TMP\""

# ── Tests: die ────────────────────────────────────────────────────────────────

echo ""
echo "=== die ==="

assert_exit "die exits with code 1" 1 \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; die 1 'msg'"

assert_exit "die exits with code 2" 2 \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; die 2 'msg'"

assert_exit "die exits with code 5" 5 \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; die 5 'msg'"

assert_output_contains "die logs ERROR level" "\[ERROR\]" \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; die 1 'boom'" || true

# ── Tests: aggregate_severity ─────────────────────────────────────────────────

echo ""
echo "=== aggregate_severity ==="

assert_equals "all pass → pass" "pass" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity pass pass pass")"

assert_equals "one warn → warn" "warn" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity pass warn pass")"

assert_equals "one fail → fail" "fail" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity pass warn fail")"

assert_equals "fail overrides warn" "fail" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity warn fail warn")"

assert_equals "skipped treated as pass" "pass" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity skipped skipped")"

assert_equals "single pass → pass" "pass" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity pass")"

assert_equals "single fail → fail" "fail" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity fail")"

# ── Tests: check_licence ──────────────────────────────────────────────────────

echo ""
echo "=== check_licence ==="

assert_output_contains "detects Apache-2.0 licence" "Apache-2.0" \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'Apache License Version 2.0' > LICENSE
    check_licence"

assert_output_contains "detects MIT licence" "MIT" \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'MIT License' > LICENSE
    check_licence"

assert_output_contains "detects GPL licence" "GPL" \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'GNU General Public License' > LICENSE
    check_licence"

assert_output_contains "detects BSD licence" "BSD" \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'BSD License' > LICENSE
    check_licence"

assert_output_contains "warns when no LICENSE file" '"status":"warn"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    check_licence"

assert_output_contains "passes when LICENSE exists" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'MIT License' > LICENSE
    check_licence"

assert_output_contains "accepts LICENSE.md filename" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'MIT License' > LICENSE.md
    check_licence"

assert_output_contains "unknown licence type returns UNKNOWN" "UNKNOWN" \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'Some custom licence text' > LICENSE
    check_licence"

# ── Tests: check_file_permissions ────────────────────────────────────────────

echo ""
echo "=== check_file_permissions ==="

assert_output_contains "pass when no world-writable scripts" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo '#!/bin/bash' > safe.sh
    chmod 755 safe.sh
    check_file_permissions"

assert_output_contains "warn when world-writable script found" '"status":"warn"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo '#!/bin/bash' > bad.sh
    chmod 777 bad.sh
    check_file_permissions"

assert_output_contains "findings list is empty when all safe" '"findings":\[\]' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo '#!/bin/bash' > safe.sh
    chmod 755 safe.sh
    check_file_permissions"

# ── Tests: check_hardcoded_secrets ───────────────────────────────────────────

echo ""
echo "=== check_hardcoded_secrets ==="

assert_output_contains "pass when no secret patterns found" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    # Run in a clean temp dir with no matching files
    cd \$(mktemp -d)
    git init -q
    check_hardcoded_secrets"

assert_output_contains "returns match_count field" 'match_count' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    git init -q
    check_hardcoded_secrets"

# ── Tests: check_cargo_audit (stubbed) ───────────────────────────────────────

echo ""
echo "=== check_cargo_audit ==="

assert_output_contains "skipped when cargo-audit absent" '"status":"skipped"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    # Override tool_available to report cargo-audit as missing
    tool_available() { [[ \"\$1\" != 'cargo-audit' ]]; }
    check_cargo_audit"

assert_output_contains "skipped message mentions cargo-audit" 'cargo-audit' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    tool_available() { [[ \"\$1\" != 'cargo-audit' ]]; }
    check_cargo_audit"

assert_output_contains "pass when cargo-audit finds 0 vulns" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    _TMPFILES=()
    FAKE_BIN=\$(mktemp -d)
    cat > \"\$FAKE_BIN/cargo-audit\" << 'STUB'
#!/bin/bash
printf '{\"vulnerabilities\":{\"count\":0,\"list\":[]}}'
exit 0
STUB
    chmod +x \"\$FAKE_BIN/cargo-audit\"
    PATH=\"\$FAKE_BIN:\$PATH\"
    check_cargo_audit
    rm -rf \"\$FAKE_BIN\""

assert_output_contains "fail when cargo-audit finds vulns" '"status":"fail"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    _TMPFILES=()
    FAKE_BIN=\$(mktemp -d)
    cat > \"\$FAKE_BIN/cargo-audit\" << 'STUB'
#!/bin/bash
printf '{\"vulnerabilities\":{\"count\":2,\"list\":[]}}'
exit 1
STUB
    chmod +x \"\$FAKE_BIN/cargo-audit\"
    PATH=\"\$FAKE_BIN:\$PATH\"
    check_cargo_audit
    rm -rf \"\$FAKE_BIN\""

# ── Tests: check_secret_leaks (stubbed) ──────────────────────────────────────

echo ""
echo "=== check_secret_leaks ==="

assert_output_contains "skipped when gitleaks absent" '"status":"skipped"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    tool_available() { [[ \"\$1\" != 'gitleaks' ]]; }
    check_secret_leaks"

assert_output_contains "skipped message mentions gitleaks" 'gitleaks' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    tool_available() { [[ \"\$1\" != 'gitleaks' ]]; }
    check_secret_leaks"

assert_output_contains "pass when gitleaks finds 0 leaks" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    _TMPFILES=()
    tool_available() { return 0; }
    gitleaks() { echo '[]'; return 0; }
    check_secret_leaks"

# ── Tests: check_cicd_pinning ─────────────────────────────────────────────────

echo ""
echo "=== check_cicd_pinning ==="

assert_output_contains "skipped when no workflows dir" '"status":"skipped"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    check_cicd_pinning"

assert_output_contains "pass when all actions are pinned to SHA/tag" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: actions/checkout@v4' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

assert_output_contains "warn when action pinned to main branch" '"status":"warn"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: actions/checkout@main' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

assert_output_contains "warn when action pinned to master branch" '"status":"warn"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: some/action@master' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

assert_output_contains "warn when action pinned to HEAD" '"status":"warn"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: some/action@HEAD' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

# ── Tests: assemble_report ────────────────────────────────────────────────────

echo ""
echo "=== assemble_report ==="

_test_assemble_report() {
  local TMP_REPORT TMP_LOG TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")
  TMP_REPORT="$TMP_DIR/report.json"
  TMP_LOG="$TMP_DIR/report.log"

  bash -c "$COMMON_FUNCS
    REPORT_FILE='$TMP_REPORT'
    REPORT_LOG='$TMP_LOG'
    assemble_report \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      'pass'" &>/dev/null

  [[ -f "$TMP_REPORT" ]]
}

assert_exit "assemble_report creates report file" 0 _test_assemble_report

_test_report_has_overall() {
  local TMP_REPORT TMP_LOG TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")
  TMP_REPORT="$TMP_DIR/report.json"
  TMP_LOG="$TMP_DIR/report.log"

  bash -c "$COMMON_FUNCS
    REPORT_FILE='$TMP_REPORT'
    REPORT_LOG='$TMP_LOG'
    assemble_report \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      'pass'" &>/dev/null

  grep -q '"overall_status"' "$TMP_REPORT"
}

assert_exit "report contains overall_status field" 0 _test_report_has_overall

_test_report_has_checks() {
  local TMP_REPORT TMP_LOG TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")
  TMP_REPORT="$TMP_DIR/report.json"
  TMP_LOG="$TMP_DIR/report.log"

  bash -c "$COMMON_FUNCS
    REPORT_FILE='$TMP_REPORT'
    REPORT_LOG='$TMP_LOG'
    assemble_report \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      'pass'" &>/dev/null

  grep -q '"checks"' "$TMP_REPORT"
}

assert_exit "report contains checks section" 0 _test_report_has_checks

_test_report_has_git_sha() {
  local TMP_REPORT TMP_LOG TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")
  TMP_REPORT="$TMP_DIR/report.json"
  TMP_LOG="$TMP_DIR/report.log"

  bash -c "$COMMON_FUNCS
    REPORT_FILE='$TMP_REPORT'
    REPORT_LOG='$TMP_LOG'
    assemble_report \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      '{\"status\":\"pass\"}' \
      'pass'" &>/dev/null

  grep -q '"git_sha"' "$TMP_REPORT"
}

assert_exit "report contains git_sha field" 0 _test_report_has_git_sha

# ── Tests: main integration ───────────────────────────────────────────────────

echo ""
echo "=== main (integration) ==="

_test_main_creates_report() {
  local TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")

  REPORT_FILE="$TMP_DIR/report.json" \
  REPORT_LOG="$TMP_DIR/report.log" \
  FAIL_ON_HIGH="false" \
    bash scripts/security_compliance_reporting.sh &>/dev/null || true

  [[ -f "$TMP_DIR/report.json" ]]
}

assert_exit "main creates report file" 0 _test_main_creates_report

_test_main_report_valid_json_structure() {
  local TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")

  REPORT_FILE="$TMP_DIR/report.json" \
  REPORT_LOG="$TMP_DIR/report.log" \
  FAIL_ON_HIGH="false" \
    bash scripts/security_compliance_reporting.sh &>/dev/null || true

  grep -q '"overall_status"' "$TMP_DIR/report.json" && \
  grep -q '"checks"' "$TMP_DIR/report.json" && \
  grep -q '"report_version"' "$TMP_DIR/report.json"
}

assert_exit "main report has required JSON fields" 0 _test_main_report_valid_json_structure

_test_main_exits_0_when_fail_on_high_false() {
  local TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")

  REPORT_FILE="$TMP_DIR/report.json" \
  REPORT_LOG="$TMP_DIR/report.log" \
  FAIL_ON_HIGH="false" \
    bash scripts/security_compliance_reporting.sh &>/dev/null
}

assert_exit "main exits 0 when FAIL_ON_HIGH=false" 0 _test_main_exits_0_when_fail_on_high_false

_test_main_truncates_log() {
  local TMP_DIR
  TMP_DIR="$(mktemp -d)"
  _TEST_TMPFILES+=("$TMP_DIR")
  echo "stale content" > "$TMP_DIR/report.log"

  REPORT_FILE="$TMP_DIR/report.json" \
  REPORT_LOG="$TMP_DIR/report.log" \
  FAIL_ON_HIGH="false" \
    bash scripts/security_compliance_reporting.sh &>/dev/null || true

  ! grep -q "stale content" "$TMP_DIR/report.log"
}

assert_exit "main truncates log at start of each run" 0 _test_main_truncates_log

_test_main_exits_2_on_blocking_findings() {
  local TMP_DIR WRAPPER
  TMP_DIR="$(mktemp -d)"
  WRAPPER="$(mktemp --suffix=.sh)"
  _TEST_TMPFILES+=("$TMP_DIR" "$WRAPPER")

  # Write a wrapper that sources the script with main stubbed, overrides checks, then calls main
  cat > "$WRAPPER" << WRAP
#!/usr/bin/env bash
set -euo pipefail
REPORT_FILE="$TMP_DIR/report.json"
REPORT_LOG="$TMP_DIR/report.log"
FAIL_ON_HIGH="true"
eval "\$(sed 's/^main \"\\\$@\"\$/: # stubbed/' '$SCRIPT')"
check_cargo_audit()       { echo '{"status":"fail","vulnerability_count":1,"findings":[]}'; }
check_secret_leaks()      { echo '{"status":"pass","leak_count":0,"findings":[]}'; }
check_hardcoded_secrets() { echo '{"status":"pass","match_count":0,"findings":[]}'; }
check_file_permissions()  { echo '{"status":"pass","world_writable_scripts":0,"findings":[]}'; }
check_licence()           { echo '{"status":"pass","licence_file":"LICENSE","licence_type":"MIT"}'; }
check_cicd_pinning()      { echo '{"status":"pass","unpinned_count":0,"findings":[]}'; }
main
WRAP

  bash "$WRAPPER" &>/dev/null
}

assert_exit "main exits 2 when FAIL_ON_HIGH=true and findings exist" 2 \
  _test_main_exits_2_on_blocking_findings

# ── Tests: edge cases ─────────────────────────────────────────────────────────

echo ""
echo "=== edge cases ==="

assert_output_contains "json_escape handles tab character" '\\t' \
  bash -c "$COMMON_FUNCS; json_escape $'a\tb'"

assert_output_contains "log handles message with special chars" 'hello & world' \
  bash -c "$COMMON_FUNCS; REPORT_LOG=/dev/null; log INFO 'hello & world'"

assert_equals "aggregate_severity with no args returns pass" "pass" \
  "$(bash -c "$COMMON_FUNCS; aggregate_severity")"

assert_output_contains "check_licence handles LICENSE.txt" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    cd \$(mktemp -d)
    echo 'MIT License' > LICENSE.txt
    check_licence"

assert_output_contains "check_cicd_pinning pass for semver tag" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: actions/checkout@v3.5.2' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

assert_output_contains "check_cicd_pinning pass for SHA pin" '"status":"pass"' \
  bash -c "$COMMON_FUNCS
    REPORT_LOG=/dev/null
    D=\$(mktemp -d)
    mkdir -p \"\$D/.github/workflows\"
    echo 'uses: actions/checkout@a81bbbf8298c0fa03ea29cdc473d45769f953675' > \"\$D/.github/workflows/ci.yml\"
    cd \"\$D\"
    check_cicd_pinning"

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Results: $PASS passed, $FAIL failed"
TOTAL=$(( PASS + FAIL ))
if [[ "$TOTAL" -gt 0 ]]; then
  COVERAGE=$(( PASS * 100 / TOTAL ))
  echo "Coverage proxy: $COVERAGE% ($PASS/$TOTAL assertions passing)"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
[[ "$FAIL" -eq 0 ]] || exit 1
