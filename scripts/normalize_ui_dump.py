#!/usr/bin/env python3
"""Normalize game-engine and wow-ui-sim dump-ui-tree output to comparable JSON.

Usage:
    python3 scripts/normalize_ui_dump.py <dump-file>
    # or pipe:
    game-engine-cli dump-ui-tree | python3 scripts/normalize_ui_dump.py -

Output: JSON array of {name, type, x, y, w, h, visible, depth} sorted by name.

Both dump formats are auto-detected:
  - game-engine:  "Name [Type] (WxH) visible ... x=X y=Y w=W h=H ..."
  - wow-ui-sim:   "Name [Type] (WxH) visible ..."
"""

import json
import re
import sys

# game-engine: x=123 y=456 w=200 h=30
RE_GE_RECT = re.compile(r"x=(-?\d+)\s+y=(-?\d+)\s+w=(-?\d+)\s+h=(-?\d+)")
# wow-ui-sim: (200x30) and position from rect info
RE_SIZE = re.compile(r"\((\d+)x(\d+)\)")
RE_TYPE = re.compile(r"\[(\w+)\]")
RE_WOW_RECT = re.compile(r"rect=\((-?[\d.]+),\s*(-?[\d.]+),\s*(-?[\d.]+),\s*(-?[\d.]+)\)")


def parse_line(line: str) -> dict | None:
    stripped = line.rstrip()
    if not stripped or stripped.startswith("[anchor]") or stripped.startswith("[texture]"):
        return None
    # skip continuation lines (anchors, textures)
    content = stripped.lstrip()
    if content.startswith("["):
        return None

    depth = (len(stripped) - len(content)) // 2

    # Extract name (first token before [Type])
    bracket = content.find(" [")
    if bracket < 0:
        return None
    name = content[:bracket].strip()

    # Extract type
    m_type = RE_TYPE.search(content)
    frame_type = m_type.group(1) if m_type else "Unknown"

    # Extract visibility
    visible = "visible" in content and "hidden" not in content.split("visible")[0][-1:]
    # More robust: check for exact word
    visible = bool(re.search(r"\bvisible\b", content))
    hidden = bool(re.search(r"\bhidden\b", content))
    is_visible = visible and not hidden

    # Extract rect — game-engine format
    m_ge = RE_GE_RECT.search(content)
    if m_ge:
        x, y, w, h = float(m_ge.group(1)), float(m_ge.group(2)), float(m_ge.group(3)), float(m_ge.group(4))
        return {"name": name, "type": frame_type, "x": x, "y": y, "w": w, "h": h, "visible": is_visible, "depth": depth}

    # Extract rect — wow-ui-sim format with rect=()
    m_wow = RE_WOW_RECT.search(content)
    if m_wow:
        x, y, w, h = float(m_wow.group(1)), float(m_wow.group(2)), float(m_wow.group(3)), float(m_wow.group(4))
        return {"name": name, "type": frame_type, "x": x, "y": y, "w": w, "h": h, "visible": is_visible, "depth": depth}

    # Fallback: extract size from (WxH)
    m_size = RE_SIZE.search(content)
    if m_size:
        w, h = float(m_size.group(1)), float(m_size.group(2))
        return {"name": name, "type": frame_type, "x": 0, "y": 0, "w": w, "h": h, "visible": is_visible, "depth": depth}

    return None


def normalize(lines: list[str]) -> list[dict]:
    frames = []
    for line in lines:
        frame = parse_line(line)
        if frame:
            frames.append(frame)
    frames.sort(key=lambda f: f["name"])
    return frames


def main():
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        sys.exit(1)

    path = sys.argv[1]
    if path == "-":
        lines = sys.stdin.readlines()
    else:
        with open(path) as f:
            lines = f.readlines()

    result = normalize(lines)
    json.dump(result, sys.stdout, indent=2)
    print()


if __name__ == "__main__":
    main()
