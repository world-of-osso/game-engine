#!/usr/bin/env python3
"""Compare two normalized UI frame tree JSON files and report differences.

Usage:
    python3 scripts/diff_ui_trees.py <expected.json> <actual.json> [--threshold N]

Reads JSON arrays produced by normalize_ui_dump.py.
Reports:
  - Frames present in expected but missing from actual
  - Frames present in actual but not in expected
  - Position/size mismatches exceeding threshold (default 1px)
  - Visibility mismatches

Exit code: 0 if identical, 1 if differences found.
"""

import json
import sys

DEFAULT_THRESHOLD = 1.0


def load_frames(path: str) -> dict[str, dict]:
    with open(path) as f:
        frames = json.load(f)
    return {frame["name"]: frame for frame in frames}


def diff_trees(expected: dict, actual: dict, threshold: float) -> list[str]:
    issues = []

    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))

    for name in missing:
        issues.append(f"MISSING  {name}")
    for name in extra:
        issues.append(f"EXTRA    {name}")

    for name in sorted(set(expected) & set(actual)):
        e = expected[name]
        a = actual[name]

        if e.get("visible") != a.get("visible"):
            issues.append(f"VIS      {name}: expected={e['visible']} actual={a['visible']}")

        for key in ("x", "y", "w", "h"):
            ev = e.get(key, 0)
            av = a.get(key, 0)
            delta = abs(ev - av)
            if delta > threshold:
                issues.append(f"RECT     {name}.{key}: expected={ev:.0f} actual={av:.0f} delta={delta:.0f}")

    return issues


def main():
    args = sys.argv[1:]
    threshold = DEFAULT_THRESHOLD

    if "--threshold" in args:
        idx = args.index("--threshold")
        threshold = float(args[idx + 1])
        args = args[:idx] + args[idx + 2:]

    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        sys.exit(2)

    expected = load_frames(args[0])
    actual = load_frames(args[1])

    issues = diff_trees(expected, actual, threshold)

    if not issues:
        print("OK — frame trees match")
        sys.exit(0)

    for issue in issues:
        print(issue)

    summary_missing = sum(1 for i in issues if i.startswith("MISSING"))
    summary_extra = sum(1 for i in issues if i.startswith("EXTRA"))
    summary_rect = sum(1 for i in issues if i.startswith("RECT"))
    summary_vis = sum(1 for i in issues if i.startswith("VIS"))
    print(f"\n{len(issues)} issues: {summary_missing} missing, {summary_extra} extra, {summary_rect} rect, {summary_vis} visibility")
    sys.exit(1)


if __name__ == "__main__":
    main()
