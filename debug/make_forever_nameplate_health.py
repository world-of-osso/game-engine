"""Generate numeric Forever health art; no addon Media pixels.

Run with the same Pillow environment as make_nameplate_skins.py.
Source: data/diagnostics/forever-reference/user-nameplate-reference-2026-10-04.png.
Fill HSV values at (54,35)/(440,35): 74/144. Edge at (250,67): (63,84,104).
The 3x3 hollow patch is drawn with fixed 1px NinePatchRect margins.
"""
from pathlib import Path

from PIL import Image

OUTPUT = Path(__file__).resolve().parents[1] / "godot/rust/src/rendering/ui/nameplate_skins"


def main():
    fill = Image.new("RGBA", (256, 1))
    fill.putdata([(value, value, value, 255)
                  for x in range(256)
                  for value in [round(74 + (144 - 74) * x / 255)]])
    fill.save(OUTPUT / "forever-health-fill.png")
    frame = Image.new("RGBA", (3, 3), (63, 84, 104, 255))
    frame.putpixel((1, 1), (0, 0, 0, 0))
    frame.save(OUTPUT / "forever-health-edge.png")


if __name__ == "__main__":
    main()
