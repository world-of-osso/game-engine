# Cage native capture resolution

Verified 2026-10-08: headless cage constrained the pinned Godot's window and viewport to 1280×720 despite `--resolution 1920x1080`. The compositor output, not the spellbook's fitting code, caused the undersized proof.

## Cause and capture-only workaround

The installed wlroots 0.20 auto-created a 1280×720 headless output; cage sized its sole client to that output. `probe.log` records screen/window/viewport all 1280×720. A narrowly scoped `LD_PRELOAD` interposer for `wlr_headless_add_output` creates a 1920×1080 output; `probe-fixed.log` records all three sizes as 1920×1080. The interposer affects cage's headless output creation and is explicitly removed from Godot's environment. It relies on the installed unstable wlroots 0.20 ABI; retire it when cage exposes headless output-mode configuration. No product window settings changed.

## Spellbook proof

`capture_ui_screen.gd`'s `spellbook_both` uses the production projection and offline local-catalog Frost mage snapshot. It captures Spellbook, Specialization and Talents in Modern and Forever, requiring actual 1920×1080 window and image sizes. All six PNGs were inspected. Native geometry records bottom buttons at 100×32, x=173/274/375, y=962; category buttons at 100×32, x=221/322, y=123. Labels remain readable without observed clipping; selected gold bottom art and selected/unselected category art render. Talents intentionally has no content, per the spec.

Specialization evidence is incomplete visually: missing local `4626004.blp` (background/borders), `4626025.blp` (mage thumbnails), and `135810.blp` (Fire icon) leave blank art. Capture logs report each missing asset. No substitute art or code workaround was added. `casc-local` was not found on PATH or in the asset-resolver's normal debug/release target paths; assets were not downloaded or committed.

## Sources

- [`capture_ui_screen.gd`](../../../godot/tests/capture_ui_screen.gd) — native capture and size assertion.
- [Spellbook contract](../../specs/spellbook-action-bar.md) — geometry, pages and preview entry points.
- Evidence: `data/diagnostics/spellbookshot-2026-10-08/` (`capture.py`, `cage-headless-mode.c`, resolution probes, six PNG/JSON pairs, `capture.log`).
- `/usr/include/wlroots-0.20/wlr/backend/headless.h` — observed interposer signature.

## See Also

- [[ui-system]] — native UI projection and scale handling.
- [[godot-wayland-exit-hang]] — pinned Godot's Wayland exit fix.
