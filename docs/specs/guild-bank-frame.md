# Guild Bank Frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

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
- [x] Frame 750×428 (GB.xml:167) with non-portrait `metal_frame_no_portrait` chrome and GuildVaultBG (590068) in both skins. GB.xml:167 inherits `BasicFrameTemplate`, not `PortraitFrameTemplate`; do not draw an empty portrait ring.
- [x] Seven UI-GuildBankFrame-Slots columns (100×311) at 18,59, 3 px apart. Each column holds 14 buttons in two sub-columns of 7: Button1 at 7,3, then 7 px apart; Button8 is 12 px to the right. Slot ids run 1–98.
- [x] Side tabs are 42×50 (UI-GuildBankFrame-Tab), starting at TOPRIGHT −1,−17 and stepping 50 down. The Guild Master's buy tab (UI-GuildBankFrame-NewTab) comes last.
- [x] Mode tabs Guild Bank / Log / Money Log / Info start at BOTTOMLEFT 7,−30. The shown mode takes no click.
- [x] Tab title plate: the name plus its access suffix — `(Full Access)` in green, or `(Withdraw Only)` / `(Deposit Only)` / `(Locked)` in red. A locked tab greys its columns.
- [x] The name and its suffix sit on one line. Both labels are sized to their text, as `TabTitle` has no width (GB.xml:195-199). The `guild-bank` fixture fails if either label wraps.
- [x] Withdrawal plate: `GUILDBANK_REMAINING_MONEY` with `N Stacks`, `None` or `Unlimited`.
- [x] Money bar: `Available Amount:` shows the withdraw allowance, or `Unlimited`. The guild money sits at BOTTOMRIGHT, with Deposit and Withdraw buttons (100×21). Withdraw is disabled without an allowance.
- [x] Buy screen (Guild Master, buy tab selected): "Do you wish to purchase this tab?", "(n/6 tabs purchased)", the price and a Purchase button (124×21). Purchase opens `CONFIRM_BUY_GUILDBANK_TAB`, and accepting sends `GuildBankBuyTab`. Non-leaders see `NO_GUILDBANK_TABS`.
- [x] Log and Money Log modes query their log. They list Retail lines: "X deposited Linen Cloth x 20 ( 1 min ago )", "withdrew", "purchased a guild bank tab for", and guild repairs "X withdrew 1g 50s 16c for repairs" (`GUILDBANK_REPAIR_MONEY_FORMAT`, GB.lua:797-798).
- [x] Info mode shows the tab text. The Guild Master gets an edit box and Save, which sends `GuildBankSetTabText`. Read-only, locked and unpurchased tabs never queue writes to an editor that is not rendered. Read-only text starts at the same top edge as editable text; `guild_bank_info_readonly_live.gd` checks the first rendered glyph's position.

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

- Guild repair (native, `data/diagnostics/merchant3-20261001/live/shots/g-7-guild-money-log.png`): the Money Log reads "Fbguildfour deposited 10g" and "Fbguildfour withdrew 16c for repairs".

### Both-skin direct item moves — 2026-10-25

`godot/tests/guild_bank_directions_live.gd` passes with two actual clients in each skin: master deposit/member withdrawal, then member deposit/master withdrawal, with both views observing the real Linen Cloth x10 moves. It also proves the member's 1g withdrawal, exhausted daily allowance/disabled Withdraw, subsequent 1g deposit and authoritative item/money logs. This closes the old mixed-skin pair's missing direct Modern deposit and Forever withdrawal cells, not every bank-permission permutation.

Inspected full-HD originals and actor/build receipts: `/syncthing/AgentShared/2026-10-25/guilds/bank/{modern-1792951590,forever-1792951825}/`. The root `coverage.md` owns the qualified lifecycle matrix.

## How it works
- [banks](../wiki/systems/banks.md)

## Implementation inventory
| File | Role |
|---|---|
| `src/game/networking/game_objects.rs` | `GameObjectDisplays`, vault spawn observer |
| `src/rendering/ui/target.rs`, `src/game/networking/quests.rs` | `ServerObject` pick → `UseGameObject` |
| `godot/ui-model/src/game/bank_data.rs` | `GuildBankState`, `GuildBankRequest`, log lines |
| `src/game/networking/bank.rs` | Guild bank receive/send |
| `src/scenes/bank_frame/{mod,view,actions}.rs` | Screen, windows, clicks |
| `godot/ui-model/src/ui/screens/guild_bank_frame_component.rs` | Retail GuildBankFrame layout |

## Tests asserting this spec
- `godot/tests/guild_bank_log_live.gd`: real private 25-transaction money log, clicks native scroll controls and reaches the oldest retained transaction; covers GameClient input routing as well as the portable screen.
- `godot/tests/guild_vault_live.gd`: private guild master right-clicks an authentic vault and waits for both vault and embedded backpack. The combined native canvas registers plain vault and portrait backpack borders before projection.
- `src/game/networking/game_objects.rs` (tests)
- `src/network_runtime/replication.rs` (`game_objects_mirror_with_their_position`)
- `src/game/networking/quests_tests.rs` (`using_a_mirrored_game_object_sends_use_game_object_and_its_role_opens_a_frame`)
- `godot/ui-model/src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `godot/ui-model/src/ui/screens/guild_bank_frame_component_tests.rs`

## Known gaps (current cycle)
- [x] Selected tabs use authentic `CheckButtonHilight` (local FDID130724), 36×34 and additive blending, as GB.xml:156 specifies. The recoloured quickslot marker is removed; native additive texture rendering already exists.
- [ ] Emblem, outer and inner corner tiling of the Retail frame are not drawn (the metal_frame chrome replaces BasicFrameTemplate).
- [x] Native tab Info uses TextEdit, preserves newlines and enforces Retail's 500-character limit (GB.xml:758). `godot/tests/guild_bank_info_live.gd` types two lines, saves, switches modes and checks server-rehydrated text.
- [x] Logs retain Retail's latest 128 lines, newest first, with wheel/stepper/thumb scrolling. Scroll position survives authoritative view refreshes on the combined vault/backpack canvas. A wheel notch or stepper press advances one transaction line, not one pixel. The current server retains 25 transactions per log; the client scrolls every supplied record.

## Tab name and icon editor

- Right-click a purchased tab as Guild Master to edit its name and icon; Cancel and Escape send no update. Okay sends the existing `GuildBankSetTabInfo` request and waits for server contents rather than changing the displayed tab optimistically. Mainline `Blizzard_GuildBankUI.lua:621–645,820–901` supplies the interaction and quote removal/empty-name rule; `SharedUIPanelTemplates.xml:1764–1767` supplies the 16-letter edit box.
- The icon list uses installed `ManifestInterfaceActionIcon.csv` and `ManifestInterfaceItemIcon.csv`. Their exact equivalence and order relative to Retail's native `GetLooseMacroIcons`, `GetLooseMacroItemIcons`, `GetMacroIcons` and `GetMacroItemIcons` are not yet established. The server currently rejects 16-character tab names; full parity is not claimed until that boundary and actual native behavior are tested.
- Automated tests: `godot/ui-model/tests/native_guild_bank.rs::guild_tab_edit_*` and `guild_tab_editor_*`. Both-skin live proof is pending.

## Out of scope
- Item moves inside the bank and search are not yet implemented. Guild Control UI is tracked separately in [guild ranks](guild-ranks.md). Guild repairs are made at a merchant ([merchant-frame](merchant-frame.md)); player-managed repair rights use the same daily allowance as gold withdrawals. Native rank settings integration remains pending.
