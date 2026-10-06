# Banks (client)

How the client shows the character bank, the Warband bank and the guild bank. The contracts are [bank-frame](../../specs/bank-frame.md) and [guild-bank-frame](../../specs/guild-bank-frame.md).

## Data flow

### Opening
- `quests::receive_interactions` is the single reader of `InteractionOpened`. It forwards `Role(Banker)` and `Role(GuildBanker)` as `NpcFrameEvent::Opened`.
- `networking/bank.rs::follow_interactions` then opens `BankState` or `GuildBankState`.
- **Banker:** Newton Burnside and the other bankers use gossip menu 699. Picking "I would like to check my deposit box." opens the Banker role.
- **Vault:** a replicated server object (`GameObjectInfo`).
  - It mirrors through `EntitySnapshot`.
  - `networking/game_objects.rs::spawn_replicated_game_object` places it and attaches the M2. The M2 FDID comes from `data/db2/12.1.0.69933/GameObjectDisplayInfo.csv`.
  - It gets `WorldObjectInteraction { kind: ServerObject }`, as does every usable object (`GameObjectInfo::is_usable`); decoration such as fires gets none (no cursor, highlight or click).
  - `target.rs` sends `NpcInteractionRequest::UseObject` without a client range check; the server checks the model-box reach. `quests.rs` turns that request into `UseGameObject`.

### Contents
- `BankContents` / `GuildBankContents` replace the whole bank on every change. There are no deltas.

### Frames
- `scenes/bank_frame` builds both frames from the state plus the player's money (`CharacterStatsSnapshot.gold`).
- The windows are Wide (`WindowId::Bank` / `GuildBank`) and open with the backpack.
- **Clicks:** clicks on `bank_*` / `guild_bank_*` actions go through `actions.rs`. They return requests, a confirmation popup (`PopupStack`) or edit box texts to set.
- **Edit boxes:** the money entries, the tab name box and the guild info box are ordinary registry edit boxes, read at Accept.
- **Bag right-click:** `bag_frame::use_bag_item` routes a right-clicked bag item to the merchant (sell), the bank (deposit into the shown tab) or the guild bank (deposit into the selected tab).

## Gotchas
- The purchase tab is selectable as index `tabs.len()` while `next_tab_cost` is set. After a purchase the same index is the new tab, so the frame lands on it.
- Guild bank logs are fetched per mode or tab. Every contents broadcast re-queries the shown log, because Retail refreshes on `GUILDBANKLOG_UPDATE`.

## Rank settings model

`godot/ui-model/src/guild_ranks.rs` retains `GuildRanksState` and exposes the selected
rank directly. Control methods return `GuildRankRequest` without editing cached state;
only `apply` replaces it. Gold input converts whole gold to copper. The model validates
GM-only editing, immutable rank zero, occupied deletion and member hierarchy.
It is not mounted or connected to native rank transport.

## Sources

- [Guild ranks contract](../../specs/guild-ranks.md) — required UI and explicit native integration gaps.
- [Portable model](../../../godot/ui-model/src/guild_ranks.rs) — state and request decisions.
