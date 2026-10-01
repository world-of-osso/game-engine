# In-world window manager

Implements the framework items "Window classes" and interaction rule 15 of the
[in-game UI plan](../plans/2026-09-23-ingame-ui.md). How the UI renders:
[UI system](../wiki/systems/ui-system.md).

## What it must do

### Open state and classes

- [x] One resource (`WindowManager`) owns every in-world window's open state. Scene frames read it for visibility; no per-scene open flag exists.
- [x] Panel: character, spellbook (`P`, root `SpellBookRoot`; the runtime shows what the manager has open), professions book (`K`), friends/social, guild, mail, loot rules, calendar, inspect, merchant, trainer. At most two; opening a third closes the oldest open panel.
- [x] NPC-driven panels (merchant, mail, trainer) take slot L and push the others right.
- [x] Wide: world map, talents, achievements, encounter journal, the 942-wide ProfessionsFrame. Opening one closes every panel and other wide window; bags stay.
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

## Native Godot boundary

- Native `WorldMapFrame` (Wide), `SpellBookRoot` (Panel), and NPC-driven `MerchantFrame` (Panel) use authored root names as canonical `ui_layout.ron` keys. Their title regions support drag, logical-viewport clamping, selected-server-character persistence and reopen. Spellbook and merchant default to (16, 104); merchant moves only its 336×444 root, not the backpack or split frame. Canvas navigation/zoom remains map input, not window dragging.
- Options → Interface → Reset Window Positions removes only the authenticated, selected server character ID's `window_positions` entry. Other characters' positions and account-wide edit-mode layouts remain; Options' own `modal_offset`/`modal_position` in `options_settings.ron` are independent. The owned fixture at `ac44cc2e` (`/tmp/claude/world-map-owned-fixture-ac44cc2e.log`) proves ID 17 removal, ID 18/edit-layout/modal retention, map reopen at the default slot, and a second-process read.
- The direct open-map reset behavior has a state test. The extended owned `reset-windows` fixture passes spellbook title/button/body separation, nonunit-scale drag, save/reopen/fresh-process restore, resize clamp, and reset (code `c66bdfef`, script `e0d02e78`; `data/diagnostics/spellbook-placement-fixture-e0d02e78.log`). Simultaneous Options-plus-window UI is not keyboard-reachable because Escape closes it. The owned merchant-click fixture at script `9f64af5f` and Depot-built native source `9576f32b` exits 0 for placement, scale/clamp, persistence, reset, bag separation, Buyback tab state and pointer-audio behavior (`data/diagnostics/merchant-placement-green-9f64af5f.log`). Native merchant coexistence, two-panel left/right stacking and NPC slot reassignment remain unimplemented; raise-on-press is native for toplevel windows (below), without the Bevy 32-level subtree sink. No generic native window-manager parity is claimed.

### Native pointer ownership, raise, bags and Escape

- [x] Each native window is its own CanvasLayer; Godot draws and picks canvases by `layer`, then sibling index. A drag release resolves its drop target with one hit-test over every RegistryUi in that order (`GameClient::ui_hit_at`, `godot/rust/src/window_stack.rs`), so the topmost frame under the pointer wins regardless of which canvas started the drag. Presses are already routed by Godot's GUI picking over the same order. Merchant and mailbox title drags start only when their canvas owns the topmost frame at the press.
- [x] Retail toplevel windows (`ContainerFrameContainer` holding every bag, ContainerFrame.xml:216; MerchantFrame.xml:91; MailFrame.xml:274; Blizzard_AuctionHouseFrame.xml:4) raise on press above their same-layer toplevel peers. Godot 4.7 does not re-sort GUI picking after a CanvasLayer `move_child`; the raise reapplies `set_layer` so picking matches drawing.
- [x] Native UI input carries one process-wide arrival stamp (`ui::input_queue`); cursor-slot input and raises from every window canvas are replayed in arrival order (`GameClient::poll_window_inputs`), so one-frame click sequences across canvases keep their order.
- [x] A mouse-button press reaches gameplay (camera drag, targeting, interaction) only from `_unhandled_input`, i.e. when no control consumed it; releases, motion and wheel are still recorded before the GUI. A drag that starts on a frame never orbits the camera.
- [x] Merchant, mailbox and auction house open every bag on show and close them on hide with Retail opener semantics (`OpenAllBags`/`CloseAllBags` and `FRAME_THAT_OPENED_BAGS`, ContainerFrame.lua:1903-1999; callers MerchantFrame.lua:147/165, MailFrame.lua:63/73, Blizzard_AuctionHouseFrame.lua:402/462, BankFrame.lua:74/81). Opening is a no-op while any bag is open; only the opener closes them. TradeFrame and GuildBankFrame do not call either. NPC windows register in `GameClient::npc_bag_windows` (`godot/rust/src/bags.rs`). Merchant and mailbox draw the backpack in their own canvas; bag placement keeps its stack slot.
- [x] Escape after the cursor/split steps runs one `CloseAllWindows` step (UIParentPanelManager.lua:1091-1106): bags, spellbook, world map, mailbox (with any open letter), auction house and merchant close in one press (`GameClient::close_all_windows`).
- Proof: `native_input_fixture ui-ownership` graded per case; unit tests `window_stack::tests`, `ui::input_queue::tests`, `input::tests`, `bags::tests`.

## Known gaps

- The world map component is authored 1920×1080, larger than the 1000×680 Wide maximum; placement clamps it to (0, 0) instead of resizing it.
