# Guild ranks and permissions

Retail guild settings belong in the native guild/communities frame, in Modern and
Forever skins. `godot/ui-model/src/guild_ranks.rs` provides portable request/state
decisions; `guild_rank_frame.rs` renders the native roster/settings canvas and
`godot/rust/src/guild_ranks.rs` connects it to GuildChannel. See [banks](../wiki/systems/banks.md#rank-settings-model) for current data flow.

## What it must do

### Communities presentation
- [x] Mainline CommunitiesFrame is814×426, centered, with32px Chat/Roster/Guild Benefits/Info icon tabs at the right edge and52px pitch. Forever changes art only.
- [x] Expanded roster uses source592×315 ScrollBox,20px CommunitiesMemberListEntryTemplate rows, ColumnDisplay headers and an unclipped sibling MinimalScrollBar. Previous/Next/Page controls and paging state are removed; wheel advances40px, stepper20px, thumb reaches every member.
- [x] Name/rank header sorting changes presentation only; member actions still use authoritative character names and rank hierarchy. Available GuildSnapshot level/class/last-online data populates source fields. Officer notes never populate the public Note column.
- [x] Guild Settings belongs to Roster for guild leaders/officers; Invite belongs to Chat. Both use the XML130×20 bottom-right anchor(-5,+5), not simultaneous invented footer positions. Info preserves authoritative MOTD/info editors.
- [ ] Communities stream/history/voice, guild benefits/rewards/news/challenges, zone/public-note metadata and extra achievement/profession columns have no backing in these snapshots. Tabs expose source navigation only; no messages/rewards/stats, emblems or service requests are fabricated. Missing-data sort headers are disabled.

### Rank controls
- [x] Guild Control uses the Retail Mainline 338×444 TranslucentFrameTemplate shell, navigation dropdown (159 pixels), rank dropdown (180 pixels), RankChangeTemplate rows and separate rank-permissions/bank panes. No oversized combined canvas, generic category buttons or extra Save controls.
- [x] Mainline Officer checkbox id22 reflects either pinned TC officer-chat bit; explicit toggle sets/clears both only. MOTD/info/note permissions remain independently preserved/enforced, represented by the Mainline OfficerPermissions bullet lines, not invented checkboxes.
- [x] Wire rights use pinned TC `a352b1fa` masks; server layout-2 migration maps each compact bit independently, preserving mixed rights and all bank data/member positions. Shared semantic epoch1 rejects older mask interpretations at connect.
- [ ] Authenticator, Neighborhood Manager, bank-tab modification, Discord and club/voice/event/guild-finder services have no authoritative backing here. Their source checkboxes remain disabled or source descriptive bullet lines; no privileges are synthesized.
- [x] Guild settings → Guild Ranks opens the same Retail functionality in both skins.
- [x] Guild Master alone adds, removes, renames, reorders and configures ranks; rank 0 is immutable, there are 2–10 ranks and names have at most 15 characters.
- [x] Occupied ranks cannot be removed; reordered members retain their rank identity.
- [x] Requests leave selected-rank data unchanged until the authoritative reply arrives.
- [x] Mutating controls wait for authoritative state before sending another write; queued clicks cannot overwrite a prior permission edit using stale flags.
- [x] Source rank/gold/tab inputs save changed values on Enter/focus loss without extra Save controls; Escape restores text before focus-loss dispatch and sends no write. Bank scrolling uses Lua stepSize8 and XML74px rows plus4px gaps.

### Invitation and guild profile
- [x] `/ginvite` / `/guildinvite`, `/gquit` / `/guildquit`, `/gremove` / `/guildremove` send authoritative guild actions, not help.
- [x] AuthChannel invitation delivery mounts GuildInviteFrame independently of Communities. Join waits for authoritative acceptance; Decline sends the pending-invite refusal. Both skins share controls and mechanics.
- [x] `/gmotd` / `/guildmotd` sends SetGuildMotd; authoritative profile replies update MOTD and info.
- [x] `/ginfo` / `/guildinfo` sends a read-only query and reports the persisted creation date for charter-created guilds; legacy guilds remain explicitly unknown, never backdated.
- [x] Communities exposes Invite Member and Info; MOTD/info editors send authoritative writes, keep displayed text unchanged pending reply, and hide edits without rank permission. Member menu Remove uses strict hierarchy.
- [x] Native Info editor uses TextEdit for multiline text (499 characters); MOTD uses Enter-to-accept input (255 characters). Native postsetup supplies limits before projection.
- [x] MOTD input preserves typed order and caret position across projection updates, including insertion in the middle of existing text.
- [x] Authenticated native multiline editor and authoritative readback inspected; both-skin profile and restart preserve both lines.
- [ ] GuildInviteFrame tabard/achievement data and exact translucent border art remain pending (no fabricated emblem or points).

### Permissions and members
- [x] Mainline exposes guild-chat speak, Officer(id22), invite/remove/promote/demote and monetary checks. MOTD/info/note rights are preserved/enforced independently and appear only in source OfficerPermissions bullet text, not invented controls.
- [x] Per-tab view/deposit/withdraw-stacks controls show server state. Zero denies withdrawals; only Guild Master is unlimited.
- [x] Whole-gold input sends copper/day, sharing the allowance with guild repairs.
- [x] Right-click member context menu stays on screen and shows Promote/Demote only when flags and strict hierarchy allow it; promotion stops below the actor's rank.
- [x] `/gpromote` / `/guildpromote` and `/gdemote` / `/guilddemote` send the same authoritative rank requests (Retail SlashCommands.lua GUILD_PROMOTE/GUILD_DEMOTE), not generic help.
- [x] Server refusals display Retail GlobalStrings. `/o` and `/officer` use the Officer channel and its styling.

## How it works

- [Banks](../wiki/systems/banks.md#rank-settings-model) — authoritative model and native integration.
- Shared-protocol `docs/guild-ranks.md` — request/state wire contract.

## Implementation inventory

| File | Role |
|---|---|
| `godot/ui-model/src/guild_ranks.rs` | Selected server state and exact request values |
| `godot/ui-model/src/guild_rank_frame.rs` | Native communities entry and member menu |
| `godot/ui-model/src/guild_control.rs`, `guild_control_art.rs` | Mainline GuildControlUI panes, menus, source art and flag IDs |
| `godot/rust/src/guild_ranks.rs` | Mount, poll clicks, send and apply native state |
| `godot/network/src/lib.rs`, `godot/rust/src/account.rs` | Receive GuildRanksState; send requests on GuildChannel |
| `godot/ui-model/src/game/chat_data.rs` | Officer channel styling classification |
| `godot/ui-model/src/ui/chat_frame.rs` | Officer chat command parsing |

## Tests asserting this spec

- `godot/tests/communities_fidelity.gd`: source rectangles, physical wheel/thumb reaching member32, name sorting, member hierarchy menu, all four source tabs and invitation reducer in both skins; fixture authority only, not a live-server/deployment claim.
- `godot/tests/guild_control_fidelity.gd`: both-skin1920×1080 native source rectangles, physical menus/rename/id22/gold/tab edits, pending-authority pacing and eighth-tab overflow. Fixture replies only; real authority/persistence are asserted by server tests.
- Mainline Discord officer bullet GlobalString is absent from the current canonical strings; its text is not fabricated. Tab icon and bank-purchase payloads are absent; those source features remain named gaps.

- `godot/tests/guild_member_controls_live.gd`: native right-click menu is inside the viewport; Promote/Demote alter and restore the authoritative roster label.
- `godot/tests/guild_profile_live.gd`: private guild master types MOTD, inserts before the final character, accepts and checks the authoritative displayed reply; real Godot input/projection boundary.

- `godot/ui-model/tests/native_guild_ranks.rs`: 5 authoritative model/Officer tests pass.
- `godot/ui-model/tests/guild_rank_widgets.rs`: 9 tests pass in both skins, including
  mounted state, exact click requests, disabled controls, hierarchy, pacing and purchased tabs.
- `godot/network/src/guild_rank_wire_tests.rs`: real loopback UDP request/reply proof passes.
- `godot/rust/src/account.rs`: native state/refusal dispatch test passes.
- Existing `micro_menu` binary: 11 regression tests pass.
- `godot/tests/capture_ui_screen.gd`: native rendered disclaimer bands pass in both skins.

Native build and inspected 1920×1080 captures pass at `1e62844b`.
Exact names, commands, revisions and exclusions: `data/diagnostics/guildranks-2026-10-05/proof.md`.

## Proof boundaries

Private UDP54381 authenticated lifecycle, profile, strict member controls and bank proof now cover Modern and Forever. Charter-created guilds survive restart with five founders; member leave/kick → decline/reinvite/Join passes in both skins. Native UI guild gate37/37 and native guild gate6/6 pass; the later registrar-portrait boundary additionally passes both-skin control and real NPC portrait checks. Evidence: canonical `data/diagnostics/guilds{3,4}-2026-10-11/` and inspected AgentShared PNGs. Tabard/achievement payloads remain absent; no emblem or points are fabricated. Full unrelated Communities/social features and resource-leak acceptance are not claimed.

## Out of scope

None of the requested native UI behavior is waived. The partial model is not completion
of the full feature. Forever changes presentation, not Retail mechanics.

## Retail sources

Pinned authority: TrinityCore `a352b1fa`, `src/server/game/Guilds/Guild.h:102–127`
(`GuildRankRights`) and `Guild.cpp:2935–2938,3732–3735` (Moderator when either
officer-chat bit is present). These named integer rights are mapped individually;
unsupported/unnamed TC rights are not accepted or fabricated.

Root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:4,536–620`: rank count/order and occupied deletion.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.xml:7`: 15-letter rank names.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:137–238,420–459`: tab rights, counts, flags and gold/repair input.
- `Blizzard_Communities/GuildRoster.lua:137–141`: member rank actions.
- `Blizzard_Communities/CommunitiesFrame.xml:241–304,369–431,458–463,567–590`: root, side tabs, member list and footer anchors; `CommunitiesFrame.lua:544–567,1753–1756`: Invite only in Chat, Guild Settings in Roster.
- `Blizzard_Communities/CommunitiesMemberList.xml:5–32,185–241,296–350` and `.lua:96–133,584–614,1235–1427`:20px rows, columns, expanded geometry, metadata and sibling scrollbar.
- `Blizzard_Communities/CommunitiesTabs.xml:5–52`:32px side-tab template,64px backing,30px icon and ADD highlights.
- `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml:1122–1230` and `.lua:766–805`: ColumnDisplay strips, headers and anchors.
- `data/GlobalStrings.csv`: Retail refusal strings.

See [guild bank](guild-bank-frame.md) for vault actions and game-server
`docs/specs/guild-ranks.md` for authority, durability and enforcement.
