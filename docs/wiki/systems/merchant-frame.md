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

## Gotchas

- `find_frame_at` needs `mouse_enabled`: item cells, bag slots and buttons set it, otherwise clicks fall through to the world.
- rsx `name:` takes a `FrameName`-like value; for a `&str` const use `{DynName(CONST.into())}`.
- `ui.rightClick(name)` (JS automation) is what drives buying and selling in headless proofs. The headless cage can't warp the OS cursor, and winit logs "could not set cursor position", but the Bevy window cursor is set, so the clicks land.
