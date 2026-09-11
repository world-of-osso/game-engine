# UI & Addon Architecture Research

Discussion from 2026-02-28. This document describes the game-engine UI and addon direction. The addon system is a game-specific JavaScript system; it is not intended to reproduce WoW's addon model.

## Context

The engine needs a native UI system with reusable widgets, game input, hot reload, and a scripting surface that addon authors can use without coupling scripts to Bevy ECS internals. The layout contract is the [registry-backed Bevy UI spec](specs/registry-bevy-ui.md).

Addons are JavaScript files loaded by the engine's embedded JavaScript runtime. They create and modify UI through the engine's addon API. There is no Lua runtime, XML/TOC loader, or WoW API compatibility target.

## UI Rendering Direction

The UI renderer owns layout, drawing, input, and widget behavior. The current UI stack uses Bevy rendering with the project's UI toolkit and frame registry. The renderer must support:

- Parent or screen-root layout placement
- Native insets, margins, translation, explicit/auto/fill sizing, and flex layout
- Logical parent-child hierarchy independent of the selected layout parent
- Visibility, opacity, z-order, and draw layers
- Text, textures, colors, borders, and nine-slice images
- Three-slice controls such as horizontally stretched buttons
- Mouse, keyboard, focus, hover, click, drag, and scroll interaction
- Reusable widgets with game-specific behavior

The JavaScript addon layer should submit operations to this UI system rather than directly manipulate Bevy entities.

## Addon System: JavaScript

The addon runtime loads `.js` files from the `addons/` directory using the embedded JavaScript runtime. Scripts execute against a narrow `addon` API. API calls are recorded as operations and applied to the UI frame registry, keeping JavaScript independent from renderer internals.

Current API surface includes:

```javascript
const panel = addon.createFrame("Panel");
addon.setSize("Panel", 320, 180);
addon.setPosType("Panel", "absolute");
addon.setAnchor("Panel", "screen");
addon.setPos("Panel", 24, 24);
addon.setBackgroundColor("Panel", 0.08, 0.08, 0.1, 0.95);
addon.show("Panel");

addon.createFontString("Title", "Panel", "Game Menu");
addon.setFontColor("Title", 1.0, 0.85, 0.4, 1.0);
```

Supported operations currently include:

- `createFrame(name, parent)`
- `createFontString(name, parent, text)`
- `setSize(name, width, height)`
- `setPos(name, x, y)`
- `setPosType(name, "relative" | "absolute")`
- `setAnchor(name, "parent" | "screen")` (`parent` is the default)
- `setText(name, text)`
- `show(name)` and `hide(name)`
- `setAlpha(name, alpha)`
- `setBackgroundColor(name, r, g, b, a)`
- `setFontColor(name, r, g, b, a)`

The API is intentionally game-specific. It may grow to cover widgets, input handlers, events, timers, and persistent addon state, but those are separate API additions rather than compatibility obligations.

### Runtime and lifecycle

- The engine watches `addons/` for JavaScript changes.
- Reloading removes frames owned by the addon before applying the new script.
- Addons run only while UI processing is enabled.
- JavaScript receives no direct access to Bevy ECS or renderer resources.

The development loop is: edit `.js` -> save -> engine reloads the addon -> inspect the updated UI.

## UI Capabilities

The UI system should provide the primitives needed by the game and its addons:

### Layout

See the [registry-backed Bevy UI spec](specs/registry-bevy-ui.md) for the API contract. Addons use parent-relative or absolute placement with `parent` or `screen` layout parents; arbitrary cross-frame anchors and `setPoint` are not supported. Logical ownership, visibility, alpha, and unload/removal behavior stay registry-controlled when layout uses `screen`.

- Native insets, margins, translation, size, and flex properties
- Per-frame scale and opacity
- Visibility propagation
- Z-order and draw-layer ordering
- Screen clamping where a widget requires it

### Rendering

- Text and font styling
- Texture and solid-color rendering
- UV coordinates and tiling
- Texture atlas regions
- Nine-slice panels
- Three-slice horizontal controls
- Borders, backgrounds, tinting, and alpha
- Optional masks and clipping

### Input

- Mouse movement, hover, click, press, release, and wheel input
- Keyboard input and text input
- Focus management
- Drag and drop
- Widget-specific interaction states

### Widgets

Reusable widgets may include frames, labels, buttons, checkboxes, text input, scroll containers, sliders, progress bars, tooltips, model views, and other game-specific controls. Each widget should expose a small JavaScript-facing API only when addon use requires it.

### Events and scripts

The addon API can expose game events and UI callbacks through explicit registration methods. Event names, payloads, callback lifetime, and error handling are part of this project's API contract; they do not need to match another game's event model.

## Non-goals

The addon system does not aim to provide:

- Lua execution or Lua compatibility
- XML layout files or TOC manifests
- WoW frame, widget, `C_*`, or saved-variable compatibility
- WoW load-order, taint, mixin, or global-environment behavior

If sandboxing becomes necessary, it should protect the JavaScript runtime without changing the JavaScript authoring model.

## Design Principles

- Keep addon scripts independent of Bevy ECS implementation details.
- Expose a small, explicit API instead of mirroring an external game's API.
- Keep layout and rendering in the native UI layer.
- Make addon ownership and reload cleanup deterministic.
- Add API surface only for concrete game or addon requirements.
