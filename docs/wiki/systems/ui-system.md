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

19 widget types matching wow-ui-sim: Frame, Button, CheckButton, Texture, FontString, Line, EditBox, ScrollFrame, Slider, StatusBar, Cooldown, Model/PlayerModel/ModelScene, ColorSelect, MessageFrame, SimpleHTML, GameTooltip, Minimap. See [ui-addon-architecture.md](../ui-addon-architecture.md) for the full capability matrix.

## Nameplates

Target-first nameplate design: current target gets full plate (name, health, cast), nearby combatants get compact plates, non-hostile/distant actors are hidden or faded. Three display states: hidden, compact, full. State driven by targeting, hostility, recent damage, and distance. See [nameplate-research-2026-03-27.md](../nameplate-research-2026-03-27.md).

## Unit Frames

PlayerFrame (232×100) and TargetFrame (232×100) mirror WoW's XML structure. PlayerFrame anchored at `x=268 y=850`; TargetFrame at `x=1100 y=850`. Both use real replicated ECS data: `LocalPlayer` + `Health`/`Mana` components; target via `CurrentTarget(Entity)`. Font: `FRIZQT__.TTF` 10px (`GameFontNormalSmall`). See [inworld-unit-frames-reference.md](../inworld-unit-frames-reference.md).

## UI Automation

JavaScript-driven automation for testing UI flows:

```bash
LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- \
  --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js
```

Available API: `ui.click(name)`, `ui.type(text)`, `ui.key(name)`, `ui.waitForState(name, secs)`, `ui.waitForFrame(name, secs)`, `ui.dumpTree()`, `ui.dumpUiTree()`, `env.NAME`.

## Keybindings

Configurable bindings cover in-world gameplay: movement (forward/backward/strafe/jump/run/autorun), camera (turn/pitch/zoom), targeting, action bar slots 1–12, audio mute. Fixed (non-bindable) inputs: LMB+RMB chord, login/charselect/menu screen keys, debug controls. See [keybindings-scope.md](../keybindings-scope.md).

## Known Issues

**Hotreload frame stability**: on Dioxus hotreload, changed static attrs become dynamic, producing a new `Template` that doesn't match the old one. `diff_node` tears down and rebuilds the entire frame tree, making cached frame IDs stale. Fix: replace `templates: Vec<Template>` with `HashMap<TemplateGlobalKey, Template>` in `GameUiRenderer`. See [hotreload-frame-stability.md](../hotreload-frame-stability.md).

**EditBox focus visual**: nine-slice center part only covers the interior (inset by `edge_size`). Border textures have transparent inner areas, creating a gap between center fill and border line. WoW solves this with `backdropColor`; current approaches either produce square corners or leave unfilled strips. See [editbox-focus-texture-swap-2026-04-06.md](../editbox-focus-texture-swap-2026-04-06.md).

## Sources

- [ui-addon-architecture.md](../ui-addon-architecture.md) — widget types, layout system, addon WASM design, wow-ui-sim parity
- [login-ui-porting.md](../login-ui-porting.md) — nine-slice editboxes, anchor layout, y-offset convention
- [hotreload-frame-stability.md](../hotreload-frame-stability.md) — template key bug, fix approach
- [ui-automation-debugging.md](../ui-automation-debugging.md) — JS automation API, debug scripts
- [editbox-focus-texture-swap-2026-04-06.md](../editbox-focus-texture-swap-2026-04-06.md) — focus visual problem, core nine-slice gap issue
- [inworld-unit-frames-reference.md](../inworld-unit-frames-reference.md) — PlayerFrame/TargetFrame geometry
- [wow-ui-sim-layout-spec-2026-03-31.md](../wow-ui-sim-layout-spec-2026-03-31.md) — exact pixel geometry for frames and tabs
- [nameplate-research-2026-03-27.md](../nameplate-research-2026-03-27.md) — nameplate design research
- [keybindings-scope.md](../keybindings-scope.md) — bindable vs fixed inputs

## See Also

- [[networking]] — login auth flow feeds into UI state transitions
- [[rendering-pipeline]] — UI renders on top of 3D scene
