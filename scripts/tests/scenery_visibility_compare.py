#!/usr/bin/env python3
"""Compare real client scenery receipts from reference and candidate Stormwind runs."""

import argparse
import json
from pathlib import Path


def read_receipt(path: Path) -> dict[str, dict]:
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    by_path = {row["path"]: row for row in rows}
    if len(rows) != len(by_path):
        raise ValueError(f"duplicate scenery paths in {path}")
    if len(rows) < 1000 or not any(row["visible"] for row in rows):
        raise ValueError(f"insufficient real scenery in {path}: {len(rows)} placements")
    return by_path


def compare(reference: Path, candidate: Path) -> None:
    before = read_receipt(reference)
    after = read_receipt(candidate)
    changed = sorted(
        path for path in before.keys() | after.keys() if before.get(path) != after.get(path)
    )
    if changed:
        raise AssertionError(f"{len(changed)} scenery records differ: {changed[:10]}")
    print(
        f"PASS {reference.name}: {len(before)} identical scenery visibility and shadow records"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    for pose in ("indoor", "outdoor"):
        compare(
            args.reference / f"{pose}-scenery.jsonl",
            args.candidate / f"{pose}-scenery.jsonl",
        )


if __name__ == "__main__":
    main()
