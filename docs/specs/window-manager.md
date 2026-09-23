# In-world window manager

Implements the framework items "Window classes" and interaction rule 15 of the
[in-game UI plan](../plans/2026-09-23-ingame-ui.md). How the UI renders:
[UI system](../wiki/systems/ui-system.md).

## What it must do

### Open state and classes

- [x] One resource (`WindowManager`) owns every in-world window's open state. Scene frames read it for visibility; no per-scene open flag exists.
- [x] Panel: character, spellbook, professions, friends/social, guild, mail, loot rules, calendar, inspect, merchant. At most two; opening a third closes the oldest open panel.
- [x] NPC-driven panels (merchant, mail) take slot L and push the others right.
- [x] Wide: world map, talents, achievements, encounter journal. Opening one closes every panel and other wide window; bags stay.
- [x] Container: each bag. Coexists with everything.
- [x] Popup and fullscreen classes are unchanged (`PopupStack`, game menu).
- [x] Server sessions: a merchant or inspect session opening opens its window; the manager closing the window ends the session data (`MerchantState::close`, inspect snapshot reset); the session ending closes the window.
- [x] Escape step "close all panels and wide windows" closes every window through the manager in one press (bags included).
- [x] Leaving the world closes every window.

### Placement and stacking

- [x] Slot L top-left (16, 104); slot R 16 units right of slot L's window.
- [x] Wide: horizontally centered, top y = 104.
- [x] Bags: stacked upward from 96 units above the bottom-right corner, 16 from the right edge, 8 apart; a new column starts to the left when a bag would cross y = 104.
- [x] Every placed position is clamped to the registry screen size in UI units.
- [x] Clicking an open window raises it. The topmost window keeps its authored frame levels; each window below sinks its whole subtree by 32 levels, so popups authored above windows stay above all of them.

### Moved windows

- [x] Pressing inside a window's top 24 units (not on a button) and dragging moves it; the position is clamped to the registry screen size in UI units (UI scale applied).
- [x] Releasing saves the position for the current character (key: server character id) in `ui_layout.ron` next to `options_settings.ron` (`window_positions: {character: {root frame name: [x, y]}}`, RON).
- [x] A moved window opens at its saved position instead of its slot, for that character only; other characters keep the slot. Saved positions are re-clamped every frame.
- [x] Options → Interface → "Reset Window Positions" clears the current character's saved positions; windows return to their slots.

## Implementation inventory

- `src/window_manager/mod.rs` — `WindowId`, `WindowClass`, `WindowManager`, plugin.
- `src/window_manager/placement.rs` — `PostUpdate` placement before `UiRenderSet::Prepare` (screen rebuilds reset authored positions).
- `src/window_manager/input.rs` — click-to-raise, title-region drag.
- `src/ui_layout_store.rs` — `UiLayoutStore` (`ui_layout.ron`) load/save.
- `src/window_manager/sessions.rs` — merchant/inspect session reconciliation.

## Tests asserting this spec

- `tests/unit/window_manager_tests.rs` — class rules, slot/wide/bag placement, raise stacking, sessions, world exit, drag/save/reload per character, clamp at UI scale 4/3, reset.
- `tests/unit/game_menu_screen_tests.rs` — Escape order and one-press close-all.

## Known gaps

- The spellbook has no mounted in-world frame yet; `WindowId::Spellbook` (`SpellbookFrame`) is reserved for it.
- The world map component is authored 1920×1080, larger than the 1000×680 Wide maximum; placement clamps it to (0, 0) instead of resizing it.
