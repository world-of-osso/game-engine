# Bank Frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The Retail combined `BankFrame` (character bank + Warband bank) running against the live server banker. Contract: shared-protocol `protocol/bank_messages.rs`; server rules: game-server `docs/specs/banks.md`. How it works: [banks](../wiki/systems/banks.md).

References: BF.xml / BF.lua = `Blizzard_UIPanels_Game/Mainline/BankFrame.xml` / `.lua` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`; strings from GlobalStrings (build 12.1); atlases from UiTextureAtlas (12.1.0.69933).

## What it must do

### Opening and closing
- [x] The `NpcFrameEvent::Opened { role: Banker }` event, which comes from the gossip option "I would like to check my deposit box.", opens `BankState` on the character bank. The server's two `BankContents` fill it. Contents that arrive while no banker is open are dropped.
- [x] The bank opens as the Wide `WindowId::Bank` window, together with the backpack.
- [x] Closing the window sends `CloseInteraction` and closes the backpack.
- [x] `InteractionClosed` for the banker closes the frame.

### Layout
- [x] Frame 738×460 (BF.xml:674) with the `metal_frame` chrome and the `bank-frame-background` atlas (5782252) from 0,20 to 0,−30.
- [x] Slot grid: 98 slots, column-major in 7 rows (BF.lua:941-968).
  - The first slot is at 26,63; the next row is 10 px below; columns are 8 apart within a pair and 19 apart between pairs.
  - Slot backgrounds are `bags-item-slot64` (4701874) in the character bank. The Warband bank uses `warband-bank-slot`, grown −6,5 / 6,−7.
  - Stack counts show above 1.
- [x] Side tabs: 32×32 with the SpellBook-SkillLineTab border (136831). The first is at TOPRIGHT +2,−25, and each next one 17 px below the previous (BF.lua:907-924).
  - The selected tab shows a gold UI-Quickslot2 marker (see Known gaps).
  - The purchase tab (`bags-icon-addslots`) comes last while a tab can be bought.
- [x] Header: the tab name in a 300×20 box at TOP −36.
- [x] Bottom tabs `Bank` / `Warband Bank` hang at the frame's BOTTOMLEFT 22,2 (PanelTab art).
- [x] Purchase prompt: while the purchase tab is selected, or while the bank has no tabs, the prompt replaces the slots.
  - It shows the title and `CHARACTER_BANK_TAB_PURCHASE_PROMPT` / `ACCOUNT_BANK_TAB_PURCHASE_PROMPT`.
  - Below that, "Cost:" and the Retail `BankTab` price, then a 105×21 Purchase button.
  - Purchase is disabled when the player can't afford the tab. Retail reds the price; here the money frame greys it.
- [x] Purchase opens `CONFIRM_BUY_*_BANK_TAB` ("Do you want to purchase a Warband Bank tab for:\n1000g"). Accepting it sends `BankPurchaseTab`.
- [x] Warband bank only: the money frame, 394×25 at BOTTOMRIGHT −3,3, with ThinGoldEdge 178×19, the stored money, and Withdraw and Deposit buttons (105×21).
- [x] Deposit All button (256×24): `Deposit All Reagents` in the character bank, `Deposit All Warbound Items` in the Warband bank. The Warband bank also shows the "Include tradeable reagents" checkbox.

### Skin art
- Dedicated bank textures resolve Blizzard atlas element names through `ui_toolkit::atlas::resolve_region`; Modern keeps the exact pre-conversion FDIDs, UVs, geometry and frame tree.
- Forever uses `bank-frame-background`, `bags-item-bankslot64` for character slots, `bank-frame-item-slotframe` over item icons, and the scaled `bank-divider`. Warband slot art and purchase-tab art retain their set-0 members. The tinted purchase-prompt background resolves `bags-item-slot64`.
- Camelot art does not change the 98-slot grid, slot actions, tabs or purchase/money state. Its 88-slot page cap and uniform spacing are behavior, excluded from this art conversion.
- Standalone container windows have solid backgrounds and dynamic item icons, no hard-coded chrome crops to convert; their Modern and Forever trees stay identical.

Source roots: Retail / Forever = `~/.cache/wow-ui-sim/blizzard-ui/{retail,wowforever}/AddOns/Blizzard_UIPanels_Game/`. M / F = `data/db2/{12.1.0.69933,1.60.1.69913}/UiTextureAtlasMember.csv`. Atlas sheet FDIDs come from those builds' `UiTextureAtlas.csv`.

| Name | UI source | Member rows M / F | Modern / Forever FDID |
|---|---|---|---|
| `bank-frame-background` | Retail `Mainline/BankFrame.xml:677` | 11422 / 17833 | 5782252 / 8118796 |
| `bags-item-slot64`; `bags-item-bankslot64` | Retail `Mainline/BankFrame.xml:571`, `.lua:586`; Forever `Camelot/BankFrame.xml:36` | 7767 / 18117; bankslot F:17832 | 4701874 / 8187737; Forever character slots 8118792 |
| `warband-bank-slot` | Retail `Mainline/BankFrame.lua:582` | 11547 / 11565 (set 0) | 5782246 / 5782246 |
| `bags-icon-addslots` | Existing purchase-tab crop, M/F member:2883 (no BankFrame XML/Lua name literal) | 2883 / 2883 | 969828 / 969828 |
| `bank-frame-item-slotframe`; `bank-divider` | Forever `Camelot/BankFrame.xml:34`; `:76-79` (scale 0.48, BOTTOM +220) | absent / 18121; absent / 18118 | absent / 8188339 |

Sheet rows M:84,1346,1772,1774; F:2616,2617,2689,2690. Forever divider's 864×32 member becomes 414.72×15.36 at (161.64,224.64) in the existing 738×460 frame.

Known missing local BLPs: **8118796, 8118792, 8188339**. Resolution is not guarded or substituted when these files are absent. Ordinary slot sheet 8187737 exists.

### Actions
- [x] Right-clicking a filled slot sends `BankWithdraw` for the shown bank and tab.
- [x] Right-clicking a bag item while the bank is open sends `BankDeposit` into the shown bank's selected tab. Nothing is sent while the purchase prompt shows.
- [x] Deposit / Withdraw opens the money entry: a gold/silver/copper `MoneyInputFrame` in a StaticPopup. Accept sends `BankMoneyTransfer` with the typed copper; an empty entry sends nothing.
- [x] Right-clicking a side tab opens the tab settings: the name box (15 letters) and the Equipment / Consumables / Profession Goods / Reagents / Junk assignments. Okay sends `BankUpdateTabSettings`.
- [x] `BankFailed` shows its Retail error text in UIErrors.
- [x] The IPC `warbank` status lists the Warband bank contents.

## Live proof
- `data/diagnostics/banks-20260924/`: isolated server on :5058.
  - Bankone buys a character tab (1g), deposits Linen and withdraws it, then buys Warband tabs 1 and 2 (1000g and 25,000g).
  - Bankone deposits Copper Ore and 100g into the Warband bank.
  - Banktwo, on the same account, sees the ore, withdraws it, and withdraws 10g.

- Native (Godot) client, `data/diagnostics/bank-live/` (2026-10-01, game-engine `bank`, private server UDP 5114, fresh redb, `godot/tests/bank_live.gd`):
  - Bankone right-clicks Olivia Burnside, picks the gossip option, buys a character tab (1g), deposits and withdraws Linen, buys Warband tab 1 (1000g), deposits Linen and 1g there and withdraws the Linen.
  - Escape closes BankFrame and its bags without opening the game menu.

## How it works
- [banks](../wiki/systems/banks.md)

## Implementation inventory
| File | Role |
|---|---|
| `godot/ui-model/src/game/bank_data.rs` | `BankState`, `BankRequest`, `BankPrompt` |
| `src/game/networking/bank.rs` | Receives contents and failures, follows the interaction, sends requests, IPC status |
| `src/scenes/bank_frame/{mod,view,actions}.rs` | Screen, windows, clicks, edit boxes, confirmation popup |
| `godot/ui-model/src/ui/screens/bank_frame_component.rs` | Retail BankFrame layout |
| `godot/ui-model/src/ui/screens/bank_art.rs` | Slots, labels, money entry, checkbox shared with the guild bank |
| `src/scenes/bag_frame/mod.rs` | Right-click deposit (`use_bag_item`) |

## Tests asserting this spec
- `godot/ui-model/tests/forever_bank_bags.rs`: concrete Modern/Forever atlas regions, Forever divider/item chrome and unchanged slot actions, 1630-line byte-identical Modern bank/container fixture, unchanged container trees across skins.
- `godot/ui-model/src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `godot/ui-model/src/ui/screens/bank_frame_component_tests.rs`
- `src/scenes/bag_frame/mod.rs` (`right_clicking_a_bag_item_deposits_into_the_open_bank_tab`)

## Known gaps (current cycle)
- [ ] Camelot bank-bag template art: `bank-frame-bag-slotframe`, `bankslot-icon-lock`, `bank-frame-bag-slot-bg` (`Camelot/BankFrame.xml:5,11,14`; F members:18120,18122,18119) resolves on sheet 8188339 but is not drawn: existing `BankFrameState` has no bank-bag slots, slot-purchase/lock or cost state. BagText/BagCost labels likewise remain unimplemented. Purchased bank-page side tabs are not bank bags; inventing state or behavior is outside this art-only pass.
- [ ] Selected-tab highlight: Retail blends `CheckButtonHilight` additively (ADD). The ui-toolkit texture renderer has no additive blending, so a gold-tinted UI-Quickslot2 marks the selected tab instead.
- [ ] Background and edge shadows are stretched, not tiled (the ui-toolkit has no tiling attribute for FDID textures); the edge shadow atlases are not drawn.

## Out of scope
- Search box, Cleanup/sort, the tab icon picker and the expansion filter (deferred by decision).
- Drag and drop and stack splitting (no cursor item).
