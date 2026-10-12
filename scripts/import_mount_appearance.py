#!/usr/bin/env python3
"""Extend a read-only NPC appearance cache with authentic MountXDisplay roots."""
import argparse
import csv
import json
import sqlite3
import sys
from pathlib import Path

from import_npc_appearance import import_files


def read_mount_displays(path):
    with path.open(newline="", encoding="utf-8-sig") as handle:
        displays = {int(row["CreatureDisplayInfoID"]) for row in csv.DictReader(handle)}
    displays.discard(0)
    if not displays:
        raise ValueError(f"{path}: no mount display roots")
    return sorted(displays)


def parse_arguments(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("mount-catalog", "db2-dir", "data-dir", "model-cache", "base-cache", "output"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    parser.add_argument("--outfit-cache", type=Path)
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_arguments(argv)
    try:
        args.display_id = read_mount_displays(args.mount_catalog)
        args.pet_catalog = None
        args.world_db = None
        report = import_files(args)
        report["mount_catalog"] = str(args.mount_catalog)
        report["mount_display_roots"] = len(args.display_id)
        report["base_cache"] = str(args.base_cache)
    except (ValueError, KeyError, OSError, sqlite3.Error, csv.Error) as error:
        print(f"Mount appearance import failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
