#!/usr/bin/env python3
"""
Starts a reth dev node via docker compose, deploys a contract that exercises
multiple EVM precompiles, and calls it so the replay example has a meaningful
block to prove.
"""

from __future__ import annotations

import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time


SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
RPC_URL = os.environ.get("RPC_URL", "http://localhost:8545")
SENDER_KEY = (
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
)
MSG_HASH = "0x1111111111111111111111111111111111111111111111111111111111111111"


def run(args: list[str], *, capture_output: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=SCRIPT_DIR,
        text=True,
        check=True,
        capture_output=capture_output,
    )


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise SystemExit(f"{name} not found in PATH; install it first.")


def wait_for_rpc() -> None:
    print("Waiting for RPC to be ready...")
    for _ in range(30):
        result = subprocess.run(
            ["cast", "block-number", "--rpc-url", RPC_URL],
            cwd=SCRIPT_DIR,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode == 0:
            print("RPC is ready.")
            return
        time.sleep(1)

    raise SystemExit(f"Timed out waiting for reth RPC at {RPC_URL}")


def parse_deployed_address(output: str) -> str:
    match = re.search(r"Deployed to:\s*(0x[0-9a-fA-F]{40})", output)
    if match is None:
        raise SystemExit(
            "Failed to parse deployed contract address from forge output:\n" + output
        )
    return match.group(1)


def remove_foundry_artifacts() -> None:
    for directory in ("cache", "out"):
        shutil.rmtree(SCRIPT_DIR / directory, ignore_errors=True)


def main() -> int:
    require_tool("docker")
    require_tool("cast")
    require_tool("forge")

    print("Starting reth dev node...")
    run(["docker", "compose", "-f", str(SCRIPT_DIR / "docker-compose.yml"), "up", "-d"])

    wait_for_rpc()

    print("\nDeploying PrecompileTest contract...")
    deploy = run(
        [
            "forge",
            "create",
            "--rpc-url",
            RPC_URL,
            "--private-key",
            SENDER_KEY,
            "--broadcast",
            "--root",
            str(SCRIPT_DIR),
            str(SCRIPT_DIR / "PrecompileTest.sol:PrecompileTest"),
        ],
        capture_output=True,
    )
    contract = parse_deployed_address(deploy.stdout + deploy.stderr)
    print(f"Deployed to: {contract}")
    remove_foundry_artifacts()

    print("\nSigning test message for ecrecover...")
    signature = run(
        [
            "cast",
            "wallet",
            "sign",
            "--private-key",
            SENDER_KEY,
            "--no-hash",
            MSG_HASH,
        ],
        capture_output=True,
    ).stdout.strip()
    r = "0x" + signature[2:66]
    s = "0x" + signature[66:130]
    v = int(signature[130:132], 16)

    print("Calling callAll()...")
    tx_hash = json.loads(
        run(
            [
                "cast",
                "send",
                "--rpc-url",
                RPC_URL,
                "--private-key",
                SENDER_KEY,
                "--json",
                contract,
                "callAll(bytes32,uint8,bytes32,bytes32)",
                MSG_HASH,
                str(v),
                r,
                s,
            ],
            capture_output=True,
        ).stdout
    )["transactionHash"]

    receipt = json.loads(
        run(
            ["cast", "receipt", "--rpc-url", RPC_URL, "--json", tx_hash],
            capture_output=True,
        ).stdout
    )

    block = int(receipt["blockNumber"], 0)
    gas_used = int(receipt["gasUsed"], 0)
    status = int(receipt["status"], 0)
    num_logs = len(receipt["logs"])

    print(f"\nDone! Block {block}: gas_used={gas_used}, status={status}, logs={num_logs}")
    print()
    print("  cargo airbender build --project examples/reth-block-replay/guest")
    print(
        "  cargo run --release --manifest-path examples/reth-block-replay/host/Cargo.toml "
        f"-- --block-num {block} --prove"
    )
    print(f"BLOCK_NUM={block}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        if error.stdout:
            sys.stderr.write(error.stdout)
        if error.stderr:
            sys.stderr.write(error.stderr)
        raise
