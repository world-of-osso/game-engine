# UI & Addon Architecture Research

Discussion from 2026-02-28. Evaluating UI framework and addon plugin approaches for game-engine.

## Context

wow-ui-sim uses WoW's XML/Lua model (mlua + XML parser + anchor constraint solver + GPU quad batcher). The question: what should game-engine's own UI system look like, and how should addons work?

## Option 1: Custom React Renderer with Anchor Layout

React's reconciler is renderer-agnostic. Build a custom host config where:
- Components resolve to positioned rects on a GPU quad batcher
- Layout uses anchor constraints instead of flexbox
- Props map to WoW concepts: `SetPoint`, strata, draw layers

```jsx
<Frame name="HealthBar" strata="MEDIUM">
  <Anchor point="BOTTOM" relativeTo="Nameplate" relativePoint="TOP" offsetY={-10} />
  <Texture drawLayer="BACKGROUND" color={[0.2, 0.2, 0.2, 0.8]} />
  <Texture drawLayer="ARTWORK" width={healthPct + "%"} color={[0, 1, 0, 1]} />
  <FontString drawLayer="OVERLAY" text={`${hp}/${maxHp}`} />
</Frame>
```

Reconciler calls: `createInstance` (allocate rect), `appendChild` (parent-child), `commitUpdate` (prop diff triggers anchor re-resolution + quad rebuild). No DOM, no flexbox, no HTML.

**Tension:** React wants unidirectional data flow (parent -> child), but anchors are relational (frame A anchored to frame B, not necessarily a parent). Needs a shared layout registry outside the React tree.

Requires embedding a JS runtime (QuickJS ~300KB, V8 ~20MB).

## Option 2: Dioxus Rendering to Bevy (Chosen Direction)

Zero language boundary, zero FFI overhead, one toolchain. Dioxus gives the React model in pure Rust:

```rust
#[component]
fn HealthBar(hp: u32, max_hp: u32) -> Element {
    let pct = hp as f32 / max_hp as f32;
    rsx! {
        Frame { strata: Strata::Medium,
            Anchor { point: TOP, relative_to: "Nameplate", offset_y: -10 }
            Texture { draw_layer: Background, color: Color::rgba(0.2, 0.2, 0.2, 0.8) }
            Texture { draw_layer: Artwork, width: pct, color: Color::GREEN }
            Label { draw_layer: Overlay, "{hp}/{max_hp}" }
        }
    }
}
```

Benefits:
- Hooks (`use_signal`, `use_effect`) with no runtime cost
- Diffing/reconciliation -- only changed quads rebuild
- No serialization across language boundaries
- Compile-time type checks on entire UI

## Addon Plugin System: WASM

Addons compile to `.wasm`, engine loads them via `wasmtime`. The engine defines the API boundary -- what functions plugins can call, what data they receive.

### Host Side (Engine)

```rust
linker.func_wrap("env", "create_frame", |name: i32, len: i32| -> u64 {
    // read string from wasm memory, create frame in ECS, return ID
})?;
linker.func_wrap("env", "set_text", |frame_id: u64, text: i32, len: i32| {
    // update frame text in Bevy
})?;
```

### Plugin Side (Addon)

The `game-api` crate provides types and extern declarations matching host functions:

```rust
use game_api::{CreateFrame, Frame};

#[no_mangle]
pub fn on_load() {
    let bar = CreateFrame::new("HealthBar");
    bar.set_text("100/100");
    bar.on_event(Event::UNIT_HEALTH, |frame, unit| {
        frame.set_text(&format!("{}/{}", unit.hp, unit.max_hp));
    });
}
```

Compiles with `cargo build --target wasm32-wasip1`, ships as `.wasm`.

### Safety

- Sandboxed memory -- can't touch anything outside its linear memory
- No filesystem/network access unless explicitly granted
- CPU fuel metering -- no infinite loops freezing the game
- Memory caps per addon

### Language Support

| Tool | Source | Approach | Output size |
|------|--------|----------|-------------|
| wasm-bindgen | Rust | Direct to wasm | ~10KB |
| javy | JavaScript | Embeds QuickJS in wasm | ~300KB |
| AssemblyScript | TypeScript-like | Compiles to wasm natively | ~10KB |

Rust as primary, JS via javy as secondary option for addon authors.

The addon API lives in a separate `game-api` crate (`../game-api/`) so addon authors can depend on it without pulling in the engine.

### Development Workflow

WASM dev loop: edit -> compile (~1-5s) -> engine hot-reloads `.wasm` (~1ms).

Engine watches addon directory and reloads on file change:

```rust
notify::recommended_watcher(|event| {
    if event.path.extension() == "wasm" {
        reload_plugin(event.path); // drop old instance, load new, call on_load()
    }
});
```

**Dev mode optimization:** embed QuickJS directly for development. Load raw `.js` files with file watching -- no compile step. Same API, same sandbox rules, just interpreted. Ship `.wasm` for distribution.

## Feature Parity with wow-ui-sim

The UI system must support the same functional capabilities as wow-ui-sim. We're replicating the feature set, not the implementation quirks (broken load orders, algorithmic edge cases). Clean reimplementation of the same UI primitives.

### Widget Types (19)

All widget types from wow-ui-sim must have equivalents:

- **Frame** -- base container, parent-child hierarchy
- **Button** -- normal/pushed/highlight/disabled states, atlas-based textures, three-slice caps, click handlers
- **CheckButton** -- checked/unchecked states, OnValueChanged
- **Texture** -- image rendering (BLP/PNG), solid color, atlas, tex coords, tiling, blend modes, vertex color, desaturation, rotation
- **FontString** -- text rendering with font selection, color, justification (H/V), shadow, outline, word wrap, max lines, text scale
- **Line** -- vector line with thickness, start/end anchor points
- **EditBox** -- text input with cursor, selection, multi-line, numeric-only, password mask, history, max letters/bytes
- **ScrollFrame** -- scrollable container with horizontal/vertical scroll, scroll range
- **Slider** -- horizontal/vertical value slider with thumb texture, step snapping
- **StatusBar** -- progress bar with fill style (standard/center/reverse), orientation, color
- **Cooldown** -- timer overlay with swipe, edge, bling, countdown text
- **Model/PlayerModel/ModelScene** -- 3D model display (already in game-engine via M2 pipeline)
- **ColorSelect** -- color picker
- **MessageFrame** -- scrolling message display with fade/hold
- **SimpleHTML** -- basic formatted text
- **GameTooltip** -- tooltip with lines, double lines, item/unit/spell tooltips
- **Minimap** -- minimap frame

### Layout System

- **Anchor-based positioning**: SetPoint with 9 anchor points (TOPLEFT..BOTTOMRIGHT), relative-to any named frame
- **Sizing**: explicit width/height, auto-sizing for text
- **Scaling**: per-frame scale factor, effective scale = product of ancestor scales
- **Alpha**: per-frame opacity, effective alpha = product of ancestor alphas
- **Frame strata**: 9 levels (WORLD through TOOLTIP) for major z-ordering
- **Frame level**: per-strata ordering with Raise/Lower/RaiseToTop/LowerToBottom
- **Draw layers**: 5 layers within each frame (BACKGROUND, BORDER, ARTWORK, OVERLAY, HIGHLIGHT) with sub-layers
- **Visibility**: Show/Hide with ancestor propagation (IsVisible vs IsShown)
- **Hit rect insets**: shrink clickable area independent of visual bounds
- **Clamped to screen**: prevent frames from going off-screen

### Rendering

- **Textures**: BLP and PNG format loading, FileDataID resolution
- **Solid color rects**: SetColorTexture
- **Tex coords**: 4-arg UVs, 8-arg quad UVs for repeat tiling
- **Tiling**: horizontal/vertical texture repeat
- **Atlases**: named atlas regions with automatic sub-region lookup
- **Nine-slice**: 9-patch panel rendering for resizable bordered frames
- **Three-slice**: horizontal cap textures for stretched buttons
- **Blend modes**: ALPHAKEY (default), ADDITIVE
- **Vertex color**: per-texture tinting
- **Desaturation**: grayscale filter with configurable strength
- **Rotation**: 2D texture rotation
- **Backdrop**: framed backgrounds with bgFile, edgeFile, tile, insets, colors
- **Masks**: mask textures for circular clipping and custom shapes
- **Font system**: WoW font table (FRIZQT__, MORPHEUS, etc.), glyph atlas, text metrics

### Input

- **Mouse**: click (left/right/middle), double-click, enter/leave (hover), mouse wheel, mouse down/up
- **Keyboard**: key down/up, char input, special keys (Enter, Escape, Tab, Space, Arrow)
- **Drag**: movable frames, resizable frames, drag start/stop, receive drag (drop target)
- **Focus**: keyboard focus management, auto-focus for EditBox
- **Keyboard propagation**: SetPropagateKeyboardInput for parent bubbling
- **Motion scripts while disabled**: optional hover events on disabled buttons

### Script Handlers (55)

Full handler set including:
- **Lifecycle**: OnLoad, OnShow/OnHide, OnUpdate, OnSizeChanged
- **Mouse**: OnClick, PreClick, PostClick, OnDoubleClick, OnEnter, OnLeave, OnMouseDown/Up, OnMouseWheel
- **Drag**: OnDragStart, OnDragStop, OnReceiveDrag
- **Keyboard**: OnKeyDown, OnKeyUp, OnChar, OnEnterPressed, OnEscapePressed, OnTabPressed, OnSpacePressed
- **Value**: OnValueChanged (Slider/CheckButton/StatusBar), OnMinMaxChanged, OnTextChanged
- **Scroll**: OnVerticalScroll, OnHorizontalScroll, OnScrollRangeChanged
- **Focus**: OnEditFocusGained, OnEditFocusLost
- **Tooltip**: OnTooltipSetItem/Unit/Spell, OnTooltipCleared
- **Animation**: OnFinished, OnLoop, OnPlay, OnStop
- **Cooldown**: OnCooldownDone
- **Model**: OnModelLoaded, OnModelCleared
- **Attribute**: OnAttributeChanged

Script management: SetScript, GetScript, HookScript, HasScript

### Event System

- **Registration**: RegisterEvent, UnregisterEvent, RegisterAllEvents, RegisterUnitEvent
- **Dispatch**: OnEvent handler receives (self, event, ...) with event-specific args
- **Callback variant**: RegisterEventCallback for direct callback binding
- Events organized by category (player, unit, combat, chat, bags, UI, etc.)

### Animations (10 types)

- **Alpha** -- opacity fade
- **Translation** -- position shift
- **Scale** -- size change with origin point
- **Rotation** -- 2D rotation
- **VertexColor** -- color animation
- **FlipBook** -- texture atlas frame animation
- **TextureCoordTranslation** -- UV shift
- **LineTranslation/LineScale** -- line endpoint animation
- **Path** -- curved motion

Animation groups with: Play/Stop/Pause/Resume, looping (NONE/REPEAT/BOUNCE), easing (IN/OUT/IN_OUT), start delay, sequential ordering.

### Timers

- C_Timer.After(delay, callback) -- one-shot
- C_Timer.NewTimer(duration, callback, repeat) -- repeating
- Timer:Cancel / Timer:IsCancelled

### Data & State

- **Frame attributes**: SetAttribute/GetAttribute key-value storage with OnAttributeChanged
- **Saved variables**: per-addon persistent state (account + character level)
- **CVars**: GetCVar/SetCVar for engine configuration

### NOT Replicating

These wow-ui-sim features exist only for WoW addon compatibility and aren't needed:

- XML template parser and `inherits` chain (replaced by Dioxus components)
- Mixin system (replaced by Rust composition)
- TOC file loading (replaced by WASM plugin manifest)
- Lua 5.1 environment and global aliases (replaced by Rust/WASM)
- Taint/security system (replaced by WASM sandboxing)
- Forbidden frame proxies (not needed with WASM isolation)
- WoW-specific C_* APIs (C_AddOns, C_Container, etc.) -- game-api defines its own API surface
- Blizzard base library loading order quirks

## Reusable from wow-ui-sim

- Anchor constraint solver (core algorithm)
- Quad batcher / GPU atlas pipeline (adapted for Bevy's renderer)
- Strata/draw layer sorting logic
- Nine-slice, tiling, texture atlas resolution
- BLP texture loading (already shared via image-blp)
- Font metrics and text layout

## Replaced from wow-ui-sim

- mlua/Lua 5.1 -> Dioxus (engine UI) + wasmtime (addons)
- XML parser + template registry -> Dioxus RSX
- Mixin system -> Rust components/traits
- FrameHandle userdata -> Bevy ECS entities
- Event string dispatch -> typed Rust events
