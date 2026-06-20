#!/bin/bash
set -euo pipefail

# Usage: ./scripts/interact.sh [--help] <contract_id> <action> [args...]
#
# Actions:
#   contribute <contributor> <amount>  — Contribute tokens to the campaign
#   withdraw   <creator>               — Withdraw funds (goal met, deadline passed)
#   refund     <caller>                — Refund a single contributor
#
# Environment variables:
#   NETWORK   Stellar network to target (default: testnet)

print_help() {
  cat <<EOF
Usage: $0 [--help] <contract_id> <action> [args...]

Arguments:
  contract_id   Deployed contract address (C...)

Actions:
  contribute <contributor> <amount>
      contributor  Stellar address of the backer
      amount       Amount in stroops (integer)

  withdraw <creator>
      creator      Stellar address of the campaign creator

  refund <caller>
      caller       Stellar address of the contributor requesting refund

Flags:
  --help   Print this usage message and exit.

Environment variables:
  NETWORK  Stellar network to target (default: testnet)

Examples:
  ./scripts/interact.sh C... contribute GBACKER... 100
  ./scripts/interact.sh C... withdraw GCREATOR...
  ./scripts/interact.sh C... refund GBACKER...
EOF
}

if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
  print_help
  exit 0
fi

CONTRACT_ID=${1:?$(print_help; echo ""; echo "ERROR: missing required argument: contract_id")}
ACTION=${2:?ERROR: missing required argument: action — one of: contribute, withdraw, refund}
NETWORK="${NETWORK:-testnet}"

case "$ACTION" in
contribute)
  CONTRIBUTOR=${3:?ERROR: missing argument: contributor}
  AMOUNT=${4:?ERROR: missing argument: amount}
  INVOKE_CMD="stellar contract invoke --id \"$CONTRACT_ID\" --network \"$NETWORK\" --source \"$CONTRIBUTOR\" -- contribute --contributor \"$CONTRIBUTOR\" --amount \"$AMOUNT\""
  echo "[LOG] action=contribute status=start contributor=$CONTRIBUTOR amount=$AMOUNT"
  stellar contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$CONTRIBUTOR" \
    -- \
    contribute \
    --contributor "$CONTRIBUTOR" \
    --amount "$AMOUNT" || {
    echo "ERROR: contribute failed (exit $?)." >&2
    echo "Command: $INVOKE_CMD" >&2
    exit 1
  }
  echo "[LOG] action=contribute status=ok contributor=$CONTRIBUTOR amount=$AMOUNT"
  ;;

withdraw)
  CREATOR=${3:?ERROR: missing argument: creator}
  INVOKE_CMD="stellar contract invoke --id \"$CONTRACT_ID\" --network \"$NETWORK\" --source \"$CREATOR\" -- withdraw"
  echo "[LOG] action=withdraw status=start creator=$CREATOR"
  stellar contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$CREATOR" \
    -- \
    withdraw || {
    echo "ERROR: withdraw failed (exit $?)." >&2
    echo "Command: $INVOKE_CMD" >&2
    exit 1
  }
  echo "[LOG] action=withdraw status=ok creator=$CREATOR"
  ;;

refund)
  CALLER=${3:?ERROR: missing argument: caller}
  INVOKE_CMD="stellar contract invoke --id \"$CONTRACT_ID\" --network \"$NETWORK\" --source \"$CALLER\" -- refund_single --contributor \"$CALLER\""
  echo "[LOG] action=refund status=start caller=$CALLER"
  stellar contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$CALLER" \
    -- \
    refund_single \
    --contributor "$CALLER" || {
    echo "ERROR: refund failed (exit $?)." >&2
    echo "Command: $INVOKE_CMD" >&2
    exit 1
  }
  echo "[LOG] action=refund status=ok caller=$CALLER"
  ;;

*)
  echo "ERROR: unknown action '$ACTION'. Valid actions: contribute, withdraw, refund" >&2
  echo "Run '$0 --help' for usage." >&2
  exit 1
  ;;
esac
