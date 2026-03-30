# Infrastructure Monitoring

## Purpose

`scripts/infrastructure_monitoring.sh` runs automated health checks against the Stellar Raise deployment. It is executed hourly by the `monitoring.yml` CI workflow and can be run locally at any time.

## What each check monitors

| Check | What it verifies | Failure means |
|---|---|---|
| `check_rpc_endpoint` | Stellar RPC/Horizon endpoint responds HTTP 200 within 5s | Node is down or unreachable; contract calls will fail |
| `check_contract_deployed` | Contract ID exists on the network via `stellar contract info` | Contract not deployed, wrong network, or bad CONTRACT_ID |
| `check_disk_usage` | Disk usage < 85% | CI runner or server is low on disk; builds may fail |
| `check_memory_usage` | Memory usage < 90% | Runner is memory-constrained; Rust compilation may OOM |
| `check_ci_artifacts` | `target/wasm32-unknown-unknown/release/crowdfund.wasm` exists and is non-empty | Build step did not produce expected output |

## Required environment variables

| Variable | Description | Where to set |
|---|---|---|
| `STELLAR_RPC_URL` | Stellar RPC/Horizon endpoint URL | GitHub Actions secret → `STELLAR_RPC_URL` |
| `CONTRACT_ID` | Deployed crowdfund contract address (C...) | GitHub Actions secret → `CONTRACT_ID` |
| `STELLAR_NETWORK_PASSPHRASE` | Network passphrase (defaults to testnet) | GitHub Actions secret → `STELLAR_NETWORK_PASSPHRASE` |

## How to run locally

```bash
export STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
export CONTRACT_ID="CYOURCONTRACTADDRESS"
bash scripts/infrastructure_monitoring.sh
```

## How to run the tests

```bash
bash scripts/infrastructure_monitoring.test.sh
```

No external test framework is required. The test script uses plain bash assertions and mocks all external calls (no real network requests).

## How to add a new check

1. Add a new function to `scripts/infrastructure_monitoring.sh`:

```bash
# ── check_my_thing ────────────────────────────────────────────────────────────
# What it checks: <description>
# Failure means:  <impact>
check_my_thing() {
  if <condition>; then
    pass "check_my_thing: ok"
    return 0
  else
    fail "check_my_thing: <reason>"
    return 1
  fi
}
```

2. Add it to `run_all_checks`:

```bash
check_my_thing || failed=$(( failed + 1 ))
```

3. Add tests to `scripts/infrastructure_monitoring.test.sh` covering pass and fail cases, mocking any external commands.

## Security notes

- No secrets or credentials are hardcoded in any script.
- All sensitive values are passed via environment variables.
- In CI, environment variables are loaded from GitHub Actions secrets — they are never printed in logs.
- The `stellar` CLI is called with `--rpc-url` from `$STELLAR_RPC_URL`; no config files with embedded keys are used.
- Scripts use `set -uo pipefail` to fail fast on unset variables and pipeline errors.
