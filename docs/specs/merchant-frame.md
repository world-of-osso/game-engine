# Merchant Frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

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
- [x] `BuybackList.items` arrives in sale order, oldest first and newest last, including after the 12-slot list wraps. The list displays that order; every buyback request uses the item's real `slot`, not its list index. The main-tab last-sale button uses the last entry.
- [x] Buyback tab: title "Merchant Buyback" (`MERCHANT_BUYBACK`), 12 cells with rows every 59 px (MF.lua:516-519). Paging, repair, buyback slot and bottom border are hidden.
- [x] Clicks (MF.lua:632-669): right-clicking an item sends `BuyItem { count: 1 }`; any click on a buyback cell sends `BuybackItemRequest`. Left-click pickup, drop-to-buy and drop-to-sell: [cursor-item](cursor-item.md).
- [x] Right-clicking a bag item while the merchant tab is shown sells the stack (`SellItem { count: 0 }`, `ContainerFrameItemButton_OnClick`). Nothing happens on the buyback tab.
- [x] `MerchantFailed` shows its Retail text in `UIErrorsFrame`, e.g. "You don't have enough money." or "The merchant doesn't want that item."
- [x] Hovering an item cell shows the full GameTooltip item tooltip (`MerchantItemButton_OnEnter`, MF.lua:710-724), the same as a bag item: the server's name in its quality colour, then the catalog lines (item level, binding, slot and type, damage/speed/DPS, armor, stats, requirements, flavor text), `DURABILITY_TEMPLATE` at the new item's `VendorItem.max_durability`, the sell price of the purchase and the item ID. `SetMerchantItem` on the merchant tab, `SetBuybackItem` (the sold stack) on the buyback tab and the last-sale slot. Shift compares it with the equipped items it replaces (`GameTooltip_ShowCompareItem`, MF.lua:714). This replaces the earlier name/stack/stock-only cell tooltip (user decision, 2026-10-01).
- [x] Bags: `InventorySnapshot` and `InventoryDelta` fill the backpack (item guid, id, count, icon from `ItemModifiedAppearance`/`ItemAppearance`). Bag slots draw their icon and count.
- [x] Both presets show the backpack search clear X only with nonempty text. Its 17×17 button draws the 10×10 `common-search-clearbutton` at half alpha; clicking clears text and focus, restoring unfiltered items (`Blizzard_SharedXML/Shared/InputBox/InputBoxTemplates.xml:221-244`, `InputBoxTemplates.lua:192-210,233-235`). Text-only visibility is this task's contract; Retail also shows the X while focused-empty.
- [x] Left-click pickup to the cursor, buy by dropping on a bag slot (`BuyItem.destination`), sell by dropping on the frame, Shift-click quantity (`StackSplitFrame`): [cursor-item](cursor-item.md).
- [x] `MerchantSellAllJunkButton` (36×36, `SpellIcon-256x256-SellJunk`): RIGHT at RepairAll LEFT +80 at a repairer, else BOTTOMRIGHT −148,33 (MF.lua:933-954); enabled (not desaturated) while a poor bag item has a sell price (`GetNumJunkItems`, MF.lua:196-198); hidden on the buyback tab. Native direct-action behavior is specified below; Retail's confirmation (MF.lua:1054-1063) is not the native client contract.
- [x] Bag item names/quality and their full item tooltips come from the client item catalog (ItemSparse): [cursor-item](cursor-item.md).
- [x] `MerchantRepairItemButton` (36×36, `SpellIcon-256x256-Repair`, RIGHT at RepairAll LEFT −8, MF.lua:948-960) shows with Repair All and is always enabled. A click toggles the repair cursor (`ShowRepairCursor`/`HideRepairCursor`, MF.xml:305-313); while it is shown the button's `ButtonHilight-Square` stays lit (additive, MF.lua:136-142) and the cursor is `Interface/CURSOR/Crosshair/Repair`.
  - A left click on an item (a CharacterFrame paperdoll slot or a bag slot) with the repair cursor sends `RepairItem { item_guid: Some(guid) }` instead of picking it up. The cursor stays until the button is clicked again or the frame hides (`ResetCursor`, MF.lua:167).
  - The server repairs a damaged owned bag or equipped item by its guid for that item's own cost; undamaged or foreign items are refused (game-server `docs/specs/merchant.md`).
- [x] Button tooltips (`ANCHOR_RIGHT`): Sell All Junk "Sell All Junk Items" (MF.xml:207-211); Repair An Item "Repair an Item" (MF.xml:300-303); Repair All, when something is damaged, "Repair All Items", the cost as a money line and, when it exceeds the player's money, "Insufficient funds to repair all items" in red (MF.xml:240-252). The last-sale slot shows its buyback item (`MerchantBuyBackButton_OnEnter`, MF.lua:1078-1082).
- [x] Sounds: `IG_CHARACTER_INFO_OPEN`/`_CLOSE` when the frame shows/hides (MF.lua:158, 174), `IG_MAINMENU_OPTION_CHECKBOX_ON` from the page buttons (MF.lua:574, 581), `ITEM_REPAIR` from Repair All (MF.xml:256). Files are the kits' `SoundKitEntry` Oggs under `data/sounds/ui/`.
- [x] The mouse wheel over the frame pages back (up) or forward (down) when that page button is shown and enabled, and never reaches the camera (`MerchantFrame_OnMouseWheel`, MF.lua:177-187).
- [x] Over a vendor item on the merchant tab the cursor is Buy, or UnableBuy (`BUY_ERROR_CURSOR`) when the player can't afford it (MF.lua:126-134).
- [x] Guild bank repair (`MerchantGuildBankRepairButton`, MF.xml:318-397), native client: shown at a repairer when `VendorInventory.guild_repair_money` is `Some` (`CanGuildBankRepair`). Then RepairAll sits BOTTOMRIGHT at 96,33, Repair An Item RIGHT at its LEFT −9, Sell All Junk RIGHT at its LEFT +128 and the guild button (icon `SpellIcon-256x256-RepairAllGuild`, UiTextureAtlasMember 23481) LEFT at its RIGHT +8 (MF.lua:936-946). It is enabled and desaturated with Repair All (`GetRepairAllCost`). A click sends `RepairItem { item_guid: None, guild_bank: true }` and plays `ITEM_REPAIR` (MF.xml:368-375). Tooltip (MF.xml:332-364): "Repair All Items", the cost, "Remaining amount for today's Guild Bank repairs:" (`GUILDBANK_REPAIR`, wrapped) over the guild money left; when the cost exceeds it, "Personal amount to be spent:" (`GUILDBANK_REPAIR_PERSONAL`) and the difference, or "Insufficient funds to repair all items" in red. The server pays only from the guild bank (TrinityCore `DurabilityRepairAll(guildBank = true)` never takes the personal remainder; game-server `docs/specs/merchant.md`). The Bevy client never shows the button.
- [x] NPC portrait in the portrait ring (`MerchantFrame:SetPortraitToUnit("npc")`, MF.lua:269): `MerchantFramePortrait`, 62×62 at TOPLEFT (-5, 7) under `TempPortraitAlphaMask` (SharedUIPanelTemplates.xml:551-572), renders the open vendor's model from its M2 portrait camera through the unit-frame portrait path (`unit_portraits.rs`); it clears when the frame closes.
- [x] Buyback replaces the NPC portrait with `Interface\MerchantFrame\UI-BuyBack-Icon` (MF.lua:508); returning to Merchant restores the vendor portrait, in both presets.
- [ ] The filter dropdown, alternate currencies (`ExtendedCost` items are not sold), and the refund confirmation popup.

## Godot client

- Shared metal chrome resolves Blizzard atlas element names under the active skin when composing the panel sheet: Modern keeps the original Retail crops; Forever uses set-1 `uiframemetal2xc60` sheets. Window geometry and bottom-edge tiling stay unchanged. Missing sheet assets report errors; no substitute art. `forever_small_windows` covers exact member rects, composition and base-Modern merchant/mail tree parity; live switching is not proven.
- [x] Right-click on a unit targets it; a living NPC within 5 yd gets `InteractNpc`. Hovering an NPC shows its Retail cursor (Buy for a vendor, from replicated `NpcFlags` and the reaction).
- [x] The frame, backpack and StackSplitFrame are the shared components above, driven by the same server messages; right-click buy and sell, buyback (tab and last-sale slot), Repair All, paging, tabs, and vendor Shift-click split with its keys.
- [x] Escape, the close button and `InteractionClosed` close the frame; the first two send `CloseInteraction`.
- [x] Repair All and Sell All Junk execute directly, without a confirmation dialog. An eligible Merchant-tab junk click sends `SellAllJunkItems` immediately; common-only inventory and the Buyback tab send none. Bags, money and repair cost remain server-owned and change only from authoritative updates.
- [x] Merchant cell (full item tooltip), service-button and last-sale tooltips; the repair cursor; sounds; wheel paging; Buy/UnableBuy cursors (`native_input_fixture merchant-tooltips`, live `godot/tests/world_merchant_live_flow.gd`).
- [ ] Cursor item (pickup, drag-buy, drop-sell, bag split) full acceptance, the gossip frame, Retail ContainerFrame art.

Native cursor coverage is partial, not wholly unconverted: [current bounded Buy + whole/split sale acceptance](../wiki/systems/godot-conversion.md#native-merchant-split-cursor-sale--main-accepted-bounded-pass). The unchecked combined requirement above is not full native acceptance.

`merchant-cursor` is an independent owned native fixture mode, not a user game screen. MAIN-accepted independent gate1446 bounded PASS at `837e2c1e`/`b073e4dd`: scoped functional/source/format/readability and matching build/five-flow evidence accepted. First vendor → own embedded bag only. Right-click/Shift/buyback model paths remain retained, not full runtime proof. [Oracle, source coverage and proof boundary](../wiki/systems/godot-conversion.md#native-merchant-cursor-buy--main-accepted-bounded-pass); [invocation](../remote-builds.md#native-merchant-cursor-fixture). Existing merchant-click open/close, placement and audio gates remain unchanged.

## Bounded live acceptance (2026-10-07)

The merchantfix follow-up passes the requested merchant steps 1–8 on the private UDP 5328 server: priced stock/paging and a two-item Shift purchase; right-click sales; 13-sale wrap with newest Gloves restored from physical slot 0 and the oldest list item from slot 1; Repair All; a bag weapon repaired from 10/20 to 20/20 for its own 8 copper; all three refusal messages without inventory/money changes; Shift comparison with Item ID retained; Escape/range close with repair-cursor reset. Two sequential client launches, never two rendered clients. Full suites: server 1,499 passed / 56 ignored / one permitted latency test filtered; client core/ui-model/godot 2,173 passed / 6 ignored; zero failures.

[Proof ledger](../../data/diagnostics/merchantloop-2026-10-07/proof-ledger.txt), `merchantfix-client{1,2}.log`, captures/state dumps under `shots-merchantfix{1,2}/`. Client source proof is `43539f8a7`, server `26f3c5c`; later documentation-only commits do not change it. Full layout/coexistence, cursor-owner coverage and other variants remain partial, as above. Shared-protocol was not changed; its buyback slot-order comment needs a separately authorized documentation correction.

## Bounded private live acceptance (2026-10-09)

Verified client `709a624318be6f7c1394f8f5d2e2d937a186276a`, supplied server `5b04aa931720d441aaea1f99d3c11dc2a69bba40`, fresh account `fb_qml3merchant`, owned UDP5490. A rendered `world_merchant_live_flow.gd` derivative used real pointer input and keyboard C for the paperdoll; admin damaged starter equipment to 50% before login. All amounts below are copper.

| Requested step | Result | Inspected PNG / authority |
| --- | --- | --- |
| Buy, sell, buyback with item/money restoration | PASS | Vest buy 89: 9994→9905; sell 17: 9905→9922; buyback 17: 9922→9905. Exact purchased-name item count and post-buy money restored. `06-buyback-tab.png`, `07-buyback-restored.png`. |
| Repair one damaged item | PASS | MainHand guid114, total cost16→8, money10000→9992; repair cursor stayed active. `03-repair-cursor.png` shows selected repair mode before the item click. |
| Repair all remaining damaged items | PASS | Cost8→0, money9992→9984; repair sound observed. `04-repaired.png`. Two Ruined Pelts then sold directly for10, funding pre-buy money9994. |

PNG root: `/syncthing/AgentShared/2026-10-09/questmerchlive/`. Scripts/logs: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/questmerchlive3-2026-10-09/` (`merchant.gd`, `merchant-green.log`, `server.log`, `proof-ledger.txt`). Sources: MF.lua:662,667, MF.xml:255,311; pinned [TrinityCore buyback](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/ItemHandler.cpp#L486-L519) restores the stored item after deducting its sale price; [single/all repair](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/NPCHandler.cpp#L389-L419) selects repair by item GUID or all items.

The inherited fixture's CharacterMicroButton click failed because the micro menu is [hidden by default](hud-edit-mode.md); keyboard C completed repair. This is a fixture/input-path limitation, not a product defect. No product fix claimed. Exact item-GUID restoration, wrapped buyback, bag repair, other skins, full coexistence/pixel parity and normal shutdown were not re-proved in this run; earlier broader acceptance above remains separately scoped.

## Tests asserting this spec

- `godot/ui-model/src/ui/screens/merchant_frame_component_tests.rs`: frame size, title, grid offsets, coins, gray price, red tint, stock, paging, repair position/enable, money anchor, buyback tab layout, last-sale slot.
- `src/scenes/merchant_frame/tests.rs`: Tharynn Bouden's 19 items page 10/9, affordability, stock, repair enable, right-click → requests, window open/close → `CloseInteraction`, bags open and close.
- `godot/ui-model/src/game/merchant_data.rs` tests: paging and a stock refresh keeping the page.
- `src/game/networking/merchant_tests.rs`: list/buyback/failure inboxes, interaction end closes, requests go out only while a vendor is open.
- `godot/ui-model/src/game/bag_data_tests/inventory.rs`: snapshot then delta drive the backpack.
- `godot/ui-model/src/ui/screens/bag_frame_component.rs` tests: icon, count, click action.
- `src/scenes/bag_frame/mod.rs` tests: right-click sells only on the merchant tab.
- `godot/ui-model/src/ui/screens/merchant_frame_component_tests.rs`: Sell All Junk placement with and without repair, disabled icon, hidden on buyback, the frame's drop action; `src/scenes/merchant_frame/tests.rs`: junk detection (Ruined Pelt vs Linen Cloth), historical Bevy confirmation behavior. Native direct services: `godot/ui-model/tests/merchant_junk.rs` and `godot/tests/world_merchant_services_flow.gd` assert immediate requests without confirmation, no optimistic mutation, and authoritative results.
- `src/scenes/tooltip_frame/mod.rs` tests: the Bevy client's merchant cell tooltip (name, stack, stock).
- `godot/network/src/ui/js_automation.rs` test: `ui.rightClick`.
- Godot: `godot/ui-model/tests/merchant_repair.rs` (repair cursor toggle, guid requests, close reset, button placement, click sounds, guild repair button layout/request/sound), `godot/ui-model/src/game_tooltip/merchant.rs` tests (vendor weapon full tooltip at full durability, bundle sell price, buyback stack, Shift comparison, service and guild repair tooltips), `godot/ui-model/tests/native_guild_bank.rs` (repair money log line), `godot/core/src/ui_sound_kits.rs` tests, `godot/ui-model/tests/merchant.rs` (session over world.db vendors), `godot/network/src/wire_tests.rs` (`NpcFlags`, `Gold`, `VendorInventory` over UDP), `godot/rust/src/merchant.rs` tests (interact range, split keys), live `godot/tests/world_merchant_flow.gd` (`data/diagnostics/merchant-godot-20260928/`).
- Live guild repair and vendor weapon tooltip (native client, `godot/tests/merchant_guild_live.gd`, private server): `data/diagnostics/merchant3-20261001/live/` — the Guild Master deposits 10g at the Stormwind vault, Janos Hammerknuckle's Shortsword shows its full tooltip (item level, binding, One-Hand Sword, damage/speed/DPS, Durability 20 / 20, sell price) and Shift compares it with the equipped Worn Shortsword and shield; the guild button repairs 16c from the guild (10g → 9g 99s 84c, personal 90g unchanged, server `RepairItem { guild_bank: true }`); the Money Log reads "withdrew 16c for repairs". Fixture: `fixture-merchant-tooltips.log` there.
- Live evidence: `data/diagnostics/merchant-20260924/`; drag buy, split, drop-sell, Sell All Junk and bundle buys: `data/diagnostics/cursoritems-20260926/run1/` (tree/shot 03-28).
