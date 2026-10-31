#!/usr/bin/env python3
"""Run on a rendered display: python3 scripts/tests/particle_texture_cache.py GODOT_BIN."""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WARNING = "WARNING: Call to texture_get_rd_texture causing RenderingServer synchronizations"


def interval(output: str, name: str) -> str:
    begin = f"PARTICLE_TEXTURE_{name}_BEGIN"
    end = f"PARTICLE_TEXTURE_{name}_END"
    if output.count(begin) != 1 or output.count(end) != 1:
        raise ValueError(f"missing or duplicate {name} markers")
    return output.split(begin, 1)[1].split(end, 1)[0]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    result = subprocess.run(
        [sys.argv[1], "--display-driver", "x11", "--audio-driver", "Dummy", "--path",
         str(ROOT / "godot"), "-s", "res://tests/particle_texture_cache.gd"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=180,
        check=False,
    )
    print(result.stdout, end="")
    if result.returncode:
        return result.returncode
    try:
        control = interval(result.stdout, "CONTROL").count(WARNING)
        steady = interval(result.stdout, "STEADY").count(WARNING)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    print(f"texture synchronization warnings: control={control}, steady={steady}")
    return 0 if control > 0 and steady == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
