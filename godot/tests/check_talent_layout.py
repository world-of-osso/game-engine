#!/usr/bin/env python3
"""Check real native capture rects and visible glyphs, not registry construction.

Usage: python3 godot/tests/check_talent_layout.py <capture directory>
The directory contains captures.json from capture_talent_layout.gd and PNGs.
"""
import json
from pathlib import Path
import re
import subprocess
import sys

WIDTH, HEIGHT = 1920, 1080
PAINTED_NODE = re.compile(r"TalentNode\d+(?:Ranks|Border)?$")


def decode_png(path):
    result = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path),
         "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        capture_output=True, check=True,
    )
    assert len(result.stdout) == WIDTH * HEIGHT * 3, f"{path}: wrong framebuffer"
    return result.stdout


def count_ink(pixels, rect, color):
    x, y, width, height = rect
    count = 0
    for row in range(round(y + 2), round(y + height - 2)):
        for column in range(round(x + 2), round(x + width - 2)):
            index = (row * WIDTH + column) * 3
            rgb = pixels[index:index + 3]
            if all(abs(component - color) < 16 for component in rgb):
                count += 1
    return count


def check_capture(directory, capture):
    assert not capture["blocker"], capture["blocker"]
    geometry = capture["geometry"]
    footer = geometry["TalentFooter"]
    failures = []
    for name, rect in geometry.items():
        if not PAINTED_NODE.fullmatch(name):
            continue
        x, y, width, height = rect
        if y + height > footer[1] + 0.51:
            failures.append(f"{name} bottom{y + height:.2f} > footer{footer[1]}")
    pixels = decode_png(directory / capture["filename"])
    counts = {}
    for name, color in [("TalentClassName", 255), ("TalentSpecName", 255),
                        ("TalentHeroSelectionLabel", 255), ("TalentApplyText", 128)]:
        counts[name] = count_ink(pixels, geometry[name], color)
        if counts[name] < 40:
            failures.append(f"{name}: only{counts[name]} visible glyph pixels")
    return {"spec": capture["spec_id"], "skin": capture["skin"],
            "filename": capture["filename"], "ink": counts, "failures": failures}


def main(directory):
    captures = json.loads((directory / "captures.json").read_text())
    results = [check_capture(directory, capture) for capture in captures]
    (directory / "native-layout-proof.json").write_text(json.dumps(results, indent=2))
    failures = [f"{r['filename']}: {failure}" for r in results for failure in r["failures"]]
    print(f"Native layout/ink: {len(results)} captures, {len(failures)} failures")
    assert not failures, "\n".join(failures)


if __name__ == "__main__":
    main(Path(sys.argv[1]))
