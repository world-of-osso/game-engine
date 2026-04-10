# UI & Addon System

The engine UI is built on Dioxus (Rust, no FFI boundary), rendering to Bevy via a custom host renderer. Addons are WASM binaries loaded by `wasmtime`, communicating with the engine through an explicit `game-api` crate boundary.

## UI Framework: Dioxus → Bevy

Dioxus provides React-style component authoring in pure Rust — hooks, signals, diffing — with no JS runtime and no serialization overhead. The custom renderer maps Dioxus elements to engine primitives:

- **Anchor-based layout**: `SetPoint` with 9 anchor points, relative-to any named frame
- **Frame strata**: 9 levels (WORLD → TOOLTIP) for z-ordering; 5 draw layers within each frame
- **Scaling and alpha**: effective values are the product of all ancestor scales/alphas
- **Visibility**: `Show`/`Hide` propagates through ancestors

Reused from `wow-ui-sim`: anchor constraint solver, quad batcher, strata/draw-layer sort, nine-slice, tiling, atlas resolution, BLP loading, font metrics. Replaced: mlua/Lua → Dioxus; XML templates → RSX; FrameHandle userdata → Bevy ECS entities.

## Widget Types

19 widget types covering the full WoW UI surface: Frame, Button, CheckButton, Texture, FontString, Line, EditBox, ScrollFrame, Slider, StatusBar, Cooldown, Model/PlayerModel/ModelScene, ColorSelect, MessageFrame, SimpleHTML, GameTooltip, Minimap.

## Addon System: WASM Sandboxing

Addons compile to `.wasm` (primary: Rust via `wasm32-wasip1`; secondary: JS via javy). The engine loads them with `wasmtime`:

- **Memory isolation**: addon can only touch its own linear memory
- **No filesystem/network** unless explicitly granted
- **CPU fuel metering**: prevents infinite loops from freezing the game
- **Per-addon memory caps**

The `game-api` crate is a standalone dependency so addon authors never pull in the engine. It declares extern host functions matching `linker.func_wrap` registrations on the engine side.

## Hot Reload

Dev loop: edit → `cargo build` (~1–5s) → engine `notify` watcher detects `.wasm` change → drops old instance, loads new, calls `on_load()` (~1ms reload).

Dev-mode shortcut: embed QuickJS and load raw `.js` files directly — no compile step, same API, same sandbox rules. Ship `.wasm` for distribution.

## Not Replicated from wow-ui-sim

XML template/TOC loading, Lua 5.1 environment, taint/security system, forbidden frame proxies, and WoW-specific `C_*` APIs are intentionally excluded — WASM sandboxing replaces all isolation needs.

## Sources

- [ui-addon-architecture.md](../../ui-addon-architecture.md) — full widget list, script handlers, rendering details, wow-ui-sim reuse inventory

## See Also

- [[nameplate-design]] — nameplates are in-world UI rendered through this system
- [[open-source-wow-clients]] — wow-ui-sim is the direct reference implementation
