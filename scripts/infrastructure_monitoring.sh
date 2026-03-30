#!/usr/bin/env bash
# scripts/infrastructure_monitoring.sh
#
# Automated infrastructure monitoring for Stellar Raise contracts.
# Runs a suite of health checks and prints a pass/fail summary.
#
# Usage:
#   bash scripts/infrastructure_monitoring.sh
#
# Required environment variables:
#   STELLAR_RPC_URL  — Stellar RPC/Horizon endpoint (e.g. https://soroban-testnet.stellar.org)
#   CONTRACT_ID      — Deployed crowdfund contract address (C...)
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
#
# Security: no secrets are hardcoded. All credentials come from environment
# variables. Never add literal keys or URLs to this file.

set -uo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────

PASS_LABEL="[PASS]"
FAIL_LABEL="[FAIL]"

pass() { printf '%s %s\n' "$PASS_LABEL" "$1"; }
fail() { printf '%s %s\n' "$FAIL_LABEL" "$1"; }

# ── Thresholds ────────────────────────────────────────────────────────────────

DISK_THRESHOLD=85   # percent
MEM_THRESHOLD=90    # percent
RPC_TIMEOUT=5       # seconds

# ── check_rpc_endpoint ────────────────────────────────────────────────────────
# What it checks: the Stellar RPC/Horizon endpoint responds with HTTP 200
#                 within $RPC_TIMEOUT seconds.
# Failure means:  the RPC node is down, unreachable, or misconfigured.
#                 Contract calls and deployments will fail.
check_rpc_endpoint() {
  local url="${STELLAR_RPC_URL:-}"
  if [[ -z "$url" ]]; then
    fail "check_rpc_endpoint: STELLAR_RPC_URL is not set"
    return 1
  fi

  local http_code
  http_code=$(curl -s -o /dev/null -w "%{http_code}" \
    --max-time "$RPC_TIMEOUT" "$url" 2>/dev/null) || true

  if [[ "$http_code" == "200" ]]; then
    pass "check_rpc_endpoint: $url responded HTTP 200"
    return 0
  else
    fail "check_rpc_endpoint: $url returned HTTP ${http_code:-unreachable} (expected 200)"
    return 1
  fi
}

# ── check_contract_deployed ───────────────────────────────────────────────────
# What it checks: the contract ID exists on the network by querying its info
#                 via the Stellar CLI.
# Failure means:  the contract is not deployed, the wrong network is configured,
#                 or the CONTRACT_ID env var is incorrect.
check_contract_deployed() {
  local contract_id="${CONTRACT_ID:-}"
  if [[ -z "$contract_id" ]]; then
    fail "check_contract_deployed: CONTRACT_ID is not set"
    return 1
  fi

  local rpc_url="${STELLAR_RPC_URL:-}"
  if [[ -z "$rpc_url" ]]; then
    fail "check_contract_deployed: STELLAR_RPC_URL is not set"
    return 1
  fi

  if stellar contract info \
       --id "$contract_id" \
       --rpc-url "$rpc_url" \
       --network-passphrase "${STELLAR_NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}" \
       > /dev/null 2>&1; then
    pass "check_contract_deployed: contract $contract_id found on network"
    return 0
  else
    fail "check_contract_deployed: contract $contract_id not found (check CONTRACT_ID and STELLAR_RPC_URL)"
    return 1
  fi
}

# ── check_disk_usage ──────────────────────────────────────────────────────────
# What it checks: disk usage of the filesystem containing the current directory
#                 does not exceed $DISK_THRESHOLD percent.
# Failure means:  the CI runner or server is running low on disk. Build
#                 artifacts, logs, or WASM binaries may fail to write.
check_disk_usage() {
  local usage
  usage=$(df -P . | awk 'NR==2 {gsub(/%/,"",$5); print $5}')

  if [[ -z "$usage" ]]; then
    fail "check_disk_usage: could not determine disk usage"
    return 1
  fi

  if (( usage < DISK_THRESHOLD )); then
    pass "check_disk_usage: ${usage}% used (threshold ${DISK_THRESHOLD}%)"
    return 0
  else
    fail "check_disk_usage: ${usage}% used — exceeds threshold of ${DISK_THRESHOLD}%"
    return 1
  fi
}

# ── check_memory_usage ────────────────────────────────────────────────────────
# What it checks: system memory usage does not exceed $MEM_THRESHOLD percent.
# Failure means:  the runner is memory-constrained. Rust compilation and
#                 Soroban test execution may OOM-kill.
check_memory_usage() {
  local total used usage

  if command -v free > /dev/null 2>&1; then
    # Linux
    read -r total used <<< "$(free -m | awk 'NR==2 {print $2, $3}')"
  else
    # macOS fallback
    total=$(( $(sysctl -n hw.memsize) / 1024 / 1024 ))
    used=$(( total - $(vm_stat | awk '/Pages free/ {print $3}' | tr -d '.') * 4 / 1024 ))
  fi

  if [[ -z "${total:-}" || "$total" -eq 0 ]]; then
    fail "check_memory_usage: could not determine memory usage"
    return 1
  fi

  usage=$(( used * 100 / total ))

  if (( usage < MEM_THRESHOLD )); then
    pass "check_memory_usage: ${usage}% used (threshold ${MEM_THRESHOLD}%)"
    return 0
  else
    fail "check_memory_usage: ${usage}% used — exceeds threshold of ${MEM_THRESHOLD}%"
    return 1
  fi
}

# ── check_ci_artifacts ────────────────────────────────────────────────────────
# What it checks: the compiled crowdfund WASM artifact exists and is non-empty
#                 after a CI build run.
# Failure means:  the build step did not produce the expected output. Deployment
#                 or size-check steps will fail downstream.
check_ci_artifacts() {
  local wasm_path="target/wasm32-unknown-unknown/release/crowdfund.wasm"

  if [[ -f "$wasm_path" && -s "$wasm_path" ]]; then
    local size
    size=$(stat -c%s "$wasm_path" 2>/dev/null || stat -f%z "$wasm_path" 2>/dev/null)
    pass "check_ci_artifacts: $wasm_path exists (${size} bytes)"
    return 0
  else
    fail "check_ci_artifacts: $wasm_path not found or empty — run: cargo build --release --target wasm32-unknown-unknown -p crowdfund"
    return 1
  fi
}

# ── run_all_checks ────────────────────────────────────────────────────────────
# Runs every check, collects results, prints a summary, and exits non-zero
# if any check failed.
run_all_checks() {
  echo "=== Stellar Raise — Infrastructure Monitoring ==="
  echo "Timestamp: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo ""

  local failed=0

  check_rpc_endpoint       || failed=$(( failed + 1 ))
  check_contract_deployed  || failed=$(( failed + 1 ))
  check_disk_usage         || failed=$(( failed + 1 ))
  check_memory_usage       || failed=$(( failed + 1 ))
  check_ci_artifacts       || failed=$(( failed + 1 ))

  echo ""
  echo "=== Summary ==="
  if (( failed == 0 )); then
    echo "All checks passed."
    return 0
  else
    echo "${failed} check(s) failed."
    return 1
  fi
}

# ── Entry point ───────────────────────────────────────────────────────────────
# Only run when executed directly (not when sourced by tests).
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  run_all_checks
fi
