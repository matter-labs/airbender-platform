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

command -v cast >/dev/null 2>&1 || {
    echo "cast not found in PATH; install Foundry first." >&2
    exit 1
}
command -v forge >/dev/null 2>&1 || {
    echo "forge not found in PATH; install Foundry first." >&2
    exit 1
}

echo "Starting reth dev node..."
docker compose -f "$SCRIPT_DIR/docker-compose.yml" up -d

echo "Waiting for RPC to be ready..."
rpc_ready=false
for i in $(seq 1 30); do
    if cast block-number --rpc-url "$RPC_URL" >/dev/null 2>&1; then
        rpc_ready=true
        echo "RPC is ready."
        break
    fi
    sleep 1
done

if [ "$rpc_ready" != "true" ]; then
    echo "Timed out waiting for reth RPC at $RPC_URL" >&2
    exit 1
fi

echo ""
echo "Deploying PrecompileTest contract..."
DEPLOY_OUT=$(forge create \
    --rpc-url "$RPC_URL" \
    --private-key "$SENDER_KEY" \
    --broadcast \
    --root "$SCRIPT_DIR" \
    "$SCRIPT_DIR/PrecompileTest.sol:PrecompileTest" 2>&1)
CONTRACT=$(echo "$DEPLOY_OUT" | grep "Deployed to:" | awk '{print $3}')
if [ -z "$CONTRACT" ]; then
    echo "Failed to parse deployed contract address from forge output:" >&2
    echo "$DEPLOY_OUT" >&2
    exit 1
fi
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
TX_HASH=$(
    cast send \
        --rpc-url "$RPC_URL" \
        --private-key "$SENDER_KEY" \
        --json \
        "$CONTRACT" \
        "callAll(bytes32,uint8,bytes32,bytes32)" \
        "$MSG_HASH" "$V" "$R" "$S" | \
        python3 -c 'import json, sys; print(json.load(sys.stdin)["transactionHash"])'
)

RECEIPT=$(cast receipt --rpc-url "$RPC_URL" --json "$TX_HASH")
read -r BLOCK GAS STATUS NUM_LOGS <<EOF
$(echo "$RECEIPT" | python3 -c '
import json, sys

receipt = json.load(sys.stdin)

def normalize(value):
    if isinstance(value, str):
        return int(value, 0)
    return int(value)

print(
    normalize(receipt["blockNumber"]),
    normalize(receipt["gasUsed"]),
    normalize(receipt["status"]),
    len(receipt["logs"]),
)
')
EOF

echo ""
echo "Done! Block $BLOCK: gas_used=$GAS, status=$STATUS, logs=$NUM_LOGS"
echo ""
echo "  cargo airbender build --project examples/reth-block-replay/guest"
echo "  BLOCK_NUM=$BLOCK cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml -- --prove"
echo "BLOCK_NUM=$BLOCK"
