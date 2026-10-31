#!/usr/bin/env python3
"""Exercise dump-scene against an owned, fully loaded Stormwind client.

Run through agent-run with the matching engine CLI, explicit socket and character name.
The caller must start the private server and Forever client, then wait for terrain loading.
"""
import argparse
import subprocess
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--socket", type=Path, required=True)
    parser.add_argument("--character", required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    command = [str(root / "scripts/agent/agent-run"), "large_scene_ipc", str(args.cli), "--socket", str(args.socket)]

    network = subprocess.run(command + ["status", "network"], check=True, capture_output=True, text=True).stdout
    assert "game_state: InWorld" in network and "zone_id: 1519" in network, network
    terrain = subprocess.run(command + ["status", "terrain"], check=True, capture_output=True, text=True).stdout
    assert "pending_tiles: 0" in terrain and "failed_tiles: 0" in terrain, terrain
    scene = subprocess.run(command + ["dump-scene"], check=True, capture_output=True).stdout
    assert 4 * 1024 * 1024 < len(scene) < 64 * 1024 * 1024, len(scene)
    assert b'Scene "root"' in scene and b"WorldUnits" in scene and args.character.encode() in scene
    assert b'Model "' in scene
    print(f"PASS: live Stormwind dump-scene returned {len(scene)} bytes")


if __name__ == "__main__":
    main()
