# In-world launcher

Native centred search grid. Contract: [launcher spec](../../specs/launcher.md). Existing micro menu remains mounted; its default visibility and controls are unchanged.

## State and dispatch

`godot/ui-model/src/launcher.rs` owns open/query/selected state. Labels match case-insensitive word prefixes; filtering resets selection. Two-column arrow navigation clamps to visible entries. Enter/click returns the entry's action and closes the launcher.

Micro entries derive labels and actions from `MICRO_BUTTONS`. Additional entries invoke existing backpack, map and Options controllers. Key Bindings enters Options directly on Interface bindings. Help invokes the existing Support unavailable message; other unconverted micro windows retain their existing messages.

`godot/rust/src/launcher.rs` observes toggle/navigation before the native search editbox consumes them, drains text before activation, focuses search on opening and releases the canvas on closing. The registry canvas participates in existing UI visitation/scale/hit-testing. Keyboard gameplay stays blocked while launcher is open.

## Art and sources

User chose filled art on October 5, 2026. All seventeen entries and the magnifier use original SVG glyphs with the approved tile bevel and gold silhouette/dark-cutout language. The five approved glyphs remain unchanged. Modern uses slate/gold; Forever keeps its brown/bronze tint. Outlined art, the style switch and empty tiles are removed. Game Menu retains its approved cog; Options uses sliders. Help uses a question mark, not the cog. No launcher Blizzard micro art or FlareUI Media sources remain.

**resvg 0.48.1 is a build-tool dependency**, not a runtime dependency. Install with `cargo install resvg --locked --version 0.48.1`; regenerate with `python3 scripts/render-launcher-icons.py`. Editable SVGs are in `godot/ui/launcher_icons/filled/`; committed transparent 96×96 PNGs are in `png/{modern,forever}/filled/`. [Art build instructions](../../../godot/ui/launcher_icons/README.md).

Launcher uses two 250×40 icon/list cells per row, with 32px icons beside 15pt left-aligned, non-wrapping labels. Row/column gaps are 2px; inset is 12px; grid starts at y86. Panel height follows visible content (526×474 full, 526×138 single result). Launcher-only chrome crops and stretches each skin's metal title strip to 36 units; its 20pt title and 24×24 close button share the vertical centre. Search/list move down 16 units; shared window chrome is unchanged. Per-skin window chrome remains: Modern's standard no-portrait dialog and Forever's existing metal panel. Magnifier opener stays 30×30, 6px left of the minimap cluster and bottom-aligned.

`godot/tests/capture_launcher_candidates.gd` renders the production launcher/HUD composition offline through actual RegistryUi projection and validates native pointer activation. `scripts/capture-launcher-candidates.py` owns a private headless Weston display and captures both skins at 1920×1080 plus 2× launcher/minimap crops. No server state changes. Final evidence uses `-final` under `data/diagnostics/launcherart-2026-10-05/`.

Earlier Blizzard-icon/keyboard-fit evidence in the spec describes superseded launcher art, not current sources. Launcher lifecycle/filter/navigation are the user's October 5 design, not Retail behavior.

## See Also

- [Launcher spec](../../specs/launcher.md)
- [[minimap]] — buff-sized opener beside the cluster
- [[keybindings]] — persisted Toggle Launcher action
