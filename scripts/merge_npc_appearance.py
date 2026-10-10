#!/usr/bin/env python3
"""Extend NPC coverage with a strict importer result, preserving every existing row.

Usage: merge_npc_appearance.py <existing.sqlite> <imported.sqlite> <new-output.sqlite>
Only displays outside existing coverage are added. No replacement, ordinary-model
substitute or automatic production-cache promotion is performed.
"""
import contextlib
import json
from pathlib import Path
import sqlite3
import sys

TABLES = ("display_coverage", "appearances", "choices", "geosets")


def read_database(path):
    with contextlib.closing(sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)) as connection:
        return {table: connection.execute(f"SELECT * FROM {table}").fetchall() for table in TABLES}


def select_new_displays(existing, incoming):
    covered = {row[0] for row in existing["display_coverage"]}
    added = {row[0] for row in incoming["display_coverage"]} - covered
    profiles = {row[0] for row in incoming["appearances"]}
    for display, required in incoming["display_coverage"]:
        if display in added and required and display not in profiles:
            raise ValueError(f"required appearance missing for display {display}")
    return added


def merge_coverage(base, incoming_path, output):
    if output.exists():
        raise ValueError(f"output exists: {output}; choose a new path")
    existing, incoming = read_database(base), read_database(incoming_path)
    added = select_new_displays(existing, incoming)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("xb"):
        pass
    try:
        with contextlib.closing(sqlite3.connect(base.resolve().as_uri() + "?mode=ro", uri=True)) as source:
            with contextlib.closing(sqlite3.connect(output)) as target:
                source.backup(target)
                with target:
                    for table in TABLES:
                        rows = [row for row in incoming[table] if row[0] in added]
                        if rows:
                            placeholders = ",".join("?" for _ in rows[0])
                            target.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)
    except Exception:
        output.unlink()
        raise
    return {"added_displays": len(added), "preserved_displays": len(existing["display_coverage"]), "output": str(output)}


def main():
    try:
        report = merge_coverage(*map(Path, sys.argv[1:4]))
    except (ValueError, OSError, sqlite3.Error) as error:
        print(f"NPC coverage merge failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
