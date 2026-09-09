# UI System

The UI system is built on Dioxus with a custom Bevy renderer. Screens are declared with the `rsx!` macro, data flows through `SharedContext` with generation-based dirty tracking, and the frame registry stores all named UI elements. The design mirrors WoW's frame model (anchors, strata, draw layers) in pure Rust.

## Core Primitives

**Screen + rsx!**: `ui_toolkit::screen::Screen` wraps a Dioxus component. Call `screen.sync(&shared, registry)` to rebuild only screens whose read types have advanced generation. No manual `mark_dirty()` needed.

**SharedContext**: insert any typed value with `shared.insert(state)`. Screens that read that type rebuild automatically.

**FrameRegistry**: stores all frames by name. Named with `FrameName` (has `.0`) or `DynName(String)` for dynamic names.

**Pre-compute negations**: `!bool_expr` doesn't work inside `rsx!` — do `let hide = !visible;` before the macro.

## Frame Hierarchy and Layout

Frames use anchor-based positioning: 9 anchor points (TOPLEFT..BOTTOMRIGHT), relative to any named frame. Strata has 9 levels (WORLD through TOOLTIP); within a stratum, frames use frame levels. Each frame has 5 draw layers (BACKGROUND, BORDER, ARTWORK, OVERLAY, HIGHLIGHT).

**Y-offset sign convention**: positive `y_offset` moves UP (smaller screen Y). WoW XML y values map with the same sign.

Nine-slice borders (`Common-Input-Border.blp`, 128×32, `edge_size: 12.0`) are set after the first `screen.sync()` because rsx! attrs don't cover all frame properties.

## Widget Types

19 widget types matching wow-ui-sim: Frame, Button, CheckButton, Texture, FontString, Line, EditBox, ScrollFrame, Slider, StatusBar, Cooldown, Model/PlayerModel/ModelScene, ColorSelect, MessageFrame, SimpleHTML, GameTooltip, Minimap. See [ui-addon-architecture.md](../../ui-addon-architecture.md) for the full capability matrix.

## Nameplates

Target-first nameplate design: current target gets full plate (name, health, cast), nearby combatants get compact plates, non-hostile/distant actors are hidden or faded. Three display states: hidden, compact, full. State driven by targeting, hostility, recent damage, and distance. See [nameplate-research-2026-03-27.md](../../nameplate-research-2026-03-27.md).

## Unit Frames

PlayerFrame (232×100) and TargetFrame (232×100) mirror WoW's XML structure. PlayerFrame anchored at `x=268 y=850`; TargetFrame at `x=1100 y=850`. Both use real replicated ECS data: `LocalPlayer` + `Health`/`Mana` components; target via `CurrentTarget(Entity)`. Font: `FRIZQT__.TTF` 10px (`GameFontNormalSmall`). See [inworld-unit-frames-reference.md](../../inworld-unit-frames-reference.md).

## World Builder Sidebar

The opt-in `--world-builder` plugin adds an InWorld diagnostic sidebar without changing normal runs. Its `Screen` reads a stable `SharedContext` view model, uses entity-derived frame names, preserves EditBox focus across rebuilds, and handles pagination in the engine because `ScrollFrame` is passive. F9 is a fixed visibility toggle. See [[world-builder]].

## UI Automation

JavaScript-driven automation for testing UI flows:

```bash
LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- \
  --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js
```

Available API: `ui.click(name)`, `ui.type(text)`, `ui.key(name)`, `ui.waitForState(name, secs)`, `ui.waitForFrame(name, secs)`, `ui.dumpTree()`, `ui.dumpUiTree()`, `env.NAME`.

## Runtime scheduling

Commits `1a8c6d58`, `cd47743e`, `9a6b6679`, `ae222f0e`, `e9b81652`, and `fc99128b` remove the combined clean UI frame path: screen sync and pointer hit-testing require relevant changes; automation requires queued work; addon application runs after load/reload; watcher handling sleeps through clean frames; cooldown display advancement runs at `NetworkTick` only while active. Rendering, active interaction, and active presentation remain render-frame-driven.

`ui-toolkit` `0f5d81c` keeps that per-frame visual synchronization but avoids replacing an existing `Transform` or `Sprite` when its computed value is identical, for ordinary and nine-slice backdrop quads. It compares all sprite fields written by the sync path; genuine layout, color, texture, clipping, and default-value updates still write immediately. It does not use `render_dirty` as a skip gate and does not change rendering cadence. Tests through `3eb9aa7` cover unchanged ordinary/backdrop values and changed geometry/color/texture/default behavior. CPU gain remains unproven; native comparable measurement is required.

Engine commit `6edcdc91` similarly runs nameplate and quest-indicator HUD visibility synchronization every `Update`, but uses `Visibility::set_if_neq` so components already at the requested state are not marked changed. Commit `6637a6d2` composes kind/colorblind RGB and distance alpha before one `TextColor::set_if_neq` write. It replaces the separate base-color and fade writers, which reset and restored alpha each frame with unordered final output. Seventeen focused tests cover settled faded plates, real camera/distance/color changes, newly added plates, and missing/nonunique camera behavior. This supersedes the base-color-only comparison in `0357727b`; whole-engine CPU savings remain unmeasured. Focused tests also cover unchanged components and HUD show/hide toggles on each update.

Commit `275bd84c` applies equivalent comparisons to world health bars: unchanged health preserves the foreground transform and material color, while unchanged HUD visibility preserves `Visibility`. Health and HUD changes still update immediately. Commit `56f82791` retains nameplate, quest-indicator, and health-bar billboarding every update but only writes a rotation when the camera-facing result differs. It preserves transform translation and scale. Six focused tests follow four behavioral RED failures. These are mutation-boundary proofs, not whole-engine CPU claims.

## Keybindings

Configurable bindings cover in-world gameplay: movement (forward/backward/strafe/jump/run/autorun), camera (turn/pitch/zoom), targeting, action bar slots 1–12, audio mute. Fixed (non-bindable) inputs: LMB+RMB chord, login/charselect/menu screen keys, debug controls. See [keybindings-scope.md](../../keybindings-scope.md).

## In-World Diagnostic Stage Boundary

Pre-`Ui` cumulative stages must not build or synchronize game UI. The empty-stage client produced **853,196** repeated `UIActionBar.BLP` blacklist lines (**75.9 MB**) because `UiRenderEnabled(false)` originally gated only the inner render systems; frame builders, registry/layout work, texture-related frame processing, button input, and game-UI observers still ran.

`game-engine` commit `508891a6` gates in-world UI builders and sync systems with `inworld_scene_stage_allows_ui`, including minimap, action bars, unit frames, frame plugins, group frames, game-menu in-world paths, quest sparkles, and nameplate/health-bar observers. Cursor and addon/panel-style processing are also stopped before the `Ui` stage. `OnExit(InWorld)` teardown remains unconditional.

`ui-toolkit` commit `50e4a17` adds default-enabled `UiProcessingEnabled` around the complete chained `Update` UI schedule: screen-size synchronization, layout, button nine-slice conversion, render systems, and button input. `UiRenderEnabled` remains the inner render-only gate; the standalone Bevy FPS overlay is separate and remains active for existing pre-`Ui` stages.

Engine commits `a60cbc38918ec27c730a2166fb8312223501fb6b` and `c299491b` add opt-in `--no-ui`, which disables toolkit processing, quad rendering, text rendering, and world-space HUD visuals for every stage while forcing the numeric FPS counter visible and keeping its frame-time graph hidden despite saved HUD/menu settings. It retains UI plugins/resources for dependent systems and leaves 3D rendering intact. Without the flag, defaults and pre-`Ui` diagnostic behavior are unchanged.

The shared UI gate also hides the sky clock and blocks health-bar/player-nameplate/NPC-nameplate observer creation before meshes, materials, and visible default entities are created. This prevents observer output surviving after its update systems are disabled.

Machine-side proof at `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` used behavior commit `508891a6` and toolkit commit `50e4a17`: the client reached connected `InWorld` with one link, one local player, and 133 remote entities. The toolkit UI tree and `MainActionBar` filter were empty; stderr contained zero `[UI]` lines and zero `UIActionBar.BLP` blacklist lines, with no font panic, GPU OOM, device-loss, or panic mentions. `ping` and `performance` remained responsive. The client was left running for visual inspection at capture time; current live-client state and the launch gate are tracked in [[procedural-cloud-regeneration]]. The three recorded performance samples are not comparative evidence and do not establish an FPS improvement.

Commit `8cac2b03` first disabled only the FPS frame-time graph at startup in strict Empty, but that setting was later overwritten by `apply_loaded_client_options`, `sync_hud_visibility_toggles`, and `apply_snapshot_to_world`; `/tmp/claude/game-engine-perf/empty-fps-graph-2246158.webp` shows the resulting solid red graph. Commit `cc5780a8` makes all overlay visibility writers stage-aware: Empty keeps visible FPS text while the graph remains disabled, and Character/later stages restore it. `/tmp/claude/game-engine-perf/empty-fps-graph-options-2283621.webp` confirms the graph is absent. Its 9.80%-at-9.95-FPS result is historical capped data, not a CPU baseline or authorization. `281d291a` removes forced Empty pacing; canonical retirement and pending uncapped Green status: [[procedural-cloud-regeneration#strict-empty-pacing-retirement]].

## Known Issues

**Hotreload frame stability**: on Dioxus hotreload, changed static attrs become dynamic, producing a new `Template` that doesn't match the old one. `diff_node` tears down and rebuilds the entire frame tree, making cached frame IDs stale. Fix: replace `templates: Vec<Template>` with `HashMap<TemplateGlobalKey, Template>` in `GameUiRenderer`. See [hotreload-frame-stability.md](../../hotreload-frame-stability.md).

**EditBox focus visual**: nine-slice center part only covers the interior (inset by `edge_size`). Border textures have transparent inner areas, creating a gap between center fill and border line. WoW solves this with `backdropColor`; current approaches either produce square corners or leave unfilled strips. See [editbox-focus-texture-swap-2026-04-06.md](../../editbox-focus-texture-swap-2026-04-06.md).

## Sources

- [ui-addon-architecture.md](../../ui-addon-architecture.md) — widget types, layout system, addon WASM design, wow-ui-sim parity
- [login-ui-porting.md](../../login-ui-porting.md) — nine-slice editboxes, anchor layout, y-offset convention
- [hotreload-frame-stability.md](../../hotreload-frame-stability.md) — template key bug, fix approach
- [ui-automation-debugging.md](../../ui-automation-debugging.md) — JS automation API, debug scripts
- [editbox-focus-texture-swap-2026-04-06.md](../../editbox-focus-texture-swap-2026-04-06.md) — focus visual problem, core nine-slice gap issue
- [inworld-unit-frames-reference.md](../../inworld-unit-frames-reference.md) — PlayerFrame/TargetFrame geometry
- [wow-ui-sim-layout-spec-2026-03-31.md](../../wow-ui-sim-layout-spec-2026-03-31.md) — exact pixel geometry for frames and tabs
- [nameplate-research-2026-03-27.md](../../nameplate-research-2026-03-27.md) — nameplate design research
- [keybindings-scope.md](../../keybindings-scope.md) — bindable vs fixed inputs
- [in-world stage gating](../../../src/game/state/inworld_scene_stage.rs) — cumulative stage predicates
- [toolkit resource gates](../../../src/main.rs) — pre-`Ui` processing/render/text resource configuration
- [cursor and panel-style gates](../../../src/app_setup.rs) — pre-`Ui` startup/update registration
- [ui-toolkit processing gate](../../../../ui-toolkit/src/plugin.rs) — registry/layout/input/render schedule boundary
- [nameplate visibility synchronization](../../../src/rendering/ui/nameplate.rs) — per-Update HUD visibility writes and change detection
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — machine-side connected empty-stage relaunch proof

## See Also

- [[networking]] — login auth flow feeds into UI state transitions
- [[rendering-pipeline]] — UI renders on top of 3D scene
- [[procedural-cloud-regeneration]] — empty-stage performance investigation and machine-side relaunch proof; human visual gate pending
- [[world-builder]] — diagnostic sidebar built on Screen, SharedContext, and FrameRegistry
