#!/usr/bin/env bash
# scripts/infrastructure_monitoring.test.sh
#
# Plain-bash test suite for infrastructure_monitoring.sh.
# No external test framework required — uses assertion helpers that match
# the style of scripts/verify_env.sh.
#
# Usage:
#   bash scripts/infrastructure_monitoring.test.sh
#
# Exit codes:
#   0 — all tests passed
#   1 — one or more tests failed

set -uo pipefail

# ── Load the script under test (source, not execute) ─────────────────────────
# shellcheck source=infrastructure_monitoring.sh
source "$(dirname "$0")/infrastructure_monitoring.sh"

# ── Test harness ──────────────────────────────────────────────────────────────

TESTS_RUN=0
TESTS_FAILED=0

assert_pass() {
  local description="$1"
  local exit_code="$2"
  TESTS_RUN=$(( TESTS_RUN + 1 ))
  if (( exit_code == 0 )); then
    printf '[PASS] %s\n' "$description"
  else
    printf '[FAIL] %s (expected exit 0, got %d)\n' "$description" "$exit_code"
    TESTS_FAILED=$(( TESTS_FAILED + 1 ))
  fi
}

assert_fail() {
  local description="$1"
  local exit_code="$2"
  TESTS_RUN=$(( TESTS_RUN + 1 ))
  if (( exit_code != 0 )); then
    printf '[PASS] %s\n' "$description"
  else
    printf '[FAIL] %s (expected non-zero exit, got 0)\n' "$description"
    TESTS_FAILED=$(( TESTS_FAILED + 1 ))
  fi
}

# ── Mock helpers ──────────────────────────────────────────────────────────────
# Override external commands with controlled stubs. Each test restores them.

mock_curl_200()  { curl() { echo "200"; }; export -f curl; }
mock_curl_fail() { curl() { echo "000"; }; export -f curl; }
mock_stellar_ok()   { stellar() { return 0; }; export -f stellar; }
mock_stellar_fail() { stellar() { return 1; }; export -f stellar; }
restore_curl()    { unset -f curl; }
restore_stellar() { unset -f stellar; }

# ── Tests: check_rpc_endpoint ─────────────────────────────────────────────────

echo ""
echo "--- check_rpc_endpoint ---"

# Passes when endpoint returns HTTP 200
(
  mock_curl_200
  # Override curl to return 200 via the -w flag path
  curl() {
    # Simulate: curl -s -o /dev/null -w "%{http_code}" ...
    echo "200"
  }
  export -f curl
  export STELLAR_RPC_URL="https://example.com"
  check_rpc_endpoint
)
assert_pass "check_rpc_endpoint passes when endpoint returns HTTP 200" $?

# Fails when endpoint is unreachable (curl returns empty/non-200)
(
  curl() { echo "000"; }
  export -f curl
  export STELLAR_RPC_URL="https://unreachable.invalid"
  check_rpc_endpoint
)
assert_fail "check_rpc_endpoint fails when endpoint is unreachable" $?

# Fails when STELLAR_RPC_URL is unset
(
  unset STELLAR_RPC_URL
  check_rpc_endpoint
)
assert_fail "check_rpc_endpoint fails when STELLAR_RPC_URL is unset" $?

# ── Tests: check_contract_deployed ───────────────────────────────────────────

echo ""
echo "--- check_contract_deployed ---"

# Fails gracefully when CONTRACT_ID is unset
(
  unset CONTRACT_ID
  export STELLAR_RPC_URL="https://example.com"
  check_contract_deployed
)
assert_fail "check_contract_deployed fails gracefully when CONTRACT_ID is unset" $?

# Fails gracefully when STELLAR_RPC_URL is unset
(
  export CONTRACT_ID="CTEST"
  unset STELLAR_RPC_URL
  check_contract_deployed
)
assert_fail "check_contract_deployed fails gracefully when STELLAR_RPC_URL is unset" $?

# Passes when stellar CLI succeeds
(
  mock_stellar_ok
  export CONTRACT_ID="CTEST123"
  export STELLAR_RPC_URL="https://example.com"
  check_contract_deployed
)
assert_pass "check_contract_deployed passes when stellar CLI succeeds" $?

# Fails when stellar CLI returns non-zero
(
  mock_stellar_fail
  export CONTRACT_ID="CBADCONTRACT"
  export STELLAR_RPC_URL="https://example.com"
  check_contract_deployed
)
assert_fail "check_contract_deployed fails when contract not found" $?

# ── Tests: check_disk_usage ───────────────────────────────────────────────────

echo ""
echo "--- check_disk_usage ---"

# Passes when usage is below threshold
(
  df() { printf 'Filesystem 1024-blocks Used Available Use%% Mounted\n/dev/sda1 100 50 50 50%% /\n'; }
  export -f df
  DISK_THRESHOLD=85
  check_disk_usage
)
assert_pass "check_disk_usage passes when usage (50%) is below threshold (85%)" $?

# Fails when usage is above threshold
(
  df() { printf 'Filesystem 1024-blocks Used Available Use%% Mounted\n/dev/sda1 100 90 10 90%% /\n'; }
  export -f df
  DISK_THRESHOLD=85
  check_disk_usage
)
assert_fail "check_disk_usage fails when usage (90%) exceeds threshold (85%)" $?

# ── Tests: check_memory_usage ─────────────────────────────────────────────────

echo ""
echo "--- check_memory_usage ---"

# Passes when memory usage is below threshold
(
  free() { printf '              total        used        free\nMem:           1000         500         500\n'; }
  export -f free
  MEM_THRESHOLD=90
  check_memory_usage
)
assert_pass "check_memory_usage passes when usage (50%) is below threshold (90%)" $?

# Fails when memory usage is above threshold
(
  free() { printf '              total        used        free\nMem:           1000         950          50\n'; }
  export -f free
  MEM_THRESHOLD=90
  check_memory_usage
)
assert_fail "check_memory_usage fails when usage (95%) exceeds threshold (90%)" $?

# ── Tests: run_all_checks ─────────────────────────────────────────────────────

echo ""
echo "--- run_all_checks ---"

# Exits non-zero when any single check fails
(
  # Make check_rpc_endpoint fail, everything else pass
  check_rpc_endpoint()      { fail "check_rpc_endpoint: forced failure"; return 1; }
  check_contract_deployed() { pass "check_contract_deployed: ok"; return 0; }
  check_disk_usage()        { pass "check_disk_usage: ok"; return 0; }
  check_memory_usage()      { pass "check_memory_usage: ok"; return 0; }
  check_ci_artifacts()      { pass "check_ci_artifacts: ok"; return 0; }
  export -f check_rpc_endpoint check_contract_deployed check_disk_usage check_memory_usage check_ci_artifacts
  run_all_checks
)
assert_fail "run_all_checks exits non-zero when any single check fails" $?

# Exits zero when all checks pass
(
  check_rpc_endpoint()      { pass "check_rpc_endpoint: ok"; return 0; }
  check_contract_deployed() { pass "check_contract_deployed: ok"; return 0; }
  check_disk_usage()        { pass "check_disk_usage: ok"; return 0; }
  check_memory_usage()      { pass "check_memory_usage: ok"; return 0; }
  check_ci_artifacts()      { pass "check_ci_artifacts: ok"; return 0; }
  export -f check_rpc_endpoint check_contract_deployed check_disk_usage check_memory_usage check_ci_artifacts
  run_all_checks
)
assert_pass "run_all_checks exits zero when all checks pass" $?

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "=== Test Summary ==="
echo "Tests run:    $TESTS_RUN"
echo "Tests failed: $TESTS_FAILED"

if (( TESTS_FAILED == 0 )); then
  echo "All tests passed."
  exit 0
else
  echo "${TESTS_FAILED} test(s) failed."
  exit 1
fi
