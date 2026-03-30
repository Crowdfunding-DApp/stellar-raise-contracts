#!/usr/bin/env bash
# @title   security_policy_enforcement.sh
# @notice  Enforces repository CI/CD security policy requirements for GitHub
#          Actions workflows and reports any compliance drift.
# @dev     This script is intended for local use and CI execution. It scans the
#          checked-in workflow files only; it never evaluates workflow YAML.
#          Exit code policy:
#            0 = all policy checks passed
#            1 = one or more policy checks failed
# @custom:security-note  Least-privilege permissions, bounded execution time,
#          checkout hardening, and secret hygiene reduce the blast radius of a
#          compromised workflow runner or malicious pull request.

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="${PROJECT_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"
readonly WORKFLOWS_DIR="${WORKFLOWS_DIR:-${PROJECT_ROOT}/.github/workflows}"
readonly SECURITY_WORKFLOW="${SECURITY_WORKFLOW:-${WORKFLOWS_DIR}/security.yml}"
readonly REPORT_DIR="${REPORT_DIR:-${PROJECT_ROOT}/.security-policy-reports}"
readonly REPORT_FILE="${REPORT_DIR}/security_policy_report_$(date -u +%Y%m%d_%H%M%S).txt"

FAILURES=0
REPORT_LINES=()

pass() {
  local message="$1"
  echo "PASS: ${message}"
  REPORT_LINES+=("PASS|${message}")
}

fail() {
  local message="$1"
  echo "FAIL: ${message}" >&2
  FAILURES=$((FAILURES + 1))
  REPORT_LINES+=("FAIL|${message}")
}

warn() {
  local message="$1"
  echo "WARN: ${message}"
  REPORT_LINES+=("WARN|${message}")
}

# @notice  Writes the accumulated policy results to an artefact for CI review.
write_report() {
  mkdir -p "${REPORT_DIR}"
  {
    echo "security_policy_report"
    echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "project_root=${PROJECT_ROOT}"
    echo "workflow_dir=${WORKFLOWS_DIR}"
    echo "failures=${FAILURES}"
    printf '%s\n' "${REPORT_LINES[@]}"
  } > "${REPORT_FILE}"
  echo "Report: ${REPORT_FILE}"
}

# @notice  Returns all workflow YAML files under WORKFLOWS_DIR in sorted order.
list_workflows() {
  if [[ ! -d "${WORKFLOWS_DIR}" ]]; then
    fail "workflow directory not found: ${WORKFLOWS_DIR}"
    return 1
  fi

  find "${WORKFLOWS_DIR}" -maxdepth 1 -type f \( -name "*.yml" -o -name "*.yaml" \) | sort
}

# @notice  Ensures every workflow declares explicit permissions and avoids
#          GitHub's broad default token scope.
check_permissions() {
  local workflow
  while IFS= read -r workflow; do
    if grep -Eq '^[[:space:]]*permissions:' "${workflow}"; then
      pass "$(basename "${workflow}") declares explicit permissions"
    else
      fail "$(basename "${workflow}") is missing an explicit permissions block"
    fi

    if grep -Eq '^[[:space:]]*permissions:[[:space:]]*write-all[[:space:]]*$' "${workflow}"; then
      fail "$(basename "${workflow}") uses forbidden permissions: write-all"
    fi
  done < <(list_workflows)
}

# @notice  Requires every workflow to define at least one timeout bound.
check_timeouts() {
  local workflow
  while IFS= read -r workflow; do
    if grep -Eq '^[[:space:]]*timeout-minutes:' "${workflow}"; then
      pass "$(basename "${workflow}") defines timeout-minutes"
    else
      fail "$(basename "${workflow}") is missing timeout-minutes"
    fi
  done < <(list_workflows)
}

# @notice  Ensures checkout steps disable credential persistence.
check_checkout_hardening() {
  local workflow
  while IFS= read -r workflow; do
    if ! grep -Eq 'uses:[[:space:]]*actions/checkout@' "${workflow}"; then
      warn "$(basename "${workflow}") has no checkout step to harden"
      continue
    fi

    if grep -Eq 'persist-credentials:[[:space:]]*false' "${workflow}"; then
      pass "$(basename "${workflow}") hardens actions/checkout with persist-credentials: false"
    else
      fail "$(basename "${workflow}") uses actions/checkout without persist-credentials: false"
    fi
  done < <(list_workflows)
}

# @notice  Prevents direct interpolation of GitHub secrets into shell commands.
check_secret_handling() {
  local workflow
  while IFS= read -r workflow; do
    if grep -Eq 'run:[[:space:]].*\${{[[:space:]]*secrets\.' "${workflow}"; then
      fail "$(basename "${workflow}") interpolates secrets directly inside a run command"
    else
      pass "$(basename "${workflow}") keeps secrets out of direct run interpolation"
    fi
  done < <(list_workflows)
}

# @notice  Rejects mutable action refs like @main or @master.
check_action_pinning() {
  local workflow
  while IFS= read -r workflow; do
    if grep -Eq 'uses:[[:space:]]*[^[:space:]]+@(main|master|HEAD)$' "${workflow}"; then
      fail "$(basename "${workflow}") uses a mutable action ref"
    else
      pass "$(basename "${workflow}") pins actions to non-mutable refs"
    fi
  done < <(list_workflows)
}

# @notice  Confirms the dedicated security workflow runs this policy script and
#          its test suite so policy regressions block merges.
check_security_workflow_hooks() {
  if [[ ! -f "${SECURITY_WORKFLOW}" ]]; then
    fail "security workflow not found: ${SECURITY_WORKFLOW}"
    return
  fi

  if grep -Eq 'run:[[:space:]]+\./scripts/security_policy_enforcement\.sh([[:space:]]|$)' "${SECURITY_WORKFLOW}"; then
    pass "security.yml runs security_policy_enforcement.sh"
  else
    fail "security.yml does not run security_policy_enforcement.sh"
  fi

  if grep -Eq 'run:[[:space:]]+\./scripts/security_policy_enforcement\.test\.sh([[:space:]]|$)' "${SECURITY_WORKFLOW}"; then
    pass "security.yml runs security_policy_enforcement.test.sh"
  else
    fail "security.yml does not run security_policy_enforcement.test.sh"
  fi
}

main() {
  echo "Stellar Raise - Security Policy Enforcement"
  echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  check_permissions
  check_timeouts
  check_checkout_hardening
  check_secret_handling
  check_action_pinning
  check_security_workflow_hooks
  write_report

  if [[ "${FAILURES}" -eq 0 ]]; then
    echo "SECURITY POLICY: PASSED"
    exit 0
  fi

  echo "SECURITY POLICY: FAILED (${FAILURES} issue(s))" >&2
  exit 1
}

main "$@"
