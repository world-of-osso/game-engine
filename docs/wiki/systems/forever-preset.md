# Forever preset

Forever is a skin and HUD layout preset imitating the FlareUI addon: it changes atlas art, frame presentation and placement, not game rules. It does **not** request Classic mechanics such as ammunition or pet loyalty. The [HUD Edit Mode contract](../../specs/hud-edit-mode.md) owns requirements and reference measurements; this page owns preset wiring and skin selection.

Verified: 2026-10-06. Engine master `2f5f3e79`; inspected ui-toolkit dependency `68719fc9`. This describes current master behavior, not the pending `skinctx` thread-local change.

## Art selection

`ActiveSkin` is the `ui_toolkit::atlas` enum with `Modern` (default) and `Forever`; it is not a separate gameplay mode. Modern resolves DB2 atlas set **0**. Forever resolves set **1**, then set 0 for names without a set-1 member (`ui-toolkit/core/src/atlas/db2.rs:14–30,126–139`). Set 1 supplies the `c60` reskins of existing Retail atlas names. This resolver order is existing behavior, not a new fallback policy.

The host's `set_data_root` initialises Retail tables from `data/db2/12.1.0.69933` and Forever tables from `data/db2/1.60.1.69913` (`godot/ui-model/src/paths.rs:11–16`). The atlas loader imports all Retail records and only Forever set-1 records (`ui-toolkit/core/src/atlas/db2.rs:73–80`). Resolution uses the 1x canvas, ranks matching canvas first and then lower canvas/newer member ID; project-owned regions are checked before DB2 regions (`ui-toolkit/core/src/atlas.rs:9–11,120–132`; `atlas/db2.rs:51–60,126–139`).

Set selection chooses a region and `FileDataID`; it does **not** make every texture lookup product-aware. `scripts/import_forever_atlas.py:1–17,80–95` documents the versioned table import and local Classic-beta extraction of set-1 textures into `data/textures/`. Some products reuse an FDID for different sheet bytes. The portrait-party component therefore binds an explicit file such as **`data/forever-1.60.1.70205/textures/4631591.blp`** (`godot/ui-model/src/ui/screens/portrait_party_frame_component.rs:28–35`). [[portrait-party-frames]] owns that sheet's crop provenance, conditional-art limitations and evidence; do not replace a Retail sheet globally to draw Forever.

### Current active-skin lifetime

The default active skin is currently a **process-wide global** `static ACTIVE_SKIN: AtomicU8`, initialised to Modern. `set_active_skin` and `active_skin` store/load it with relaxed atomic ordering; `get_region` reads this global, whereas `resolve_region(name, skin)` receives an explicit skin (`ui-toolkit/core/src/atlas.rs:58,86–95,120–132`). Per-character persistence does not imply thread-local resolver state or simultaneous independent skins inside one process.

`skinctx` is a pending thread-local change, not part of this master description. HUD settings likewise currently use a process-wide `RwLock<Option<LayoutSettings>>`; each canvas reads skin/settings mirrored into its `SharedContext` (`godot/ui-model/src/ui/hud_layout.rs:478–507`). No code was changed for this documentation pass.

## HUD layout presets

`MODERN` and `FOREVER` are `HudLayout` constants, not separate screen implementations (`godot/ui-model/src/ui/hud_layout.rs:206–285,343–399`). They provide anchors, action-bar layout, sizes and unit-frame styles. Forever explicitly replaces the central unit-frame cluster, cast bar, stacked action bars, pet bar, chat geometry, meter size, objective-tracker position and top-centre XP bar. It ends with **`..MODERN`**, inheriting every field not explicitly overridden, including shared corner placement. Do not assume every HUD frame has a different Forever placement.

`layout_of` selects the constant using `ActiveSkin` and applies optional settings over it; `hud_layout(ctx)` reads the canvas's skin and settings (`godot/ui-model/src/ui/hud_layout.rs:438–507`). Exact dimensions, colours, user-approved deviations and expected behavior belong in [HUD Edit Mode](../../specs/hud-edit-mode.md), not a second table here. [[xp-bar]], [[chat-frame]] and [[portrait-party-frames]] describe their subsystem-specific consumers.

## Edit Mode and character selection

`ui_layout.ron` sits beside `options_settings.ron` in the `world-of-osso` config directory. Saved layouts are account-wide; `edit_mode.active_layout` maps stringified **server character IDs** to layout names (`godot/core/src/ui_layout_data.rs:5–28,226–252`; `godot/rust/src/ui_layout.rs:14–16`). Minimal selection for an owned character is:

```ron
(edit_mode: (active_layout: {"<character_id>": "Forever"}))
```

Replace the placeholder with the actual returned character ID; retain other entries when editing an existing file. For isolated live runs use `$XDG_CONFIG_HOME/world-of-osso/ui_layout.ron`, not another player's config. [Private live-run recipe](../../headless-live-run.md) owns account/server isolation.

The two system presets are `Modern` and `Forever`; an unconfigured character chooses Modern. Selecting a preset persists that character's name mapping. Saving settings while a system preset is active creates a player layout carrying its skin rather than rewriting the preset (`godot/core/src/ui_layout_data.rs:127–151,226–252,269–298`).

InWorld applies the selected character's layout; Login, CharacterSelect and CharacterCreate use Modern, while Loading/GameMenu leave the current layout unchanged. Applying a layout maps `LayoutSkin` to `ActiveSkin`, publishes settings and resyncs registry canvases when skin/settings change (`godot/rust/src/ui_layout.rs:19–35,77–94`). It does not change the server character's race, class, spells or mechanics.

## FlareUI reference boundary

The local reference checkout is **`data/reference/flareui`**: `Core.lua` and `Modules/` provide numbers, colours and layout behavior. Use those references and the approved screenshots cited by the [HUD contract](../../specs/hud-edit-mode.md); do not import FlareUI `Media` artwork, bars or icons. `data/reference/flareui/LICENSE.txt:25–35` excludes `Media/Art`, `Media/Bars` and `Media/Icons` from the source-code license. Existing game-product atlas art and project-owned chrome are distinct from that addon Media.

## Sources

- [HUD Edit Mode contract](../../specs/hud-edit-mode.md) — requirements, reference citations and exact geometry/colour decisions.
- [HUD layout constants](../../../godot/ui-model/src/ui/hud_layout.rs) — presets, inheritance and settings application; line ranges above.
- [Layout persistence](../../../godot/core/src/ui_layout_data.rs) and [runtime selection](../../../godot/rust/src/ui_layout.rs) — saved schema, character selection and registry resync.
- [Atlas initialisation](../../../godot/ui-model/src/paths.rs) and [Forever import](../../../scripts/import_forever_atlas.py) — table directories and set-1 extraction.
- `ui-toolkit/core/src/atlas.rs:58,86–95,120–132`; `ui-toolkit/core/src/atlas/db2.rs:14–30,51–60,73–80,126–139` (sibling dependency `68719fc9`) — process-wide skin and atlas resolution.
- `data/reference/flareui/{Core.lua,Modules/,LICENSE.txt}` — local reference and Media reuse exclusion. Data is external/untracked; not copied into this commit.
- User handoff (2026-10-06) — Forever is presentation, not Classic mechanics; `skinctx` pending scope.

## See Also

- [[ui-system]] — shared screens, context and authored asset resolution.
- [[portrait-party-frames]] — product-specific file binding and conditional set-0 art boundary.
- [[xp-bar]] — preset-specific XP presentation.
- [[chat-frame]] — shared chat state and Forever chrome.
