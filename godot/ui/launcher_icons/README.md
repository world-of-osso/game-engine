# Filled launcher art

Original SVG artwork; no Blizzard micro icons or FlareUI Media. Eighteen distinct glyphs cover all seventeen launcher entries and the magnifier. Solid gold silhouettes and dark cutouts use the approved 48-unit bevelled tile and 2.4-unit glyph weight. Modern uses slate/gold; Forever keeps its brown/bronze tint. No alternate style or placeholder tiles remain.

## Build-tool dependency

Install **resvg 0.48.1**, a build-only SVG rasterizer: `cargo install resvg --locked --version 0.48.1` (installs `~/.cargo/bin/resvg`). Regenerate with `python3 scripts/render-launcher-icons.py` from the checkout root. It renders transparent 96×96 PNGs under `png/{modern,forever}/filled/`. Commit regenerated PNGs with SVG edits; runtime requires no SVG renderer.

Launcher icons draw at 32×32 beside 15pt one-line labels. Minimap magnifier stays 30×30, left of the minimap and bottom-aligned. [Launcher contract](../../../docs/specs/launcher.md).
