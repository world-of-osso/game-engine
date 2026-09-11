"""Report reference-vs-GPU nameplate errors; text rasterization is excluded.

Run: uv run --with pillow python debug/compare_nameplates.py
This diagnostic reports mismatches; its exit status is not pixel-match acceptance.
"""

from collections import Counter
import json
from pathlib import Path

from PIL import Image, ImageChops

from make_nameplate_skins import linear, srgb

ROOT = Path(__file__).resolve().parents[1]
ARTIFACTS = ROOT / "data/diagnostics/nameplate-pixel-match"
REFERENCE = ROOT / "data/diagnostics/nameplate-style/reference.png"
# Bounds are reference screenshot pixels, not inferred from the current renderer.
CASES = {
    "thick-thin": ((32, 42, 466, 136), ((63, 59, 299, 88), (65, 112, 200, 134))),
    "thin-thin": ((497, 38, 937, 136), ((528, 42, 765, 70), (530, 112, 665, 134))),
    "thick-thick": ((31, 240, 468, 326), ((65, 257, 301, 287), (66, 292, 205, 316))),
    "thin-thick": ((493, 237, 940, 326), ((532, 242, 770, 269), (532, 292, 671, 316))),
}


def inside(point, rectangle):
    x, y = point
    left, top, right, bottom = rectangle
    return left <= x < right and top <= y < bottom


def compare(reference, actual, region, text_regions):
    errors = []
    red = []
    for y in range(region[1], region[3]):
        for x in range(region[0], region[2]):
            if any(inside((x, y), text) for text in text_regions):
                continue
            expected = reference.getpixel((x, y))
            observed = actual.getpixel((x, y))
            errors.append(max(abs(a - b) for a, b in zip(expected, observed)))
            if observed[0] > 100 and observed[1] < 90 and observed[2] < 90:
                red.append((x, y))
    histogram = Counter(errors)
    return {
        "compared_pixels": len(errors),
        "mean_max_channel_error": sum(errors) / len(errors),
        "exact_pixel_fraction": histogram[0] / len(errors),
        "within_4_fraction": sum(n for error, n in histogram.items() if error <= 4)
        / len(errors),
        "maximum_channel_error": max(errors),
        "red_bounds_outside_text": [
            min(x for x, _ in red),
            min(y for _, y in red),
            max(x for x, _ in red),
            max(y for _, y in red),
        ]
        if red
        else None,
    }


def linear_half_reference(source):
    channels = []
    for channel in source.crop((0, 0, 978, 364)).split():
        reduced = channel.point([linear(value) for value in range(256)], "F").resize(
            (489, 182), Image.Resampling.BOX
        )
        channels.append(
            Image.frombytes(
                "L",
                reduced.size,
                bytes(srgb(value) for value in reduced.get_flattened_data()),
            )
        )
    return Image.merge("RGB", channels)


def main():
    source = Image.open(REFERENCE).convert("RGB")
    # Drop the final background-only column for an exact 2:1 reduction.
    reference = source.crop((0, 0, 978, 364)).resize((489, 182), Image.Resampling.BOX)
    linear_reference = linear_half_reference(source)
    composed = Image.new("RGB", reference.size, (24, 21, 20))
    report = {}
    for name, (region, text_regions) in CASES.items():
        region = tuple(value // 2 for value in region)
        text_regions = tuple(
            tuple(value // 2 for value in text) for text in text_regions
        )
        actual = Image.open(ARTIFACTS / "rendered" / f"{name}.png").convert("RGB")
        if actual.size != reference.size:
            raise ValueError(
                f"{name}: expected canvas {reference.size}, got {actual.size}"
            )
        report[name] = compare(reference, actual, region, text_regions)
        report[name]["linear_light_reference"] = compare(
            linear_reference, actual, region, text_regions
        )
        composed.paste(actual.crop(region), region[:2])
        ImageChops.difference(reference.crop(region), actual.crop(region)).save(
            ARTIFACTS / f"diff-{name}.png"
        )
    composed.save(ARTIFACTS / "rendered-comparison.png")
    side_by_side = Image.new("RGB", (reference.width * 2, reference.height))
    side_by_side.paste(reference, (0, 0))
    side_by_side.paste(composed, (reference.width, 0))
    side_by_side.save(ARTIFACTS / "reference-versus-rendered.png")
    (ARTIFACTS / "pixel-diff.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
