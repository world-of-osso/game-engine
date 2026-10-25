# Guild ranks and permissions

Retail guild settings belong in the native guild/communities frame, in Modern and
Forever skins. `godot/ui-model/src/guild_ranks.rs` provides portable request/state
decisions; `guild_rank_frame.rs` renders the native roster/settings canvas and
`godot/rust/src/guild_ranks.rs` connects it to GuildChannel. See [banks](../wiki/systems/banks.md#rank-settings-model) for current data flow.

## What it must do

### Communities presentation
- [x] Mainline CommunitiesFrame is814×426, centered, with32px Chat/Roster/Guild Benefits/Info icon tabs at the right edge and52px pitch. Selected/hover tab glows use source ADD blending, preserving icon pixels. Forever changes art only.
- [x] Expanded roster uses source592×315 ScrollBox,20px CommunitiesMemberListEntryTemplate rows, ColumnDisplay headers and an unclipped sibling MinimalScrollBar. Previous/Next/Page controls and paging state are removed; wheel advances40px, stepper20px, thumb reaches every member. Production pointer admission uses Communities visibility, so settings-closed roster input remains enabled; the game menu still blocks it.
- [x] Name/rank header sorting changes presentation only; member actions still use authoritative character names and rank hierarchy. Available GuildSnapshot level/class/last-online data populates source fields. Officer notes never populate the public Note column.
- [x] Guild Settings belongs to Roster for guild leaders/officers; Invite belongs to Chat. Both use the XML130×20 bottom-right anchor(-5,+5), not simultaneous invented footer positions. Info preserves authoritative MOTD/info editors.
- [ ] Club/mobile streams and voice, guild benefits/rewards/news/challenges and extra achievement/profession columns have no backing in these snapshots. Tabs expose source navigation only; no messages/rewards/stats, emblems or service requests are fabricated. Unsupported extra-column sort actions remain disabled; [Zone sorting](guild-zone-sort.md) owns the existing Zone header and approved logout-seconds policy, while public Note sorting uses the qualified comparison below. No missing metadata is fabricated.

- [x] Level/Class headers sort the existing authoritative profile metadata by numeric level/class ID, not display-string order. Initial click sorts ascending, repeated click reverses and another column starts ascending. Missing attributes sort first ascending and last descending, matching `CommunitiesMemberList.lua:798–819,905–914`. Sorting never changes authoritative member/rank order or action targets. Equal-key fallback has a separate [numeric-header tie contract](guild-numeric-sort-ties.md), using the qualified represented-field base policy rather than claiming full native Club ordering. Both-skin native physical Level/Class clicks, row context targeting and unchanged source header geometry pass; ten1920×1080 before/after PNGs inspected2026-10-15. Fixtures are offline snapshots, not a fresh server-authority claim.

### Online member zone

- [x] Existing roster/detail Zone fields display only the server's optional `GuildMemberSnapshot.online_zone` for an online member; missing authority remains empty, not a map name or invented label. Both skins keep existing field geometry and member action identity.
- [x] New authoritative snapshots replace displayed zone labels; offline detail Zone remains empty. The separate [Last Online contract](guild-last-online.md) owns offline recency, not this zone-only slice. [Zone header sorting](guild-zone-sort.md) is a separate contract; Club/mobile presence remains excluded.
- Source: Mainline `CommunitiesMemberList.lua:115–120,1285–1297`, `GuildRoster.lua:115`; server uses existing GroundMap→AreaTable zone authority. Protocol/server/engine land together.
- Test: `guild_online_zone_roster_and_detail_follow_authority_both_skins` (integration); behavioral RED observed before production change. Targeted GREEN and coordinated gz689 passed. Post-landing two authenticated members prove actual Elwynn Forest/Westfall fields, movement refresh and disconnected/offline fields in both skins; [native artifact provenance and proof limits](../wiki/systems/guilds.md#online-member-zone).

### Direct member-rank assignment

- [x] A lower-ranked member's detail card shows the Mainline `WowStyle1DropdownTemplate` only when Promote/Demote rights allow a change; otherwise it keeps rank text. `GuildRoster.xml` anchors LEFT at RankLabelRIGHT+10; Lua fixes width169−label width and RankLabel height20. The25px dropdown centers on that label; following rows and total card height grow10px, to207/260 without/with officer-note visibility.
- [x] Menu radios use real rank names and current selection. With Promote, highest allowed order is actor+1; otherwise current target. With Demote, lowest is the actual last rank; otherwise current target. Self/actor-or-higher targets and destinations, rank0 and out-of-range choices cannot dispatch. Lua display ranks1..N map to the existing wire0..N−1 without shifting the destination twice.
- [x] Selecting current rank sends nothing. Changed selection captures the original target name in the action and emits one `SetMemberRank`, never chained Promote/Demote commands. Displayed rank remains authoritative until reply; pending controls cannot submit another mutation. Escape closes only the menu, without a write.
- Named exclusions: `GUILD_RANK_ASSIGNMENT_AUTHENTICATOR_GAP` (no authenticator-backed eligibility/provider), `GUILD_RANK_ASSIGNMENT_LEADERSHIP_GAP` (direct assignment cannot transfer/disband; voluntary transfer has a [separate contract](guild-leadership-transfer.md)), `GUILD_RANK_ASSIGNMENT_CONFIG_EXCLUDED` (existing rank create/reorder controls stay unchanged). No eligibility flags are fabricated.
- Protocol/server/client land together. Backend chooses the direction right from requested/current rank and rechecks membership, self and strict hierarchy; direct TC rank updates do not append step lifecycle events. Model, real UDP and persistent backend proof precede the gate; native proof follows landing.
- [x] Post-landing private-realm acceptance proves both skins' actual4→1 assignment, current-selection/Escape/self controls, source menu above covered card labels, close/reopen and fresh server/client Officer readback. Eight inspected unchanged1920×1080 originals are published; [artifact provenance, exit qualifications and proof limits](../wiki/systems/guilds.md#native-acceptance-2026-10-18). Named exclusions above remain unchanged.

### Public Note sorting

- [x] Expanded roster Note header sorts only `GuildSnapshot.public_note`, never officer text. Mainline `GUILD_COLUMN_INFO[6].attribute=memberNote` and `CommunitiesMemberList.lua:798–925` define the comparison and direction: first Note click descending by the case-insensitive comparison, repeat ascending; switching back from another column resets descending. Existing header geometry and other column sorts stay unchanged.
- [x] Absent profile/nil note sorts first descending and last ascending. A present empty public note is a string, not missing metadata. Equal mapped notes invoke the original-direction base comparison even when Note direction changes; never reverse the entire sorted vector.
- [x] **Named gap `GUILD_ROSTER_CLUB_FIELDS_ABSENT`:** base order uses only represented fields: online→offline→unknown profile, both-known level descending, known guild rank ascending, raw character name ascending. Mainline `CommunitiesUtil.lua:53–78,172–182` also has six-way presence, Club role and memberId; those remain absent/unclaimed, with no protocol widening or fabricated values.
- **Named project policy `GUILD_NOTE_COMPARE_STORM_HISTORICAL`:** one replaceable function implements the approved laptop clean-room prose. ASCII a–z→A–Z; U+00E0–U+00FE subtract0x20 (including ÷→×, excluding ÿ); œ→Œ; Cyrillic U+0430–U+044F→uppercase and U+0410–U+0415/partners subtract1, while Ё/ё→U+0415; values above U+FFFF→U+FFFD. Lexicographic mapped values, shorter prefix first, no case-sensitive secondary comparison, normalization, expansion or other case mappings. Provenance labels C15595/W12340 Storm; **Lua binding and Retail70448 parity remain unverified**. Historical count limit and invalid-UTF8 decoding are unmodelled; Rust strings are valid UTF8. This supersedes the never-shipped ASCII placeholder, not a claim of Retail Unicode collation.
- [x] Filtering precedes Note sorting and row actions retain authoritative names/ranks. Gate529e is green. Post-landing actual owner public-note writes `alpha`/`Zulu`, `RAID`/`raid` and `é`/`É` prove both-skin physical direction/reset/filter/identity and unchanged online-owner tie order in both directions. Fresh server/client processes read exact persisted accented notes and tie order with identical artifacts. Eight inspected unchanged1920×1080 originals published. Native proves two real members/presence ties; remaining base keys and twelve historical mapping cases retain model proof, not a Retail oracle claim.

### Show Offline policy and preference

- [x] Expanded Roster alone exposes `CommunitiesMemberList.ShowOfflineButton`: Mainline `CommunitiesMemberList.xml:282–296`, inherited32×32 `UICheckButtonTemplate`, label RIGHT−2/GameFontNormalSmall; bottom-left sits26px above the MemberList top. The checked label is installed GlobalStrings40817, **Show Offline Members**. Modern and Forever share the same control/behavior.
- [x] Unchecked shows only members whose authoritative profile says `is_online=true`; offline and unavailable/unknown presence are excluded, matching `CommunitiesUtil.lua:136–146`. Filtering precedes existing sorting and never changes authoritative member/rank identities. Scroll extent/clamping follows the filtered list.
- [x] Preference persists as `communitiesShowOffline` in existing client options and reloads on fresh sessions, preserving other options. Source: `CommunitiesMemberList.lua:463–464,575–583`. Other tabs have no checkbox or new compact-list behavior.
- **Named project policy:** `COMMUNITIES_SHOW_OFFLINE_DEFAULT_ON = true`. This is an explicit World of Osso decision, **not a verified Retail default**. No authoritative Retail CVar declaration was found in the local source cache. [WoWUIBugs issue883](https://github.com/Stanzilla/WoWUIBugs/issues/883) reports true before saved settings load on a **Classic2.5.6** client; that indirect evidence does not establish Retail authority. Missing fields use this project policy; saved false remains false.
- [x] Post-landing native acceptance on master8b4c56e65: real invitation/Join seeds a member who then disconnects; actual owner query in both skins proves checked default, physical show/hide, source checkbox rectangle and filtered-row detail identity. Fresh server/client processes reload saved false in both skins from the same isolated options directory. Six inspected unchanged1920×1080 PNGs published; identical engine/server/officer hashes across all three stages. Native roster has two real members;40→20 overflow and unknown-presence coverage remain model proofs.

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
- [x] Decline uses the source140×22 button with right/bottom24px insets (local146,142 in the310×188 popup); Join stays110×22 at24,142. Both-skin native rectangles and physical Decline/pending pacing pass. Compared with unchanged ce574 before controls, each skin changes1703 RGBA pixels, zero outside the button paint rectangle plus4px margin. Four1920×1080 before/after PNGs inspected2026-10-15; offline fixture authority only.
- [x] `/gmotd` / `/guildmotd` sends SetGuildMotd; authoritative profile replies update MOTD and info.
- [x] `/ginfo` / `/guildinfo` sends a read-only query and reports the persisted creation date for charter-created guilds; legacy guilds remain explicitly unknown, never backdated.
- [x] Communities exposes Invite Member and Info; MOTD/info editors send authoritative writes, keep displayed text unchanged pending reply, and hide edits without rank permission. Member menu Remove uses strict hierarchy.
- [x] Native Info editor uses TextEdit for multiline text (499 characters); MOTD uses Enter-to-accept input (255 characters). Native postsetup supplies limits before projection.
- [x] MOTD input preserves typed order and caret position across projection updates, including insertion in the middle of existing text.
- [x] Authenticated native multiline editor and authoritative readback inspected; both-skin profile and restart preserve both lines.
- [x] GuildInviteFrame uses its inherited TranslucentFrameTemplate: black0.9 background inset10px, four32px DialogFrame corner quadrants and tiled16px border strips at the source anchors. Join/Decline/pending state and existing content remain unchanged. Two focused controls and both-skin native source rectangles/physical Join/Decline/pending pacing pass; four1920×1080 before/after PNGs inspected2026-10-15. Offline fixture authority only.
- [x] GuildInviteFrame renders the inviting guild's actual optional registered tabard from GuildInvitation in both skins: source GuildExtra background/ring, GuildFrame background/border and large emblem sheet/palette. No design uses GuildUtil's explicit no-logo branch, never an invented registration. Real no-design and registered owner packets, four actual invitations/four physical declines and identical-binary both-skin native source rectangles pass; four inspected1920×1080 originals published.
- [x] Invitation text uses the source TOP-relative anchors: inviter28, invitation38, guild name50; inherited Roman Small10 and Normal12 fonts. Guild name measures intrinsically at the centered TOP anchor, not an invented28px-high cell over the crest. Native overlap→source-render RED→six focused controls GREEN; actual native fonts, top anchors, centered intrinsic text and non-overlap pass in both optional states/skins.
- [x] Invitation display expires strictly after60 elapsed displayed seconds (`>60`, not `>=60`); Escape dismisses it even without an open Communities panel. Both send the existing authenticated decline exactly once. Loading teardown preserves the invitation and pauses displayed elapsed time; a newly delivered invitation resets elapsed time. Pending Join retains its existing authoritative acceptance pacing and never emits a timeout/Escape decline. Source: Mainline `GuildInviteFrame.xml:140–158`, `StaticPopup.lua:781–795`. Model boundary RED→10focused GREEN, independent gate503e15+68/fmt passes. Post-landing native both-skin physical Escape, natural timeout, real Join/removal and explicit Decline pass; fresh server/client process retains neither invitation nor membership. Eight inspected1920×1080 originals published; exact strict boundary is model-proven, not a native packet-to-hide timestamp claim.
- [ ] Achievement points and reputation warning data remain unavailable; no values are fabricated. Other source content geometry and server cancellation notifications remain separate.

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

- `guild_invitation_source_text_top_anchors_and_fonts_stay_above_tabard_both_skins` plus the five existing invitation render/action controls; `godot/tests/guild_invitation_tabard_live.gd` proves real received optional data, exact snapped art/text anchors and physical decline in both skins. [Native provenance and boundaries](../wiki/systems/guilds.md#native-invitation-tabards).

- `godot/tests/communities_fidelity.gd`: source rectangles, physical wheel/thumb reaching member32, name sorting, member hierarchy menu, all four source tabs and invitation reducer in both skins; fixture authority only, not a live-server/deployment claim.
- `godot/tests/communities_roster_sort.gd`: physical Level/Class ascending/descending clicks, numeric class ordering, unchanged40/45px headers and sorted-row context identity in both skins. `guild_rank_widgets::guild_communities_metadata_columns_sort_numeric_values_and_preserve_member_identity` additionally covers distinct numeric level/class order, missing metadata and unchanged authoritative state; seven focused Communities controls pass.
- `godot/tests/guild_invite_decline.gd`: actual source Decline/unchanged Join rectangles and physical no-duplicate pending Decline in both skins. The source-size render-contract RED observes110×22 instead of140×22; all three invitation controls pass after the correction.
- `godot/tests/guild_invite_chrome.gd`: both-skin310×188 popup, source inset/eight border-piece rectangles, physical Join/Decline and no duplicate request while pending. `guild_rank_widgets::guild_invitation_uses_source_translucent_dialog_chrome_in_both_skins` checks the emitted source texture/UV/tiling/color contract; the existing invitation action control passes unchanged. Debug-only fixture supplies the payload, not a live server.
- `godot/tests/guild_control_fidelity.gd`: both-skin1920×1080 native source rectangles, physical menus/rename/id22/gold/tab edits, pending-authority pacing and eighth-tab overflow. Fixture replies only; real authority/persistence are asserted by server tests.
- Mainline Discord officer bullet GlobalString is absent from the current canonical strings; its text is not fabricated. Tab icon and bank-purchase payloads are absent; those source features remain named gaps.

- `godot/tests/guild_member_controls_live.gd`: native right-click menu is inside the viewport; Promote/Demote alter and restore the dedicated authoritative Rank label. The helper now reads that field rather than a combined Name/Rank label; no new live-server execution is claimed for the Communities port.
- `godot/tests/guild_profile_live.gd`: private guild master types MOTD, inserts before the final character, accepts and checks the authoritative displayed reply; real Godot input/projection boundary.

- `godot/ui-model/tests/native_guild_ranks.rs`: 5 authoritative model/Officer tests pass.
- `godot/ui-model/tests/guild_rank_widgets.rs`:16 tests cover both-skin controls, including
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
