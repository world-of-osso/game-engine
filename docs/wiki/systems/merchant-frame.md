# Merchant Frame

This page covers the client vendor. The spec is [merchant-frame](../../specs/merchant-frame.md), and the server rules are in game-server `docs/specs/merchant.md`.

## Pieces

- `src/game/networking/merchant.rs` (`MerchantNetworkPlugin`):
  - `VendorInventory` → `MerchantState::apply_inventory`. A new NPC resets the tab and page; the same NPC (a stock refresh) keeps them.
  - `BuybackList` → `MerchantState.buyback`.
  - `MerchantFailed` → `UiErrors`.
  - `NpcFrameEvent::Closed` for the open vendor → close.
  - `MerchantRequest` messages go out on `MerchantChannel` with the open NPC's server bits.
- `src/game/networking/inventory.rs` (`InventoryNetworkPlugin`): `InventorySnapshot` / `InventoryDelta` → `InventoryState::apply_snapshot` / `apply_delta` (bag data in `src/game/bag_data.rs`).
  - Slots keep `item_guid` and `item_id`; the icon comes from `item_icons::item_icon_fdid`.
  - `item_icon_fdid` returns the appearance icon, or `Item.IconFileDataID` for items without an appearance.
  - Bags of size 0 are left out.
- `src/scenes/merchant_frame/`:
  - `build_state` builds the view model.
  - `sync_merchant_window` opens the window (plus `Bag(0)`) when a vendor list arrives. When the window is closed, it sends `NpcInteractionRequest::Close` and closes the bag.
  - `dispatch_action` handles clicks: right-click on an item buys; any click on a buyback cell buys back. Left clicks on vendor items belong to [[cursor-item]] (pickup, drop on a bag slot buys there).
  - Sell All Junk pushes `SELL_ALL_JUNK_ITEMS`; `sell_junk_on_confirm` sends `MerchantRequest::SellAllJunk` on Yes. `has_junk` = a poor bag item with a catalog sell price.
- `src/ui/screens/merchant_frame_component.rs`: the Retail layout, with every offset cited from MerchantFrame.xml/.lua. It reuses `quest_art::window_chrome` (metal_frame).
- Right-click on a bag slot (`bag_slot:<bag>:<slot>`, `scenes/bag_frame::sell_bag_item`) sends `MerchantRequest::Sell` only while the merchant tab is shown.
- `scenes/tooltip_frame::hovered_merchant_tooltip`: name in quality colour, stack count and stock for `MerchantItem<n>`.
- The frame root carries `onclick: merchant_frame` so a cursor item dropped on its background is sold (MF.xml `OnMouseUp` → `PickupMerchantItem(0)`).

## Godot

[Current bounded native Buy + whole/split sale acceptance](godot-conversion.md#native-merchant-split-cursor-sale--main-accepted-bounded-pass) owns acceptance status and exclusions; not full merchant/cursor parity.

- `godot/rust/src/merchant.rs`: right-click interact (targets, then `InteractNpc` within 5 yd), the hover cursor (`wow_cursor_data::npc_cursor`, `Input.set_custom_mouse_cursor` with the `Interface/CURSOR` BLPs), the `MerchantUI` RegistryUi, Escape before the target/game menu. The native `MerchantFrame` alone uses `ui_layout.ron` selected-character placement and 24-logical-unit title drag at effective UI scale; close excludes title capture, release saves, resize clamps, and Options reset clears cached placement. Its backpack and split frame remain separate. Native Panel L/R ordering, coexistence and raise parity remain open.
- `godot/ui-model/src/merchant.rs` (`MerchantSession`): the Bevy scene logic without ECS — frame/bag/split states, `click_frame` / `click_bag` / `split_key` → `MerchantEffect` (a request or `CloseInteraction`).
- The shared data files (`merchant_data`, `bag_data`, `stack_split`, `item_catalog`, `item_icons`, `wow_cursor_data`) compile in `godot/ui-model` with `--cfg godot_host` (its `build.rs`), which drops their Bevy `Resource`/`Message` derives; item tables read from `ui-model::paths::set_data_root`.
- `metal_frame` is composed by `panel_style_data::compose_metal_sheet` into a registry dynamic texture.
- The projection reports right-clicks and Shift-left-clicks on `onclick` frames as `UiInput::AltClick` (`RegistryUi::pop_alt_click`).
- `NpcFlags` and `Gold` are read from the host `Replica` ([[godot-replication]]); interaction/vendor/bag/durability messages arrive as `AccountEvent::Npc`.
- Live-server fixture: `godot/tests/world_merchant_flow.gd` turns with TurnRight (ArrowRight); D strafes and walks the character away. The separate owned `native_input_fixture merchant-click` test replicates a nearby vendor with `NpcFlags::VENDOR`, receives actual `InteractNpc` from a ray-picked right-click, then sends `InteractionOpened`, `InventorySnapshot`, and `VendorInventory`. Authored buyback-tab and close-button left-down reach the owned Effects player; right press, release, Escape, and reopen stay quiet. Its original exit-0 log is `/tmp/claude/merchant-click-f1609b4e-final.log`; verification at `616689b7` independently revalidated that runtime/source identity (`/tmp/claude/verify-merchant-click-final.md`). The extended fixture at script `9f64af5f` and Depot-built native source `9576f32b` passes merchant-only placement, bag separation, scale/clamp, reopen/reset, target-before-menu Escape order and pointer-audio assertions (`data/diagnostics/merchant-placement-green-9f64af5f.log`, exit 0). See [[sound]] for limits.

## Gotchas

- `find_frame_at` needs `mouse_enabled`: item cells, bag slots and buttons set it, otherwise clicks fall through to the world.
- rsx `name:` takes a `FrameName`-like value; for a `&str` const use `{DynName(CONST.into())}`.
- `ui.rightClick(name)` (JS automation) is what drives buying and selling in headless proofs. The headless cage can't warp the OS cursor, and winit logs "could not set cursor position", but the Bevy window cursor is set, so the clicks land.
