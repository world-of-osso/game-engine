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

## Layout invalidation

`ui-toolkit` `0fdcf3f` makes an empty `rect_dirty` set a no-op: layout does not collect frames, resolve rectangles, or dirty render state. Layout work begins only when an actual geometry boundary explicitly calls `FrameRegistry::mark_rect_dirty`. Toolkit `a8c846b` also checks this before the caller mutably accesses `UiState`, avoiding false resource-change notifications on clean layout calls. Two behavioral regressions and independent toolkit/engine checks pass; this does not imply other UI systems are idle.

Insertion/removal, screen resize, anchor changes, dimension/flex attributes, flex-child changes, text/EditBox auto-sizing, resolved named anchors, and owned-addon resize propagate through children, anchor dependents, and flex parents. `get_mut` remains render-dirty only; arbitrary raw geometry mutation is not made implicitly layout-dirty. Explicit unanchored cached rectangles retain their existing behavior.

Toolkit proof is 14 focused regressions plus 41 existing layout/screen/diff/plugin tests (55 total). Engine addon integration has 7 GREEN tests at `86bbc945`; the coordinated engine integration check remains pending. No CPU or native claim.

## Visibility and alpha invalidation

`ui-toolkit` `2dec7fe` leaves a frame and its subtree out of `render_dirty` when `set_hidden` or `set_alpha` receives values that leave the frame's stored and derived visibility/alpha unchanged. Actual hidden/alpha/effective-value changes still dirty their changed frames. `ff2acd0` makes `set_hidden` walk descendants once, calculating visibility then effective alpha at each node; conditional writes and stale-derived-value repair remain intact. `set_alpha` keeps its existing alpha-only propagation. Registry proof: 6 RED cases, then 28 GREEN cases after the `ecbd655` refactor characterization. No CPU or native claim.

## Text render reconciliation

Toolkit `c8fa209`/`c3a518a` retains full `sync_ui_text` reconciliation but compares the renderer-owned `Text2d`, layout, bounds, font face/size, color, transform, and anchor values before mutating their Bevy components. It preserves unowned `TextFont` fields and inserts a missing `Anchor` for externally altered entities. Three RED/GREEN behavioral cases and independent toolkit/engine checks pass. No CPU or native claim.

## Shadow text render reconciliation

Toolkit `3e6951d` applies the same compare-before-write policy to existing shadow entities: `Text2d`, layout, bounds, font face/size, color, anchor, and transform update only when their renderer-owned values differ. It reuses the main-text layout/bounds/font helpers, preserves unowned `TextFont` fields, retains full reconciliation and external repair, and does not alter existing shadow alpha semantics. Outline synchronization is unchanged. Three RED/GREEN behavioral cases and independent toolkit/engine checks pass. No CPU or native claim.

## Button nine-slice reconciliation

Toolkit `eebcf28` builds a complete desired `NineSlice` from immutable button state, then compares it before obtaining mutable registry access. `NineSlice` and `TextureSource` derive `PartialEq`, so equality includes texture variants, handles, edge arrays, colors, per-part textures, and UV rectangles. Settled buttons no longer enter `render_dirty`; state, hover, resize, and externally altered derived slice fields still reconcile through the existing full traversal. Three corrected RED/GREEN cases and independent toolkit/engine checks pass. No CPU or native claim.

## Nine-slice sprite reconciliation

Toolkit `67413da` reuses the shared quad visual comparison when reconciling existing nine-slice part entities. Unchanged `Transform` and `Sprite` values are not reinserted; real geometry, color, image, and UV changes still apply. Full traversal, externally altered or missing component repair, spawning, and stale-part removal remain unchanged. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU or native claim.

## Tiled sprite reconciliation

Toolkit `2b3c1f3` applies the shared quad comparison to retained tiled sprites. Unchanged `Transform` and `Sprite` values are not reinserted; tile discovery, real updates, external or missing-component repair, stale-tile cleanup, and registry dirty clearing remain unchanged. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU or native claim.

## Render-dirty clearing

Toolkit `f041c0e` checks `render_dirty` immutably before quad or tiled reconciliation clears it. Empty sets no longer falsely mutate `UiState`; nonempty sets drain at the existing points, after the existing reconciliation work. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU or native claim.

## Three-slice, border, and highlight sprite reconciliation

Toolkit `598ded9` routes retained three-slice parts, backdrop borders, CSS borders, and direct button-highlight overlay entities through the existing full-field `Transform`/`Sprite` comparison helper. Unchanged values are not reinserted; full reconciliation, real visual updates, externally altered or missing-component repair, and stale-entity lifecycle remain unchanged. The suite first recorded 4 failing settled-write cases and 8 passing preservation cases; all 12 current cases pass independently. The highlight evidence exercises its direct synchronization system, not default plugin reachability. No CPU or native claim.

## Button input reconciliation

Toolkit `e25eecc` first collects visible buttons whose `hovered` value differs from the current hit-test result, then mutates only those frames. Press/release handling remains ordered after hover processing and now enters its mutable path only on a left-button edge; disabled and already-correct states remain untouched. Hit testing, disabled hover behavior, pushed-button release reset, and actual hover transitions are unchanged. Five integration cases plus the adapted unit case pass after the five-case RED; independent toolkit/engine checks pass. No CPU or native claim.

## Widget Types

19 widget types matching wow-ui-sim: Frame, Button, CheckButton, Texture, FontString, Line, EditBox, ScrollFrame, Slider, StatusBar, Cooldown, Model/PlayerModel/ModelScene, ColorSelect, MessageFrame, SimpleHTML, GameTooltip, Minimap. See [ui-addon-architecture.md](../../ui-addon-architecture.md) for the full capability matrix.

## Development hot reload

`ui-toolkit` `808117a` moves queued attribute hot-reload patches out of `Screen::sync()` into `UiPlugin` polling once per real-time second in debug builds. Normal shared-state rebuilds, anchor resolution, and auto-sizing stay immediate. Reload is attribute-only for existing named frames; structural reload remains unsupported. A queued edit can take up to one second to appear; no CPU or native UI claim is made.

`ui-toolkit` `a118e8c` collects each screen subtree’s frame IDs once per `Screen::sync()` and reuses them for FontString and EditBox auto-sizing. It adds no cache or change-driven policy. Four characterization tests pass before and after the reorder; no RED or CPU claim.

## Character-creation shared-state updates

`14f4a691` compares the complete character-creation view model before inserting it into `SharedContext`, including focus, labels, and swatches. `screen.sync()` remains unconditional for fresh/replaced screens and normal layout updates; hot-reload polling is now separate. Three focused tests cover settled and changed output; CPU savings are unmeasured.

## Login shared-state updates

`70f14c2a` checks the four login shared values before reinserting them, so unchanged status, connection, realm text, and realm-selectability do not advance dependency generations. `screen.sync()` remains every update for fresh/replaced screens and normal layout updates; hot-reload polling is now separate. Three focused tests and a development compiler check pass; CPU savings are unmeasured.

## Action-bar flash updates

`a00e88aa` compares desired slot backgrounds through immutable access before mutating `UiState` or the registry. Expired/idle flashes no longer dirty unchanged slots; keypress flashes, expiry, and replacement-slot initialization still update. Ten scoped tests and independent verification pass; CPU savings are unmeasured.

## Minimap tracking collection

`d319f9cf` collects tracking icons only after the existing grid/pixel redraw check. Frames that will not redraw no longer scan tracking entities or allocate the discarded point list. Redraws still use current tracking inputs; redraw policy and stationary-icon refresh behavior are unchanged. Thirty-eight scoped tests pass before and after this semantics-preserving reorder; no CPU claim.

## Minimap coordinate updates

`b0f4a006` caches exact raw X/Z coordinate inputs and the formatted string. Static positions skip formatting; replacement frames and external text edits reuse the cached text for correction. The displayed rounding, including signed zero and ties, is unchanged. Seven scoped tests pass; CPU savings are unmeasured.

## Nameplates

Name text is an unparented `Text2d` overlay on the existing `UiCamera` layer. `Camera3d::world_to_viewport` projects the actor's world anchor, then the UI camera converts the logical viewport point to its 2D world. The old text was parented into 3D actor space on layer0, but Bevy's sprite/text queue renders only `Transparent2d` camera phases; it never reached the 3D image. Layer changes alone cannot fix that.

`NameplateOwner` is a linked ownership relationship, separate from transform parenting, so actor despawn removes its overlay label. NPCs use authoritative `Npc.name`; players use their name. Font sizes remain20/24 logical pixels, distance fade uses the owner world anchor, and hidden owners, off-screen/behind-camera anchors, UI stage, `--no-ui`, and HUD toggles determine one final visibility write. Position/global pose and final color update only when changed. Quest indicator M2s stay in their original world-space billboard path.

Six CPU tests cover camera/actor movement, DPI1/2, visibility, fade, ownership cleanup and unchanged writes. A headless GPU fixture using the real NPC observer and both cameras reproduces zero glyph pixels before the fix and renders yellow glyphs at the projected anchor afterward. A bounded native capture shows a named Diseased Timber Wolf; it does not establish long-run wolf animation frequencies. Target-first full/compact combat framing is a separate [design proposal](../../nameplate-research-2026-03-27.md), not implemented by this text projection correction.

## World NPC Picking

Left-click targeting and right-click interaction use the 3D camera, not an unfiltered camera query. The initial `cb2d169f` controller-specific filter was broadened to `Camera3d` so selection-debug cameras without `WowCamera` remain supported. The UI/FPS camera remains present alongside the world camera, including `--no-ui`; requiring a single camera across both silently prevented picking. The final focused proof passes two real mesh-raycast tests with the UI camera, initialized world projection, and an NPC child mesh. `--no-ui` intentionally suppresses health bars, as user-confirmed; world selection does not depend on showing them. Existing UI hit-testing policy is unchanged. Native click verification remains pending.

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

Character-select commit `1262fc58` compares character-list/selection, campsite, and delete-confirmation shared state before reinserting it. `screen.sync`, post-setup, and focus flow remain active; three focused tests cover settled and changed generations. This is mutation-boundary proof, not a whole-engine CPU claim.

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

- `../../data/diagnostics/npc-motion-20260909/target-camera-final.txt` — focused world-camera selection proof
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
- [visibility and alpha propagation](../../../../ui-toolkit/src/registry.rs) — conditional derived-state repair and single-pass `set_hidden` traversal
- [UI layout invalidation spec](../../specs/ui-layout-invalidation.md) — invalidation contract and scope
- `../../data/diagnostics/ui-layout-dirty-20260909/verification/toolkit-report.md` — 14 focused toolkit regressions and mutation audit
- `../../data/diagnostics/ui-layout-dirty-20260909/engine-addon/green.log` — 7 addon integration tests
- [nameplate visibility synchronization](../../../src/rendering/ui/nameplate.rs) — per-Update HUD visibility writes and change detection
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — machine-side connected empty-stage relaunch proof

## See Also

- [[networking]] — login auth flow feeds into UI state transitions
- [[rendering-pipeline]] — UI renders on top of 3D scene
- [[procedural-cloud-regeneration]] — empty-stage performance investigation and machine-side relaunch proof; human visual gate pending
- [[world-builder]] — diagnostic sidebar built on Screen, SharedContext, and FrameRegistry
- [[npc-motion-validation]] — revision-pinned world-picking and `--no-ui` policy evidence
- [UI layout invalidation spec](../../specs/ui-layout-invalidation.md) — explicit geometry invalidation contract
