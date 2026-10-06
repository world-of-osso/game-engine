# Launcher art candidates (phase 1)

Original SVG artwork; no Blizzard micro icons or FlareUI Media. `filled/` uses solid gold silhouettes with dark cutouts; `outline/` uses open gold contours. Both use a 48-unit tile with a 2.4-unit glyph stroke and a dark bevel. Five entries are drawn: Character Info, Talents & Spellbook, Quest Log, Bags and Game Menu. The magnifier is drawn too; all other entries use the explicitly neutral `empty.svg` tile until the user chooses a style.

## Build-tool dependency

Install **resvg**, a build-only SVG rasterizer, with `cargo install resvg --locked` (installs `~/.cargo/bin/resvg`). Run `python3 scripts/render-launcher-icons.py` from this checkout. It renders 96×96 PNGs under `png/{modern,forever}/{filled,outline}/`, preserving transparency. Modern uses slate/gold; Forever uses brown/bronze. Commit regenerated PNGs with SVG edits; runtime requires no SVG renderer.

The runtime pick is the single `ICON_STYLE` constant in `godot/ui-model/src/launcher.rs`. Offline capture tests can select either candidate without changing that default. Launcher icons draw at 40×40; minimap magnifier remains 30×30 at its existing left-of-minimap, bottom-aligned position.
