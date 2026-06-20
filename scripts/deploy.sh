#!/bin/bash
set -euo pipefail

# Usage: ./scripts/deploy.sh [--dry-run] [--help] <creator> <token> <goal> <deadline> [min_contribution]
#
# Environment variables:
#   NETWORK                    Stellar network to target (default: testnet)
#   STELLAR_RPC_URL            Optional custom RPC endpoint
#   STELLAR_NETWORK_PASSPHRASE Optional network passphrase override
#   SOURCE_ACCOUNT             Optional override for the creator/source account

print_help() {
  cat <<EOF
Usage: $0 [--dry-run] [--help] <creator> <token> <goal> <deadline> [min_contribution]

Arguments:
  creator          Stellar address or identity of the campaign creator
  token            Stellar address of the token contract
  goal             Funding goal in stroops (integer)
  deadline         Unix timestamp for campaign end (must be in the future)
  min_contribution Minimum pledge in stroops (default: 1)

Flags:
  --dry-run  Print the resolved parameters and deploy command without
             submitting any transaction; exits 0.
  --help     Print this usage message and exit.

Environment variables:
  NETWORK                    Stellar network to target (default: testnet)
  STELLAR_RPC_URL            Optional custom RPC endpoint
  STELLAR_NETWORK_PASSPHRASE Optional network passphrase override
  SOURCE_ACCOUNT             Optional override for the creator/source account

Examples:
  ./scripts/deploy.sh GCREATOR... GTOKEN... 1000 1735689600 10
  ./scripts/deploy.sh --dry-run GCREATOR... GTOKEN... 1000 1735689600 10
  NETWORK=mainnet ./scripts/deploy.sh GCREATOR... GTOKEN... 1000 1735689600 10
EOF
}

DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --help|-h) print_help; exit 0 ;;
    *) break ;;
  esac
done

CREATOR=${1:?$(print_help; echo ""; echo "ERROR: missing required argument: creator")}
TOKEN=${2:?ERROR: missing required argument: token}
GOAL=${3:?ERROR: missing required argument: goal}
DEADLINE=${4:?ERROR: missing required argument: deadline}
MIN_CONTRIBUTION=${5:-1}
NETWORK="${NETWORK:-testnet}"

WASM_GLOB="target/wasm32-unknown-unknown/release/*.wasm"

# Resolve the WASM glob — fail early before any network call.
WASM_FILES=()
while IFS= read -r -d '' f; do
  WASM_FILES+=("$f")
done < <(find target/wasm32-unknown-unknown/release -maxdepth 1 -name "*.wasm" -print0 2>/dev/null || true)

if [[ ${#WASM_FILES[@]} -eq 0 ]]; then
  echo "ERROR: No WASM artifact found at $WASM_GLOB" >&2
  echo "Hint:  Run 'pnpm build:contracts' or" >&2
  echo "       'cargo build --target wasm32-unknown-unknown --release -p crowdfund'" >&2
  exit 1
fi
CONTRACT_WASM="${WASM_FILES[0]}"

# ── Dry-run mode ─────────────────────────────────────────────────────────────
if [[ "$DRY_RUN" == "true" ]]; then
  echo "[DRY-RUN] Resolved parameters:"
  echo "  CREATOR          = $CREATOR"
  echo "  TOKEN            = $TOKEN"
  echo "  GOAL             = $GOAL"
  echo "  DEADLINE         = $DEADLINE"
  echo "  MIN_CONTRIBUTION = $MIN_CONTRIBUTION"
  echo "  NETWORK          = $NETWORK"
  echo "  CONTRACT_WASM    = $CONTRACT_WASM"
  echo ""
  echo "[DRY-RUN] Would execute:"
  echo "  stellar contract deploy \\"
  echo "    --wasm \"$CONTRACT_WASM\" \\"
  echo "    --network \"$NETWORK\" \\"
  echo "    --source \"$CREATOR\""
  exit 0
fi

# ── Deploy ────────────────────────────────────────────────────────────────────
DEPLOY_CMD="stellar contract deploy --wasm \"$CONTRACT_WASM\" --network \"$NETWORK\" --source \"$CREATOR\""
echo "[LOG] step=deploy status=start network=$NETWORK wasm=$CONTRACT_WASM"
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$CONTRACT_WASM" \
  --network "$NETWORK" \
  --source "$CREATOR") || {
  echo "ERROR: 'stellar contract deploy' failed (exit $?)." >&2
  echo "Command: $DEPLOY_CMD" >&2
  exit 4
}
echo "[LOG] step=deploy status=ok contract_id=$CONTRACT_ID"

# ── Initialize ────────────────────────────────────────────────────────────────
INIT_CMD="stellar contract invoke --id \"$CONTRACT_ID\" --network \"$NETWORK\" --source \"$CREATOR\" -- initialize ..."
echo "[LOG] step=initialize status=start contract_id=$CONTRACT_ID"
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  --source "$CREATOR" \
  -- \
  initialize \
  --admin "$CREATOR" \
  --creator "$CREATOR" \
  --token "$TOKEN" \
  --goal "$GOAL" \
  --deadline "$DEADLINE" \
  --min_contribution "$MIN_CONTRIBUTION" \
  --platform_config "null" \
  --bonus_goal "null" \
  --bonus_goal_description "null" || {
  echo "ERROR: 'stellar contract invoke initialize' failed (exit $?)." >&2
  echo "Command: $INIT_CMD" >&2
  exit 5
}
echo "[LOG] step=initialize status=ok"

# ── Post-deploy smoke check ───────────────────────────────────────────────────
SMOKE_CMD="stellar contract invoke --id \"$CONTRACT_ID\" --network \"$NETWORK\" -- goal"
echo "[LOG] step=smoke_check status=start contract_id=$CONTRACT_ID"
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  -- \
  goal || {
  echo "ERROR: post-deploy smoke check failed (exit $?). Contract may not be live." >&2
  echo "Command: $SMOKE_CMD" >&2
  exit 6
}
echo "[LOG] step=smoke_check status=ok"

echo "[LOG] step=done contract_id=$CONTRACT_ID"
