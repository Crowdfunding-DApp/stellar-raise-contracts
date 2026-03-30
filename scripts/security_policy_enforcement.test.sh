#!/usr/bin/env bash
# @title   security_policy_enforcement.test.sh
# @notice  Behavioral test suite for security_policy_enforcement.sh.
# @dev     Each test runs the script against an isolated fixture repository so
#          assertions cover the script's real output and exit behavior.
#          Exit code policy:
#            0 = all tests passed
#            1 = one or more tests failed
# @custom:security-note  The suite validates policy enforcement without
#          touching the network, GitHub APIs, or repository secrets.

set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly SCRIPT_UNDER_TEST="${REPO_ROOT}/scripts/security_policy_enforcement.sh"

TESTS_PASSED=0
TESTS_FAILED=0
TMPDIRS=()

cleanup() {
  if [[ "${#TMPDIRS[@]}" -gt 0 ]]; then
    rm -rf "${TMPDIRS[@]}"
  fi
}
trap cleanup EXIT

pass() {
  echo "PASS: $1"
  TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
  echo "FAIL: $1"
  TESTS_FAILED=$((TESTS_FAILED + 1))
}

make_tmpdir() {
  local dir
  dir="$(mktemp -d)"
  TMPDIRS+=("${dir}")
  echo "${dir}"
}

make_valid_fixture() {
  local root="$1"
  mkdir -p "${root}/.github/workflows"

  cat > "${root}/.github/workflows/security.yml" <<'EOF'
name: Security Compliance
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  security:
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - run: chmod +x scripts/security_policy_enforcement.sh scripts/security_policy_enforcement.test.sh
      - run: ./scripts/security_policy_enforcement.sh
      - run: ./scripts/security_policy_enforcement.test.sh
EOF

  cat > "${root}/.github/workflows/rust_ci.yml" <<'EOF'
name: Rust CI
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  frontend:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - run: npm test
  check:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - run: cargo test --workspace
EOF

  cat > "${root}/.github/workflows/spellcheck.yml" <<'EOF'
name: Spellcheck
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  spellcheck:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - run: cspell .
EOF

  cat > "${root}/.github/workflows/testnet_smoke.yml" <<'EOF'
name: Testnet Smoke Test
on:
  workflow_dispatch:
permissions:
  contents: read
jobs:
  smoke-test:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - name: Configure testnet identity from secret
        env:
          TESTNET_SECRET_KEY: ${{ secrets.TESTNET_SECRET_KEY }}
        run: stellar keys add ci-deployer --secret-key "$TESTNET_SECRET_KEY"
EOF

  cat > "${root}/.github/workflows/stale.yml" <<'EOF'
name: Stale
on:
  workflow_dispatch:
jobs:
  stale:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    permissions:
      issues: write
      pull-requests: write
    steps:
      - uses: actions/stale@v9
EOF
}

run_script() {
  local project_root="$1"
  PROJECT_ROOT="${project_root}" bash "${SCRIPT_UNDER_TEST}" 2>&1
}

assert_exit_and_pattern() {
  local description="$1"
  local expected_exit="$2"
  local pattern="$3"
  shift 3
  local output
  local actual=0

  output="$("$@" 2>&1)" || actual=$?

  if [[ "${actual}" -ne "${expected_exit}" ]]; then
    fail "${description} (expected exit ${expected_exit}, got ${actual})"
    return
  fi

  if [[ -n "${pattern}" ]] && ! grep -Eq "${pattern}" <<< "${output}"; then
    fail "${description} (pattern not found: ${pattern})"
    return
  fi

  pass "${description}"
}

test_valid_fixture_passes() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  assert_exit_and_pattern "valid fixture passes" 0 'SECURITY POLICY: PASSED' run_script "${root}"
}

test_report_file_is_generated() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  run_script "${root}" >/dev/null

  local report_count
  report_count=$(find "${root}/.security-policy-reports" -type f -name 'security_policy_report_*.txt' | wc -l)
  if [[ "${report_count}" -ge 1 ]]; then
    pass "report artefact is generated"
  else
    fail "report artefact is generated"
  fi
}

test_missing_permissions_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/permissions:\n  contents: read\n//' "${root}/.github/workflows/spellcheck.yml"
  assert_exit_and_pattern "missing permissions fails" 1 'spellcheck\.yml is missing an explicit permissions block' run_script "${root}"
}

test_write_all_permissions_fail() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/permissions:\n  contents: read\n/permissions: write-all\n/' "${root}/.github/workflows/rust_ci.yml"
  assert_exit_and_pattern "write-all permissions fail" 1 'rust_ci\.yml uses forbidden permissions: write-all' run_script "${root}"
}

test_missing_timeout_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/    timeout-minutes: 30\n//' "${root}/.github/workflows/testnet_smoke.yml"
  assert_exit_and_pattern "missing timeout fails" 1 'testnet_smoke\.yml is missing timeout-minutes' run_script "${root}"
}

test_missing_persist_credentials_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/        with:\n          persist-credentials: false\n//' "${root}/.github/workflows/security.yml"
  assert_exit_and_pattern "missing persist-credentials fails" 1 'security\.yml uses actions/checkout without persist-credentials: false' run_script "${root}"
}

test_direct_secret_interpolation_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/run: stellar keys add ci-deployer --secret-key "\$TESTNET_SECRET_KEY"/run: echo \${{ secrets.TESTNET_SECRET_KEY }}/' "${root}/.github/workflows/testnet_smoke.yml"
  assert_exit_and_pattern "direct secret interpolation fails" 1 'testnet_smoke\.yml interpolates secrets directly inside a run command' run_script "${root}"
}

test_mutable_action_ref_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/actions\/checkout\@v4/actions\/checkout\@main/' "${root}/.github/workflows/spellcheck.yml"
  assert_exit_and_pattern "mutable action ref fails" 1 'spellcheck\.yml uses a mutable action ref' run_script "${root}"
}

test_missing_security_hook_fails() {
  local root
  root="$(make_tmpdir)"
  make_valid_fixture "${root}"
  perl -0pi -e 's/      - run: \.\/scripts\/security_policy_enforcement\.test\.sh\n//' "${root}/.github/workflows/security.yml"
  assert_exit_and_pattern "missing security hook fails" 1 'security\.yml does not run security_policy_enforcement\.test\.sh' run_script "${root}"
}

test_real_repo_passes() {
  assert_exit_and_pattern "repository workflows satisfy policy" 0 'SECURITY POLICY: PASSED' run_script "${REPO_ROOT}"
}

main() {
  test_valid_fixture_passes
  test_report_file_is_generated
  test_missing_permissions_fails
  test_write_all_permissions_fail
  test_missing_timeout_fails
  test_missing_persist_credentials_fails
  test_direct_secret_interpolation_fails
  test_mutable_action_ref_fails
  test_missing_security_hook_fails
  test_real_repo_passes

  echo "Tests passed: ${TESTS_PASSED}"
  echo "Tests failed: ${TESTS_FAILED}"

  if [[ "${TESTS_FAILED}" -eq 0 ]]; then
    exit 0
  fi

  exit 1
}

main "$@"
