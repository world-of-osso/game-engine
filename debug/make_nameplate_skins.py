"""Extract text-free nameplate art from the user-supplied reference.

Run: uv run --with pillow python debug/make_nameplate_skins.py
Source is intentionally not bundled: data/diagnostics/nameplate-style/reference.png.
Coordinates are half-open, unscaled screenshot pixels. Runtime owns all resizing
and dynamic value clipping; these images contain no health/cast values or labels.
"""

from collections import deque
import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "data/diagnostics/nameplate-style/reference.png"
OUTPUT = ROOT / "src/rendering/ui/nameplate_skins"
BACKGROUND = (24, 21, 20)
# The frame bands exclude all text and status fill. Endcaps remain outside the
# inner rectangle; no text-bearing center pixels are copied into frame skins.
FRAMES = {
    "health-thick": ((50, 48, 446, 96), (58, 54, 434, 92)),
    "health-thin": ((521, 67, 917, 97), (529, 73, 905, 92)),
    "cast-thin": ((32, 92, 464, 114), (60, 98, 432, 109)),
    "cast-thick": ((32, 289, 466, 318), (61, 296, 433, 314)),
}
FILL = (529, 73, 816, 92)
THICK_FILL = (58, 54, 345, 92)


def linear(channel):
    value = channel / 255.0
    return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4


def srgb(value):
    value = max(0.0, min(1.0, value))
    encoded = (
        value * 12.92 if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
    )
    return round(encoded * 255)


def inside(x, y, rect):
    return rect[0] <= x < rect[2] and rect[1] <= y < rect[3]


def unmatte(pixel):
    """Minimal valid alpha against a known linear-light background.

    Darker pixels are represented as black shadows with their own alpha, not
    baked brown background. Mixed/chromatic pixels use a bounded RGB solution.
    Pixels within three sRGB levels of the matte are discarded as background noise.
    Recovering original RGBA from a single composite is underdetermined; this is
    a reproducible reconstruction, not a claim to recover original source art.
    """
    if max(abs(p - b) for p, b in zip(pixel, BACKGROUND)) <= 3:
        return (0, 0, 0, 0)
    observed = tuple(linear(p) for p in pixel)
    matte = tuple(linear(b) for b in BACKGROUND)
    if all(p <= b for p, b in zip(observed, matte)):
        alpha = 1 - sum(observed) / sum(matte)
        return (0, 0, 0, round(alpha * 255))
    alpha = max(
        (p - b) / (1 - b) if p >= b else (b - p) / b for p, b in zip(observed, matte)
    )
    # Quantize alpha upward so the reconstructed foreground remains in gamut.
    alpha_byte = min(255, int(alpha * 255 + 0.999999))
    alpha = alpha_byte / 255
    foreground = tuple(
        srgb((p - b * (1 - alpha)) / alpha) for p, b in zip(observed, matte)
    )
    return (*foreground, alpha_byte)


def extract_frame(source, crop, interior, name):
    skin = Image.new("RGBA", (crop[2] - crop[0], crop[3] - crop[1]))
    for y in range(crop[1], crop[3]):
        for x in range(crop[0], crop[2]):
            # Remove the neighboring cast glow from the health crop.
            if name == "health-thick" and y >= 94:
                continue
            if name == "health-thin" and y >= 95:
                continue
            if not inside(x, y, interior):
                # Glyph fringes touch this rail; use its clean right segment.
                sample_x = (
                    350
                    if name == "cast-thick" and 61 <= x < 240 and 294 <= y < 296
                    else x
                )
                pixel = source.getpixel((sample_x, y))
                if (
                    name.startswith("health")
                    and pixel[0] > 70
                    and pixel[0] > 1.6 * max(pixel[1:])
                ):
                    continue
                skin.putpixel((x - crop[0], y - crop[1]), unmatte(pixel))
    bleed_transparent_rgb(skin)
    return skin


def bleed_transparent_rgb(skin):
    """Avoid dark fringes when straight-alpha textures are linearly filtered."""
    pixels = skin.load()
    visited = {
        (x, y) for y in range(skin.height) for x in range(skin.width) if pixels[x, y][3]
    }
    pending = deque(sorted(visited))
    while pending:
        x, y = pending.popleft()
        for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if (
                0 <= nx < skin.width
                and 0 <= ny < skin.height
                and (nx, ny) not in visited
            ):
                pixels[nx, ny] = (*pixels[x, y][:3], 0)
                visited.add((nx, ny))
                pending.append((nx, ny))


def extract_thick_fill(source):
    fill = source.crop(THICK_FILL).convert("RGBA")
    # Reconstruct only the label-covered region from clean rows above/below it.
    for x in range(6, 242):
        upper = tuple(linear(c) for c in fill.getpixel((x, 7))[:3])
        lower = tuple(linear(c) for c in fill.getpixel((x, 34))[:3])
        for y in range(8, 34):
            fraction = (y - 7) / 27
            color = tuple(srgb(a + (b - a) * fraction) for a, b in zip(upper, lower))
            fill.putpixel((x, y), (*color, 255))
    return fill


def main():
    source = Image.open(SOURCE).convert("RGB")
    if source.size != (979, 364):
        raise ValueError(f"Unexpected reference dimensions: {source.size}")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    metadata = {
        "source": str(SOURCE.relative_to(ROOT)),
        "source_sha256": hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
        "source_size": list(source.size),
        "coordinates": "half-open, unscaled reference pixels",
        "matte_srgb": list(BACKGROUND),
        "alpha_method": "linear RGB minimal valid alpha; black shadow alpha; <=3 sRGB matte noise removed; transparent RGB edge bleed",
        "limitations": "Original alpha is not uniquely recoverable. Textured background and screenshot resampling remain uncertain. Health bottom rows overlapping cast glow are removed. Thick cast rail x61:240/y294:296 is reconstructed from clean x350 to remove glyph fringes. GPU comparison required.",
        "frames": {},
    }
    for name, (crop, interior) in FRAMES.items():
        skin = extract_frame(source, crop, interior, name)
        local = (
            interior[0] - crop[0],
            interior[1] - crop[1],
            interior[2] - crop[0],
            interior[3] - crop[1],
        )
        assert skin.crop(local).getchannel("A").getbbox() is None, name
        skin.save(OUTPUT / f"{name}.png")
        metadata["frames"][name] = {
            "crop": crop,
            "size": skin.size,
            "transparent_interior": local,
        }
    # A complete gradient sample, not a complete status bar. No empty-health
    # portion is included; runtime maps its full width onto the current value.
    fill = source.crop(FILL).convert("RGBA")
    fill.save(OUTPUT / "health-fill.png")
    extract_thick_fill(source).save(OUTPUT / "health-fill-thick.png")
    metadata["thick_fill"] = {
        "crop": THICK_FILL,
        "reconstructed_label_rectangle": (6, 8, 242, 34),
        "clean_rows": (7, 34),
    }
    metadata["fill"] = {
        "crop": FILL,
        "size": fill.size,
        "role": "glyph-free colored gradient; stretch to dynamic filled width",
    }
    (OUTPUT / "provenance.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
