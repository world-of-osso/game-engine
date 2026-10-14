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

## Native character-bank item cursor (2026-10-08)
`godot/rust/src/bank.rs` resolves bank slot actions through `BankSession::slot_location` on the currently selected character tab. `bag_cursor.rs` captures that location at pickup; the original tab remains on CursorItem while the frame switches tabs. `receive_bank` projects Character BankContents into InventoryState.bank_slots using the same item-catalog conversion as bags; Warband contents never overwrite it. BankState still owns the frame's server snapshot. Closing clears the projection and stale bank cursor/split owners.

Character-bank left clicks and drag releases use the shared cursor's SwapItem/SplitItem requests on InventoryChannel. Shift-click shares StackSplitFrame, anchored to BankFrameItemN instead of ContainerFrameNSlotN. Right-click bank withdrawal and bag auto-deposit are unchanged. Warband whole-bag deposit retains its existing BankChannel path; exact-slot operations there remain out of scope.

Sources: Retail `Blizzard_UIPanels_Game/Mainline/BankFrame.lua` HandleItemPickup/OnDragStart/OnReceiveDrag (416-448), OnModifiedClick (433-444), SplitStack (590-592); [character-bank contract and targeted tests](../../specs/bank-frame.md). Server move semantics follow TrinityCore a352b1fa CMSG_SWAP_ITEM/CMSG_SPLIT_ITEM, CanBankItem and StoreItem; server contract is `game-server/docs/specs/banks.md`. No live acceptance added on this host.

## Native bank art (2026-10-08)

`apply_bank_postsetup` applies texture tiling/ADD flags not exposed by RSX. The selected tab uses CheckButtonHilight; each ItemButton has a model-owned additive overlay whose visibility follows native hover through `registry.set_hidden`, including inherited alpha and closure. Bank buttons keep the original frame press callback for coordinates/drag and right/Shift dispatch, adding only hover callbacks; ordinary Button `pressed` callbacks would lose that boundary.

Native `ImagePart.tiling` carries each authored axis to a cropped-member shader embedded in Rust. Repetition uses the member's pixel dimensions and clamps samples inside its UV rectangle, not the surrounding atlas. The build helper ships Rust/WGSL sources but excludes project `.gdshader`, so an `include_str!` of a project shader failed the first build. No alternate rendering path was retained.

The [bankart source/proof table](../../specs/bank-frame.md#bounded-bankart-pass--2026-10-08) owns atlas names/rects, RED/GREEN counts, six native captures, and the unresolved bank-bag layout/state and local vertical-shadow asset gaps. Offline `bank_preview.rs` calls the production bank projection; `capture_ui_screen.gd` checks true1920×1080 and native hover/modified-click dispatch. No server/live movement proof is implied.

## Gotchas
- The purchase tab is selectable as index `tabs.len()` while `next_tab_cost` is set. After a purchase the same index is the new tab, so the frame lands on it.
- Guild bank logs are fetched per mode or tab. Every contents broadcast re-queries the shown log, because Retail refreshes on `GUILDBANKLOG_UPDATE`.

## Rank settings model

`godot/ui-model/src/guild_ranks.rs` retains `GuildRanksState` and exposes the selected
rank directly. The rank settings contain eight tab-rights entries even for unpurchased
tabs; only `tab_names` entries are mounted or accepted for editing.
Control methods return `GuildRankRequest` without editing cached state;
only `apply` replaces it. Gold input converts whole gold to copper. The model validates
GM-only editing, immutable rank zero, occupied deletion and member hierarchy.
`guild_rank_frame.rs` mounts CommunitiesFrame; `guild_communities.rs` renders the
Mainline814×426 shell, right-edge icon tabs and expanded20px roster rows. Scroll
positions belong to the registry, not a session page. Name/rank sorting reorders
references for painting without changing authoritative member/rank state. Optional
GuildSnapshot metadata fills level/class/last-online fields; officer notes are never
painted into the public Note column. Settings and Invite share the source footer
anchor but appear in Roster and Chat respectively. Unsupported Communities services
remain [named contract gaps](../../specs/guild-ranks.md#communities-presentation).
`guild_control.rs` switches
Mainline's 338×444 GuildControlUI between order, permission and bank panes. Its
159px navigation and180px rank dropdowns use shared Retail menus; rank zero is
excluded from editable rank menus. `guild_control_art.rs` projects the source
TranslucentFrameTemplate,20px permission checks and26px square buttons in both skins.
Checkbox IDs map explicitly to pinned TC masks, not to bit offsets. id22 checks
either officer-chat bit and toggles only both chat bits; independent MOTD/info/note
rights remain in server state and source bullet lines, without invented controls.
Member menu actions use the same hierarchy checks as request generation; roster
paging keeps members inside the window. A query or mutation disables writes until
`apply` receives authoritative state, preventing consecutive permission clicks from
sending a full bitmap derived from stale state. Immediate send failure clears that
wait. Selected rank data is never edited optimistically.

`godot/rust/src/guild_ranks.rs` owns the native canvas and polls its action queue.
`account.rs` sends `GuildRankRequest` on ordered-reliable `GuildChannel`; the network
bridge receives `GuildRanksState` and account dispatch forwards it to the session.
Refusals also go to `UIErrorsFrame`. Enter/focus loss saves changed rank/gold/stack
fields; field Escape restores text before Blur and sends no write. Outside a field,
Escape closes the canvas. Native bank scrolling routes through the production
ScrollBox controller with Lua stepSize8 and74px rows plus4px gap. Loading preserves
early guild/invitation state; logout resets it.
The Guild micro button opens this entry point (previously unavailable).
Offline capture methods feed a concrete snapshot into the same production screen.

## Guild lifecycle and charters

[Guilds](guilds.md) owns invitations, membership/profile controls and the authentic Retail charter flow. Both-skin vault proof includes authoritative item deposit/withdraw, multiline tab info, selected-tab additive art and reaching the oldest retained log row through native steppers. Guild log retention is server-owned25, not the portable display cap128. Inspected evidence: canonical `data/diagnostics/guilds3-2026-10-11/`, AgentShared `2026-10-11/guilds/`; native guild log/input and readonly-info fixtures cover the projection boundary.

## Sources

- [Guild ranks contract](../../specs/guild-ranks.md) — required UI and proof boundaries.
- [Portable model](../../../godot/ui-model/src/guild_ranks.rs) — state and request decisions.
