#!/bin/bash
set -e

# Interaction script for Stellar Raise Crowdfund Contract
# Provides common post-deploy actions: contribute, withdraw, refund

usage() {
    echo "Usage: $0 <CONTRACT_ID> <ACTION> [ARGS...]"
    echo ""
    echo "Arguments:"
    echo "  CONTRACT_ID - The deployed contract ID"
    echo "  ACTION      - One of: contribute, withdraw, refund"
    echo ""
    echo "Actions:"
    echo "  contribute <CONTRIBUTOR> <AMOUNT>  - Contribute tokens to the campaign"
    echo "  withdraw <CREATOR>                  - Creator withdraws funds (after deadline, goal met)"
    echo "  refund <CALLER>                     - Refund all contributors (after deadline, goal not met)"
    echo ""
    echo "Examples:"
    echo "  $0 CDLYMY5EUKZKIJEFYJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q contribute GDIY63C4EHX2PF6UXJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q 1000000"
    echo "  $0 CDLYMY5EUKZKIJEFYJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q withdraw GDIY63C4EHX2PF6UXJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q"
    echo "  $0 CDLYMY5EUKZKIJEFYJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q refund GDIY63C4EHX2PF6UXJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q"
    exit 1
}

CONTRACT_ID=${1:?$(usage)}
ACTION=${2:?$(usage)}
NETWORK="testnet"

case "$ACTION" in
contribute)
    CONTRIBUTOR=${3:?$(usage)}
    AMOUNT=${4:?$(usage)}
    echo "Contributing $AMOUNT from $CONTRIBUTOR to contract $CONTRACT_ID..."
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$CONTRIBUTOR" \
        -- \
        contribute \
        --contributor "$CONTRIBUTOR" \
        --amount "$AMOUNT"
    echo "Contribution successful."
    ;;

withdraw)
    CREATOR=${3:?$(usage)}
    echo "Withdrawing funds from contract $CONTRACT_ID as creator $CREATOR..."
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$CREATOR" \
        -- \
        withdraw
    echo "Withdrawal successful."
    ;;

refund)
    CALLER=${3:?$(usage)}
    echo "Requesting refund from contract $CONTRACT_ID as $CALLER..."
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$CALLER" \
        -- \
        refund
    echo "Refund successful."
    ;;

*)
    echo "Unknown action: $ACTION"
    echo "Use: contribute | withdraw | refund"
    exit 1
    ;;
esac
