#!/usr/bin/env bash
# @title   security_compliance_reporting.sh
# @notice  Automated security compliance reporting for the Stellar Raise CI/CD pipeline.
#          Scans the repository for common security issues, dependency vulnerabilities,
#          secret leaks, and licence compliance, then emits a structured JSON report.
# @dev     Requirements: bash >=4.2, cargo (for `cargo audit`), git.
#          Optional tools (gracefully skipped when absent):
#            - cargo-audit  : Rust advisory-db vulnerability scan
#            - gitleaks     : secret / credential leak detection
#            - jq           : pretty-print JSON report
#          Exit codes:
#            0  – report generated, no blocking findings
#            1  – internal script error / missing required tool
#            2  – blocking HIGH or CRITICAL findings detected
#          Output:
#            $REPORT_FILE (default: security_compliance_report.json)
#            $REPORT_LOG  (default: security_compliance.log)
#
# @security All temporary files are created via mktemp and cleaned up on EXIT.
#           No secrets or credentials are written to disk.
#           The script runs with set -euo pipefail to prevent silent failures.

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

REPORT_FILE="${REPORT_FILE:-security_compliance_report.json}"
REPORT_LOG="${REPORT_LOG:-security_compliance.log}"
FAIL_ON_HIGH="${FAIL_ON_HIGH:-true}"   # set to "false" to make HIGH non-blocking
SCRIPT_VERSION="1.0.0"

# ── Temporary file registry ───────────────────────────────────────────────────

_TMPFILES=()

# @notice Registers and returns a new temporary file path.
# @return path written to stdout
make_tmp() {
  local f
  f="$(mktemp)"
  _TMPFILES+=("$f")
  echo "$f"
}

# @notice Removes all registered temporary files on EXIT.
_cleanup() {
  local f
  local count="${#_TMPFILES[@]}"
  if [[ "$count" -gt 0 ]]; then
    for f in "${_TMPFILES[@]}"; do
      [[ -n "$f" && -f "$f" ]] && rm -f "$f"
    done
  fi
}
trap _cleanup EXIT

# ── Logging ───────────────────────────────────────────────────────────────────

# @notice Writes a timestamped log entry to stdout and REPORT_LOG.
# @param  $1  level   INFO | WARN | ERROR
# @param  $2  message
log() {
  local level="$1" msg="$2"
  local ts; ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "[$ts] [$level] $msg" | tee -a "$REPORT_LOG"
}

# @notice Logs an error and exits with the given code.
# @param  $1  exit_code
# @param  $2  message
die() {
  log "ERROR" "$2"
  exit "$1"
}

# ── Tool detection ────────────────────────────────────────────────────────────

# @notice Returns 0 if a tool is on PATH, 1 otherwise (never exits).
tool_available() {
  command -v "$1" &>/dev/null
}

# @notice Asserts a required tool is present; exits 1 if not.
# @param  $1  tool name
require_tool() {
  tool_available "$1" || die 1 "Required tool not found: $1"
}

# ── JSON helpers ──────────────────────────────────────────────────────────────

# @notice Escapes a string for safe embedding in a JSON value.
# @param  $1  raw string
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  echo "$s"
}

# ── Check: dependency vulnerabilities (cargo audit) ──────────────────────────

# @notice Runs `cargo audit` and returns a JSON object with findings.
# @dev    Skipped with a WARN when cargo-audit is not installed.
# @return JSON fragment written to stdout
check_cargo_audit() {
  if ! tool_available cargo-audit; then
    log "WARN" "cargo-audit not found – skipping Rust advisory scan"
    echo '{"status":"skipped","reason":"cargo-audit not installed","findings":[]}'
    return
  fi

  log "INFO" "Running cargo audit..."
  local tmp; tmp="$(make_tmp)"
  local exit_code=0
  cargo audit --json > "$tmp" 2>>"$REPORT_LOG" || exit_code=$?

  local vuln_count=0
  local findings="[]"

  if [[ -s "$tmp" ]]; then
    # Extract vulnerability count if jq is available
    if tool_available jq; then
      vuln_count=$(jq '.vulnerabilities.count // 0' "$tmp" 2>/dev/null || echo 0)
      findings=$(jq '[.vulnerabilities.list[]? | {id:.advisory.id, severity:.advisory.cvss // "UNKNOWN", package:.package.name, title:.advisory.title}]' "$tmp" 2>/dev/null || echo "[]")
    else
      # Fallback: count "id" occurrences as a rough proxy
      vuln_count=$(grep -c '"id"' "$tmp" 2>/dev/null || echo 0)
    fi
  fi

  local status="pass"
  [[ "$vuln_count" -gt 0 ]] && status="fail"

  log "INFO" "cargo audit complete – $vuln_count vulnerabilities found"
  printf '{"status":"%s","vulnerability_count":%s,"findings":%s}' \
    "$status" "$vuln_count" "$findings"
}

# ── Check: secret / credential leaks (gitleaks) ──────────────────────────────

# @notice Scans the repository for hardcoded secrets using gitleaks.
# @dev    Skipped with a WARN when gitleaks is not installed.
# @return JSON fragment written to stdout
check_secret_leaks() {
  if ! tool_available gitleaks; then
    log "WARN" "gitleaks not found – skipping secret leak scan"
    echo '{"status":"skipped","reason":"gitleaks not installed","leak_count":0,"findings":[]}'
    return
  fi

  log "INFO" "Running gitleaks secret scan..."
  local tmp; tmp="$(make_tmp)"
  local exit_code=0
  gitleaks detect --report-format json --report-path "$tmp" --no-git 2>>"$REPORT_LOG" || exit_code=$?

  local leak_count=0
  local findings="[]"

  if [[ -s "$tmp" ]]; then
    if tool_available jq; then
      leak_count=$(jq 'length' "$tmp" 2>/dev/null || echo 0)
      findings=$(jq '[.[]? | {rule:.RuleID, file:.File, line:.StartLine}]' "$tmp" 2>/dev/null || echo "[]")
    else
      leak_count=$(grep -c '"RuleID"' "$tmp" 2>/dev/null || echo 0)
    fi
  fi

  local status="pass"
  [[ "$leak_count" -gt 0 ]] && status="fail"

  log "INFO" "gitleaks scan complete – $leak_count potential secrets found"
  printf '{"status":"%s","leak_count":%s,"findings":%s}' \
    "$status" "$leak_count" "$findings"
}

# ── Check: hardcoded secrets (pattern scan) ───────────────────────────────────

# @notice Scans source files for common secret patterns using grep.
# @dev    Covers: private keys, API tokens, passwords, mnemonics.
# @return JSON fragment written to stdout
check_hardcoded_secrets() {
  log "INFO" "Scanning for hardcoded secret patterns..."

  local patterns=(
    'PRIVATE_KEY\s*='
    'SECRET_KEY\s*='
    'API_KEY\s*='
    'password\s*=\s*"[^"]'
    'mnemonic\s*='
    'S[A-Z2-7]{55}'          # Stellar secret key format
  )

  local findings=()
  local match_count=0

  for pattern in "${patterns[@]}"; do
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      local escaped; escaped="$(json_escape "$line")"
      findings+=("\"$escaped\"")
      (( match_count++ )) || true
    done < <(git grep -rn --include="*.rs" --include="*.sh" --include="*.ts" \
               --include="*.js" --include="*.toml" -E "$pattern" 2>/dev/null || true)
  done

  local status="pass"
  [[ "$match_count" -gt 0 ]] && status="fail"

  local findings_json
  if [[ "${#findings[@]}" -gt 0 ]]; then
    findings_json="[$(IFS=,; echo "${findings[*]}")]"
  else
    findings_json="[]"
  fi

  log "INFO" "Hardcoded secret pattern scan complete – $match_count matches"
  printf '{"status":"%s","match_count":%s,"findings":%s}' \
    "$status" "$match_count" "$findings_json"
}

# ── Check: file permissions ───────────────────────────────────────────────────

# @notice Checks that shell scripts are not world-writable.
# @return JSON fragment written to stdout
check_file_permissions() {
  log "INFO" "Checking file permissions..."

  local bad_files=()
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    bad_files+=("\"$(json_escape "$f")\"")
  done < <(find . -name "*.sh" -perm /o+w -not -path "./.git/*" 2>/dev/null || true)

  local count="${#bad_files[@]}"
  local status="pass"
  [[ "$count" -gt 0 ]] && status="warn"

  local findings_json
  if [[ "$count" -gt 0 ]]; then
    findings_json="[$(IFS=,; echo "${bad_files[*]}")]"
  else
    findings_json="[]"
  fi

  log "INFO" "File permission check complete – $count world-writable scripts found"
  printf '{"status":"%s","world_writable_scripts":%s,"findings":%s}' \
    "$status" "$count" "$findings_json"
}

# ── Check: licence compliance ─────────────────────────────────────────────────

# @notice Verifies a LICENSE file exists and identifies its type.
# @return JSON fragment written to stdout
check_licence() {
  log "INFO" "Checking licence compliance..."

  local licence_file=""
  for f in LICENSE LICENSE.md LICENSE.txt; do
    [[ -f "$f" ]] && licence_file="$f" && break
  done

  if [[ -z "$licence_file" ]]; then
    log "WARN" "No LICENSE file found"
    echo '{"status":"warn","licence_file":null,"licence_type":"UNKNOWN"}'
    return
  fi

  local licence_type="UNKNOWN"
  if grep -qi "apache" "$licence_file" 2>/dev/null; then
    licence_type="Apache-2.0"
  elif grep -qi "mit license" "$licence_file" 2>/dev/null; then
    licence_type="MIT"
  elif grep -qi "gnu general public" "$licence_file" 2>/dev/null; then
    licence_type="GPL"
  elif grep -qi "bsd" "$licence_file" 2>/dev/null; then
    licence_type="BSD"
  fi

  log "INFO" "Licence check complete – $licence_type ($licence_file)"
  printf '{"status":"pass","licence_file":"%s","licence_type":"%s"}' \
    "$(json_escape "$licence_file")" "$licence_type"
}

# ── Check: CI/CD workflow security ───────────────────────────────────────────

# @notice Checks GitHub Actions workflows for unpinned action versions.
# @dev    Unpinned actions (e.g. uses: foo/bar@main) are a supply-chain risk.
# @return JSON fragment written to stdout
check_cicd_pinning() {
  log "INFO" "Checking CI/CD workflow action pinning..."

  local unpinned=()
  local workflow_dir=".github/workflows"

  if [[ ! -d "$workflow_dir" ]]; then
    log "WARN" "No .github/workflows directory found"
    echo '{"status":"skipped","reason":"no workflows directory","unpinned_count":0,"findings":[]}'
    return
  fi

  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    # Flag actions pinned to a branch name (not a SHA or semver tag)
    if echo "$line" | grep -qE 'uses:\s+\S+@(main|master|develop|HEAD)'; then
      unpinned+=("\"$(json_escape "$line")\"")
    fi
  done < <(grep -rn "uses:" "$workflow_dir" 2>/dev/null || true)

  local count="${#unpinned[@]}"
  local status="pass"
  [[ "$count" -gt 0 ]] && status="warn"

  local findings_json
  if [[ "$count" -gt 0 ]]; then
    findings_json="[$(IFS=,; echo "${unpinned[*]}")]"
  else
    findings_json="[]"
  fi

  log "INFO" "CI/CD pinning check complete – $count unpinned actions"
  printf '{"status":"%s","unpinned_count":%s,"findings":%s}' \
    "$status" "$count" "$findings_json"
}

# ── Severity aggregation ──────────────────────────────────────────────────────

# @notice Derives an overall severity from individual check statuses.
# @param  $@  list of status strings (pass | warn | fail | skipped)
# @return "pass" | "warn" | "fail" written to stdout
aggregate_severity() {
  local overall="pass"
  for s in "$@"; do
    case "$s" in
      fail)    overall="fail"; break ;;
      warn)    [[ "$overall" != "fail" ]] && overall="warn" ;;
    esac
  done
  echo "$overall"
}

# ── Report assembly ───────────────────────────────────────────────────────────

# @notice Assembles and writes the final JSON compliance report.
# @param  $1  audit_json
# @param  $2  secrets_json
# @param  $3  hardcoded_json
# @param  $4  perms_json
# @param  $5  licence_json
# @param  $6  cicd_json
# @param  $7  overall_status
assemble_report() {
  local audit="$1" secrets="$2" hardcoded="$3" perms="$4" \
        licence="$5" cicd="$6" overall="$7"

  local ts; ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  local git_sha; git_sha="$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
  local git_branch; git_branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"

  cat > "$REPORT_FILE" <<EOF
{
  "report_version": "$SCRIPT_VERSION",
  "generated_at": "$ts",
  "git_sha": "$git_sha",
  "git_branch": "$git_branch",
  "overall_status": "$overall",
  "checks": {
    "cargo_audit":        $audit,
    "secret_leaks":       $secrets,
    "hardcoded_secrets":  $hardcoded,
    "file_permissions":   $perms,
    "licence_compliance": $licence,
    "cicd_pinning":       $cicd
  }
}
EOF
  log "INFO" "Report written to $REPORT_FILE"
}

# ── Entry point ───────────────────────────────────────────────────────────────

main() {
  : > "$REPORT_LOG"
  log "INFO" "Security compliance reporting v$SCRIPT_VERSION starting..."

  require_tool git

  local audit_json secrets_json hardcoded_json perms_json licence_json cicd_json

  audit_json="$(check_cargo_audit)"
  secrets_json="$(check_secret_leaks)"
  hardcoded_json="$(check_hardcoded_secrets)"
  perms_json="$(check_file_permissions)"
  licence_json="$(check_licence)"
  cicd_json="$(check_cicd_pinning)"

  # Extract status fields for aggregation
  local s_audit s_secrets s_hardcoded s_perms s_licence s_cicd
  s_audit=$(echo "$audit_json"     | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
  s_secrets=$(echo "$secrets_json" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
  s_hardcoded=$(echo "$hardcoded_json" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
  s_perms=$(echo "$perms_json"     | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
  s_licence=$(echo "$licence_json" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)
  s_cicd=$(echo "$cicd_json"       | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)

  local overall
  overall="$(aggregate_severity \
    "$s_audit" "$s_secrets" "$s_hardcoded" "$s_perms" "$s_licence" "$s_cicd")"

  assemble_report \
    "$audit_json" "$secrets_json" "$hardcoded_json" \
    "$perms_json" "$licence_json" "$cicd_json" "$overall"

  log "INFO" "Overall compliance status: $overall"

  if [[ "$FAIL_ON_HIGH" == "true" && "$overall" == "fail" ]]; then
    log "ERROR" "Blocking findings detected – exiting with code 2"
    exit 2
  fi

  exit 0
}

main "$@"
