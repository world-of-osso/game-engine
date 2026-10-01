# Bank Frame

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
| `src/game/bank_data.rs` | `BankState`, `BankRequest`, `BankPrompt` |
| `src/game/networking/bank.rs` | Receives contents and failures, follows the interaction, sends requests, IPC status |
| `src/scenes/bank_frame/{mod,view,actions}.rs` | Screen, windows, clicks, edit boxes, confirmation popup |
| `src/ui/screens/bank_frame_component.rs` | Retail BankFrame layout |
| `src/ui/screens/bank_art.rs` | Slots, labels, money entry, checkbox shared with the guild bank |
| `src/scenes/bag_frame/mod.rs` | Right-click deposit (`use_bag_item`) |

## Tests asserting this spec
- `src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `src/ui/screens/bank_frame_component_tests.rs`
- `src/scenes/bag_frame/mod.rs` (`right_clicking_a_bag_item_deposits_into_the_open_bank_tab`)

## Known gaps (current cycle)
- [ ] Selected-tab highlight: Retail blends `CheckButtonHilight` additively (ADD). The ui-toolkit texture renderer has no additive blending, so a gold-tinted UI-Quickslot2 marks the selected tab instead.
- [ ] Background and edge shadows are stretched, not tiled (the ui-toolkit has no tiling attribute for FDID textures); the edge shadow atlases are not drawn.

## Out of scope
- Search box, Cleanup/sort, the tab icon picker and the expansion filter (deferred by decision).
- Drag and drop and stack splitting (no cursor item).
