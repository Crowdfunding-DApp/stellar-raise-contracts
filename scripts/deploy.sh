#!/bin/bash
set -e

# Deployment script for Stellar Raise Crowdfund Contract
# Builds WASM, deploys to testnet, and initializes a campaign

usage() {
    echo "Usage: $0 <CREATOR> <TOKEN> <GOAL> <DEADLINE> <MIN_CONTRIBUTION>"
    echo ""
    echo "Arguments:"
    echo "  CREATOR          - Public key of the campaign creator"
    echo "  TOKEN            - Token contract address (e.g., USDC)"
    echo "  GOAL             - Funding goal in token's smallest unit"
    echo "  DEADLINE         - Campaign deadline as a UNIX timestamp"
    echo "  MIN_CONTRIBUTION - Minimum contribution amount"
    echo ""
    echo "Example:"
    echo "  $0 GDIY63C4EHX2PF6UXJ5K7A7K7Y3LZ5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q5Q 5000000000 1740000000 1000000"
    exit 1
}

# Validate required arguments
CREATOR=${1:?$(usage)}
TOKEN=${2:?$(usage)}
GOAL=${3:?$(usage)}
DEADLINE=${4:?$(usage)}
MIN_CONTRIBUTION=${5:?$(usage)}

NETWORK="testnet"
WASM_PATH="target/wasm32-unknown-unknown/release/crowdfund.wasm"

echo "=== Stellar Raise Deployment Script ==="
echo ""

# Step 1: Build WASM
echo "Building WASM..."
cargo build --target wasm32-unknown-unknown --release --manifest-path stellar-raise-contracts/Cargo.toml
echo "WASM built successfully."
echo ""

# Step 2: Deploy contract to testnet
echo "Deploying contract to $NETWORK..."
CONTRACT_ID=$(soroban contract deploy \
    --wasm "$WASM_PATH" \
    --network "$NETWORK" \
    --source "$CREATOR")

echo "Contract deployed: $CONTRACT_ID"
echo ""

# Step 3: Initialize campaign
echo "Initializing campaign..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$CREATOR" \
    -- \
    initialize \
    --creator "$CREATOR" \
    --token "$TOKEN" \
    --goal "$GOAL" \
    --deadline "$DEADLINE" \
    --min_contribution "$MIN_CONTRIBUTION"

echo ""
echo "=== Deployment Complete ==="
echo "Contract ID: $CONTRACT_ID"
echo "Creator: $CREATOR"
echo "Token: $TOKEN"
echo "Goal: $GOAL"
echo "Deadline: $DEADLINE"
echo "Min Contribution: $MIN_CONTRIBUTION"
