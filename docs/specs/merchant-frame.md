# Merchant Frame

The Retail `MerchantFrame` running against the live server vendor. The contract is shared-protocol `protocol/merchant_messages.rs`; server rules are in game-server `docs/specs/merchant.md`.

References:
- MF.xml = `Blizzard_UIPanels_Game/Mainline/MerchantFrame.xml`
- MF.lua = `MerchantFrame.lua`
- both under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`
- strings from GlobalStrings (build 12.1)

## What it must do

- [x] The server's `VendorInventory` (sent when the vendor role opens) opens the frame.
  - Title is the NPC name; tab 1 is selected.
  - It opens as the NPC-driven Panel in window slot L, together with the backpack (`OpenAllBags`, MF.lua OnShow).
- [x] Closing the frame sends `CloseInteraction` and closes the backpack (`CloseAllBags`, OnHide). Closing means the close button, Escape or eviction.
- [x] `InteractionClosed` for the vendor closes it without a request. The server sends that on walking out of range, NPC death or combat, or opening another NPC.
- [x] Frame: 336×444 `ButtonFrameTemplate` (MF.xml:91-92) with the shared `metal_frame` chrome and the `Inset` at 4,60 / −6,26 (UI-Background-Marble 374154).
- [x] Merchant tab: 10 `MerchantItemTemplate` cells (153×44, MF.xml:3-4), `MERCHANT_ITEMS_PER_PAGE = 10` (MF.lua:1).
  - Cell positions: Item1 at 11,69; the right column at +165 (TOPRIGHT +12); rows every 52 px (MF.xml:127-186).
  - Cell art:
    - `SlotTexture` UI-EmptySlot 130766, 64×64 at −13,−13
    - `NameFrame` UI-Merchant-LabelSlots 136423, 128×78
    - 37×37 item button with UI-Quickslot2 130841
  - Cell text:
    - `Name` 100×30 in the quality colour
    - price in coins (`coin-gold/silver/copper`, atlas 1667824)
    - `Count` when the stack is above 1
    - `Stock` `(%d)` (`MERCHANT_STOCK`) for limited items
- [x] An unaffordable price is **gray**, not red (`canAfford == false → "gray"`, MF.lua:316). Unusable items tint the slot and name frame 1,0,0 and the icon 0.9,0,0 (MF.lua:362-390).
- [x] Paging appears only with more than 10 items:
  - `MerchantPageText` "Page %s of %s" (`MERCHANT_PAGE_NUMBER`) at BOTTOM 0,86
  - Prev (130869/130867) and Next (130866/130864) 32×32 at BOTTOMLEFT 25,96 / 310,96, over UI-PageButton-Background 130822
  - Prev is disabled on page 1 and Next on the last page (MF.lua:432-450).
- [x] Repair All (36×36, BOTTOMRIGHT at BOTTOMLEFT 118,33, icon `SpellIcon-256x256-RepairAll`) shows only at a repairing vendor. It is disabled (desaturated) while nothing is damaged (`GetRepairAllCost`), from `DurabilityStateUpdate.total_repair_cost`. A click sends `RepairItem { item_guid: None }`.
- [x] `MerchantBuyBackItem` (115×37 at MerchantItem10 BOTTOMLEFT 30,−53) shows the last sale on a 0.65 button, or the dimmed `common-icon-undo` arrow. A click buys it back.
- [x] `UI-Merchant-BotFrame` (334×61 at BOTTOMLEFT 1,26, atlas 5222222) and the player's money at BOTTOMRIGHT −4,8 over the ThinGoldEdge money background (525911).
- [x] Tabs `MerchantFrameTab1` "Merchant" (CENTER at BOTTOMLEFT 50,−15) and Tab2 "Buyback" (Tab1 RIGHT −16):
  - `PanelTabButtonTemplate` art (atlas 4707839), active 42 / inactive 36 high
  - width = text + 20, at least both caps
- [x] Buyback tab: title "Merchant Buyback" (`MERCHANT_BUYBACK`), 12 cells with rows every 59 px (MF.lua:516-519). Paging, repair, buyback slot and bottom border are hidden.
- [x] Clicks (MF.lua:632-669): right-clicking an item sends `BuyItem { count: 1 }`; any click on a buyback cell sends `BuybackItemRequest`. Left-click pickup, drop-to-buy and drop-to-sell: [cursor-item](cursor-item.md).
- [x] Right-clicking a bag item while the merchant tab is shown sells the stack (`SellItem { count: 0 }`, `ContainerFrameItemButton_OnClick`). Nothing happens on the buyback tab.
- [x] `MerchantFailed` shows its Retail text in `UIErrorsFrame`, e.g. "You don't have enough money." or "The merchant doesn't want that item."
- [x] Hovering an item cell shows a tooltip: name in quality colour, stack count, stock.
- [x] Bags: `InventorySnapshot` and `InventoryDelta` fill the backpack (item guid, id, count, icon from `ItemModifiedAppearance`/`ItemAppearance`). Bag slots draw their icon and count.
- [x] Left-click pickup to the cursor, buy by dropping on a bag slot (`BuyItem.destination`), sell by dropping on the frame, Shift-click quantity (`StackSplitFrame`): [cursor-item](cursor-item.md).
- [x] `MerchantSellAllJunkButton` (36×36, `SpellIcon-256x256-SellJunk`): RIGHT at RepairAll LEFT +80 at a repairer, else BOTTOMRIGHT −148,33 (MF.lua:933-954); enabled (not desaturated) while a poor bag item has a sell price (`GetNumJunkItems`, MF.lua:196-198); hidden on the buyback tab. A click asks `SELL_ALL_JUNK_ITEMS_POPUP` (Yes / No) and Yes sends `SellAllJunkItems` (MF.lua:1054-1063).
- [x] Bag item names/quality and their full item tooltips come from the client item catalog (ItemSparse): [cursor-item](cursor-item.md).
- [ ] The per-item repair cursor (`MerchantRepairItemButton`), guild-bank repair, the Sell All Junk hover tooltip, and the stats/armor/damage lines of item tooltips.
- [ ] NPC portrait in the portrait ring, the filter dropdown, alternate currencies (`ExtendedCost` items are not sold), and the refund confirmation popup.

## Godot client

- [x] Right-click on a unit targets it; a living NPC within 5 yd gets `InteractNpc`. Hovering an NPC shows its Retail cursor (Buy for a vendor, from replicated `NpcFlags` and the reaction).
- [x] The frame, backpack and StackSplitFrame are the shared components above, driven by the same server messages; right-click buy and sell, buyback (tab and last-sale slot), Repair All, paging, tabs, and vendor Shift-click split with its keys.
- [x] Escape, the close button and `InteractionClosed` close the frame; the first two send `CloseInteraction`.
- [ ] Cursor item (pickup, drag-buy, drop-sell, bag split), tooltips, the Sell All Junk popup, the gossip frame, Retail ContainerFrame art.

`merchant-cursor` is an independent owned native fixture mode, not a user game screen. MAIN-observed bounded GREEN at `837e2c1e`/`b073e4dd`: matching Depot build and five full runtimes exit0; independent gate1446/readability pending, not accepted until MAIN confirms. First vendor → own embedded bag only. Right-click/Shift/buyback model paths remain retained, not full runtime proof. [Oracle, source coverage and proof boundary](../wiki/systems/godot-conversion.md#native-merchant-cursor-buy--main-observed-bounded-green-gates-pending); [invocation](../remote-builds.md#native-merchant-cursor-fixture). Existing merchant-click open/close, placement and audio gates remain unchanged.

## Tests asserting this spec

- `src/ui/screens/merchant_frame_component_tests.rs`: frame size, title, grid offsets, coins, gray price, red tint, stock, paging, repair position/enable, money anchor, buyback tab layout, last-sale slot.
- `src/scenes/merchant_frame/tests.rs`: Tharynn Bouden's 19 items page 10/9, affordability, stock, repair enable, right-click → requests, window open/close → `CloseInteraction`, bags open and close.
- `src/game/merchant_data.rs` tests: paging and a stock refresh keeping the page.
- `src/game/networking/merchant_tests.rs`: list/buyback/failure inboxes, interaction end closes, requests go out only while a vendor is open.
- `src/game/bag_data_tests/inventory.rs`: snapshot then delta drive the backpack.
- `src/ui/screens/bag_frame_component.rs` tests: icon, count, click action.
- `src/scenes/bag_frame/mod.rs` tests: right-click sells only on the merchant tab.
- `src/ui/screens/merchant_frame_component_tests.rs`: Sell All Junk placement with and without repair, disabled icon, hidden on buyback, the frame's drop action; `src/scenes/merchant_frame/tests.rs`: junk detection (Ruined Pelt vs Linen Cloth), the popup sends `SellAllJunk` only on Yes.
- `src/scenes/tooltip_frame/mod.rs` tests: merchant cell tooltip.
- `src/ui/js_automation.rs` test: `ui.rightClick`.
- Godot: `godot/ui-model/tests/merchant.rs` (session over world.db vendors), `godot/network/src/wire_tests.rs` (`NpcFlags`, `Gold`, `VendorInventory` over UDP), `godot/rust/src/merchant.rs` tests (interact range, split keys), live `godot/tests/world_merchant_flow.gd` (`data/diagnostics/merchant-godot-20260928/`).
- Live evidence: `data/diagnostics/merchant-20260924/`; drag buy, split, drop-sell, Sell All Junk and bundle buys: `data/diagnostics/cursoritems-20260926/run1/` (tree/shot 03-28).
