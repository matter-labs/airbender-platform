#!/usr/bin/env bash
#
# Starts a reth dev node via docker-compose, deploys a contract that exercises
# EVM precompiles (ecrecover, bn256Add, bn256ScalarMul), and calls it so the
# replay example has a meaningful block to prove.
#
# Pre-funded accounts use the standard Foundry mnemonic:
#   test test test test test test test test test test test junk
# First derived address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RPC_URL="${RPC_URL:-http://localhost:8545}"
SENDER="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
SENDER_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

echo "Starting reth dev node..."
docker compose -f "$SCRIPT_DIR/docker-compose.yml" up -d

echo "Waiting for RPC to be ready..."
for i in $(seq 1 30); do
    if cast block-number --rpc-url "$RPC_URL" >/dev/null 2>&1; then
        echo "RPC is ready."
        break
    fi
    sleep 1
done

echo ""
echo "Deploying PrecompileTest contract..."
DEPLOY_OUT=$(forge create \
    --rpc-url "$RPC_URL" \
    --private-key "$SENDER_KEY" \
    --broadcast \
    --root "$SCRIPT_DIR" \
    "$SCRIPT_DIR/PrecompileTest.sol:PrecompileTest" 2>&1)
CONTRACT=$(echo "$DEPLOY_OUT" | grep "Deployed to:" | awk '{print $3}')
echo "Deployed to: $CONTRACT"
rm -rf "$SCRIPT_DIR/cache" "$SCRIPT_DIR/out"

echo ""
echo "Signing test message for ecrecover..."
MSG_HASH="0x1111111111111111111111111111111111111111111111111111111111111111"
SIG=$(cast wallet sign --private-key "$SENDER_KEY" --no-hash "$MSG_HASH")
R="0x$(echo "$SIG" | cut -c 3-66)"
S="0x$(echo "$SIG" | cut -c 67-130)"
V_HEX=$(echo "$SIG" | cut -c 131-132)
V=$((16#$V_HEX))

echo "Calling callAll()..."
TX_HASH=$(cast send \
    --rpc-url "$RPC_URL" \
    --private-key "$SENDER_KEY" \
    --json \
    "$CONTRACT" \
    "callAll(bytes32,uint8,bytes32,bytes32)" \
    "$MSG_HASH" "$V" "$R" "$S" | jq -r '.transactionHash')

RECEIPT=$(cast receipt --rpc-url "$RPC_URL" --json "$TX_HASH")
BLOCK=$(echo "$RECEIPT" | jq -r '.blockNumber' | xargs printf "%d")
GAS=$(echo "$RECEIPT" | jq -r '.gasUsed' | xargs printf "%d")
STATUS=$(echo "$RECEIPT" | jq -r '.status' | xargs printf "%d")
NUM_LOGS=$(echo "$RECEIPT" | jq '.logs | length')

echo ""
echo "Done! Block $BLOCK: gas_used=$GAS, status=$STATUS, logs=$NUM_LOGS"
echo ""
echo "  cd guest && cargo airbender build"
echo "  cd ../host && BLOCK_NUM=$BLOCK cargo run -- --prove"
