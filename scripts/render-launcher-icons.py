#!/usr/bin/env python3
"""Render the phase-one launcher SVG candidates with resvg (a build-only tool)."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ROOT / "godot/ui/launcher_icons"
RESVG = Path.home() / ".cargo/bin/resvg"
PALETTES = {
    "modern": {},
    "forever": {
        "#171c23": "#241b13",
        "#303944": "#493728",
        "#6a5632": "#876039",
        "#e6c47a": "#d6ad62",
        "#0a0e13": "#100c09",
    },
}


def render_candidates():
    if not RESVG.is_file():
        raise SystemExit("Missing ~/.cargo/bin/resvg; install with cargo install resvg --locked")
    for skin, palette in PALETTES.items():
        for source in sorted(SOURCES.glob("*/*.svg")):
            destination = SOURCES / "png" / skin / source.parent.name / f"{source.stem}.png"
            destination.parent.mkdir(parents=True, exist_ok=True)
            svg = source.read_text()
            for original, tinted in palette.items():
                svg = svg.replace(original, tinted)
            # resvg accepts stdin; no intermediate recoloured source files.
            subprocess.run([str(RESVG), "--width", "96", "-", str(destination)],
                           input=svg.encode(), check=True)
            print(destination.relative_to(ROOT))


if __name__ == "__main__":
    render_candidates()
