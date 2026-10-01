# Cursor Item

How the client picks items up onto the cursor, drops them, splits stacks and shows item tooltips. The spec is [cursor-item](../../specs/cursor-item.md); the server owns every move (game-server `docs/specs/inventory.md`, `docs/specs/merchant.md`).

## Pieces

- `src/game/cursor_item.rs`: `CursorItem` (Empty / Inventory {from, count, split} / Merchant {slot, purchases}) and `click(target)`, the pure Retail pickup/drop rules. A drop returns a `CursorEffect`: an `InventoryRequest` (Swap / Split / Destroy / Equip), a `MerchantRequest` (Buy with `destination`, Sell) or a destroy confirmation. The client never moves items itself; it clears the cursor and waits for the server's `InventoryDelta`.
- `src/scenes/cursor_item/mod.rs`: left press (and a drag released elsewhere) → `cursor_target(action)` from the frame's `onclick` (`bag_slot:b:s`, `equipment_slot:n`, `merchant_item:i`, `merchant_frame`; no frame = world). Shift-click opens the split frame instead. `DELETE_ITEM` / `DELETE_GOOD_ITEM` popups resolve through `PopupResult`. `CursorItemIcon` is a registry texture moved to the pointer every frame (same pattern as `ActionDragIcon`).
- `src/game/stack_split.rs` + `src/scenes/cursor_item/stack_split_frame.rs` + `src/ui/screens/stack_split_frame_component.rs`: StackSplitFrame logic (StackSplitFrame.lua ported: steps, typing, backspace), its screen, and Okay → `CursorItem::split_from` or a vendor `Buy { count: split / stackCount }`.
- `src/game/item_catalog.rs`: `Item.csv` + `ItemSparse.csv` + `ItemSubClass.csv` (build 12.1.0.69933), parsed with `CsvTable` (RFC 4180; ItemSparse descriptions contain quoted newlines) and warmed on a thread at startup. `bag_data::stack_slot` takes name and quality from it.
- `src/scenes/tooltip_frame/item_tooltip.rs`: bag and paperdoll tooltips from the catalog; `TooltipLineState::money` draws `SELL_PRICE:` coins with `merchant_frame_component::money`.
- `src/ui/popup.rs` `PopupSpec::confirm_text`: an edit-box popup whose Accept waits for the typed word (`DELETE`).

## Native coverage

[Conversion evidence](godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass) owns exact saved logs and limits. Bounded equip observation: one EquipItem0/5, authoritative bag clear/MainHand25 guid9170005 count1 and quiet EquipDone; actions run still ends parent101 on missing StaticPopup1 at world-drop PoorNo. `2223d6d3` routes native world presses to original destroy effects through existing shared popup host/results and adds popup keyboard ownership. Original `destroy_popup`/keys now share one source through `bf1b1a73`; targeted pure10/10 passes warning-free. After retained native match-arm build failure, `a0b295b4` fixes error handling/shared keys; `6bc3cde0` updates read-only diagnostics. Depot `jv155pp2xb` exits0 with existing WMO warning only. Latest actual actions parent0 proves exactly one Equip/two Destroy, authoritative bags empty/MainHand retained, PoorNo quiet/cursor cleared, rare disabled Yes+Enter inert, Unicode32/scalar Backspace and lowercase DELETE acceptance. Earlier parent101 remains historical equip→world-drop RED, not latest status. Independent verifier 1377 functional/compile/format PASS, readability not clean; followup proof pending, no resolved actions gate, full goal, startup-equipment, mesh, drag, NPC/global owner or normal-shutdown acceptance.

## Gotchas

- Keyboard ownership: the split frame and edit-box popups read `KeyboardInput` directly and force `UiInputMode::Text`, so digits don't fire action-bar keybinds.
- `walk_up_for_onclick` stops at the first frame with an `onclick`, even an empty one (disabled buttons): `Hit` treats an empty action as "not a target".
- The paperdoll (`CharacterFramePlugin`) is not registered in the app (pre-existing), so paperdoll pickup/drop is unit-tested but not reachable live; right-click equip is.
- There is no bag keybind; open the backpack with `ui.click("MainMenuBarBackpackButton")` in headless proofs. `ui.shiftClick(name)` drives Shift-clicks.
- A fresh `MessageReader` in `run_system_once` rereads every unread message: tests that run a keyboard system repeatedly must clear `Messages<KeyboardInput>` between runs.

## Sources

- [cursor-item spec](../../specs/cursor-item.md)
- Live evidence: `data/diagnostics/cursoritems-20260926/` (run1 vendor buy/split/sell/junk/bundle, run3-equip right-click equip, Escape and ui.key, run4-tooltip/tt-22 vest tooltip)

## See Also

- [[merchant-frame]] — vendor frame the cursor buys from and sells to
- [[unit-tooltip]] — tooltip anchors and the ID line
