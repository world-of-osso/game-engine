# Guild Bank Frame

The Retail `GuildBankFrame` at a Guild Vault. The vault is a replicated server game object: it is picked with a right-click, which sends `UseGameObject`. Contract: shared-protocol `protocol/guild_bank_messages.rs` and `interaction_messages.rs` (`GameObjectInfo`, `UseGameObject`). Server rules: game-server `docs/specs/banks.md`. How it works: [banks](../wiki/systems/banks.md).

References: GB.xml / GB.lua = `Blizzard_GuildBankUI/Mainline/Blizzard_GuildBankUI.xml` / `.lua` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

## What it must do

### The vault in the world
- [x] Replicated `GameObjectInfo` mirrors into the main world (`network_runtime/replication.rs`).
- [x] The vault is placed at the replicated position and facing. Its model comes from Retail `GameObjectDisplayInfo.FileDataID`: display 7607 is `guildvault_human_01.m2` (199769).
- [x] The vault is pickable as `WorldObjectInteractionKind::ServerObject`. Right-clicking it sends `UseGameObject` with its server entity bits.
- [x] The `NpcFrameEvent::Opened { role: GuildBanker }` event opens `GuildBankState`. The server's `GuildBankContents` fill it.
- [x] The guild bank opens as the Wide `WindowId::GuildBank` window with the backpack. Closing the window sends `CloseInteraction`, and `InteractionClosed` closes the frame.

### Layout
- [x] Frame 750×428 (GB.xml:167) with the `metal_frame` chrome and GuildVaultBG (590068).
- [x] Seven UI-GuildBankFrame-Slots columns (100×311) at 18,59, 3 px apart. Each column holds 14 buttons in two sub-columns of 7: Button1 at 7,3, then 7 px apart; Button8 is 12 px to the right. Slot ids run 1–98.
- [x] Side tabs are 42×50 (UI-GuildBankFrame-Tab), starting at TOPRIGHT −1,−17 and stepping 50 down. The Guild Master's buy tab (UI-GuildBankFrame-NewTab) comes last.
- [x] Mode tabs Guild Bank / Log / Money Log / Info start at BOTTOMLEFT 7,−30. The shown mode takes no click.
- [x] Tab title plate: the name plus its access suffix — `(Full Access)` in green, or `(Withdraw Only)` / `(Deposit Only)` / `(Locked)` in red. A locked tab greys its columns.
- [x] Withdrawal plate: `GUILDBANK_REMAINING_MONEY` with `N Stacks`, `None` or `Unlimited`.
- [x] Money bar: `Available Amount:` shows the withdraw allowance, or `Unlimited`. The guild money sits at BOTTOMRIGHT, with Deposit and Withdraw buttons (100×21). Withdraw is disabled without an allowance.
- [x] Buy screen (Guild Master, buy tab selected): "Do you wish to purchase this tab?", "(n/6 tabs purchased)", the price and a Purchase button (124×21). Purchase opens `CONFIRM_BUY_GUILDBANK_TAB`, and accepting sends `GuildBankBuyTab`. Non-leaders see `NO_GUILDBANK_TABS`.
- [x] Log and Money Log modes query their log. They list Retail lines: "X deposited Linen Cloth x 20 ( 1 min ago )", "withdrew", "purchased a guild bank tab for".
- [x] Info mode shows the tab text. The Guild Master gets an edit box and Save, which sends `GuildBankSetTabText`.

### Actions
- [x] Right-clicking a filled slot sends `GuildBankWithdraw`. Right-clicking a bag item sends `GuildBankDeposit` into the selected tab.
- [x] Money entry sends `GuildBankMoneyTransfer`.
- [x] A change broadcast while a log is shown re-queries it.
- [x] `GuildBankFailed` shows its Retail error text.
- [x] The IPC `guild_vault` status lists the vault contents.

## Live proof
- `data/diagnostics/banks-20260924/`:
  - Bankone, the Guild Master, uses the Stormwind Guild Vault, buys tab 1 (100g), and deposits Peacebloom and 50g.
  - Banktwo, rank Member with tab 1 set to view, deposit and 2 stacks a day, plus 5g a day in gold, withdraws the Peacebloom and 5g.
  - The item log reads "Bankone deposited Peacebloom x 10" and "Banktwo withdrew Peacebloom x 10". The money log shows the tab purchase, the 50g deposit and the 5g withdrawal.

- Native (Godot) client, `data/diagnostics/bank-live/` (2026-10-01, two clients, `godot/tests/bank_live.gd`):
  - Bankone, Guild Master, opens the Stormwind vault, deposits 150g, buys tab 1 (100g) and deposits Linen x10.
  - Banktwo, rank Member with the vault open, sees the tab and the Linen arrive, and withdraws the Linen; Bankone's slot empties.
  - Item log: "Bankone deposited Linen Cloth x 10", "Banktwo withdrew Linen Cloth x 10". Money log: "Bankone deposited 150g", "Bankone purchased a guild bank tab for 100g".

## How it works
- [banks](../wiki/systems/banks.md)

## Implementation inventory
| File | Role |
|---|---|
| `src/game/networking/game_objects.rs` | `GameObjectDisplays`, vault spawn observer |
| `src/rendering/ui/target.rs`, `src/game/networking/quests.rs` | `ServerObject` pick → `UseGameObject` |
| `src/game/bank_data.rs` | `GuildBankState`, `GuildBankRequest`, log lines |
| `src/game/networking/bank.rs` | Guild bank receive/send |
| `src/scenes/bank_frame/{mod,view,actions}.rs` | Screen, windows, clicks |
| `src/ui/screens/guild_bank_frame_component.rs` | Retail GuildBankFrame layout |

## Tests asserting this spec
- `src/game/networking/game_objects.rs` (tests)
- `src/network_runtime/replication.rs` (`game_objects_mirror_with_their_position`)
- `src/game/networking/quests_tests.rs` (`using_a_mirrored_game_object_sends_use_game_object_and_its_role_opens_a_frame`)
- `src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `src/ui/screens/guild_bank_frame_component_tests.rs`

## Known gaps (current cycle)
- [ ] The selected tab uses the same gold marker as the bank frame, because there is no additive blending.
- [ ] Emblem, outer and inner corner tiling of the Retail frame are not drawn (the metal_frame chrome replaces BasicFrameTemplate).
- [ ] The info edit box is single-line: the ui-toolkit edit box has no multi-line attribute.
- [ ] The log is not a scrolling message frame: only the last 21 lines show.

## Out of scope
- Tab name/icon editing UI (the icon picker is deferred), item moves inside the bank, search, guild repairs, and the Guild Control UI.
