# Group Frames

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Party and raid play against the live server group backend: the invite flow, raid-style party frames and raid frames with live member state, leadership, conversions, roles and ready checks.

- Contract: shared-protocol `protocol/group_messages.rs`, plus `GroupRosterSnapshot` and `GroupCommandResponse` in `protocol_snapshots.rs`.
- Server rules: game-server `crates/server/src/group/`.

References (under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`):
- CUF = `Blizzard_UnitFrame/Shared/CompactUnitFrame.lua` / `.xml`
- CRG = `Blizzard_UnitFrame/Shared/CompactRaidGroup.xml` / `.lua`
- EMP = `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`
- RC = `Blizzard_FrameXML/Mainline/ReadyCheck.xml` / `.lua`
- UPB = `Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua`
- Strings: GlobalStrings, build 12.1.
- Atlas rects: `UiTextureAtlasMember`, build 12.1.0.69933.

## What it must do

### Data
- [x] The server sends `GroupMemberStates` for **every** online member, in interest range or not, at most every 0.2 s and only for changed members. This is the only source of the frames' live data.
  - Each state carries: health and max health, primary power (`PowerEntry`), `DeathState` (Alive/Dead/Ghost), `Position`, and harmful visible auras (`AuraView`).
  - A connection's first update after login, relog or joining carries every member.
- [x] The roster (`GroupRosterSnapshot`) is resent on join, leave, promote, conversion, role, loot, ready check result, login/logout/relog, and level change.
- [x] Client `GroupState` holds the roster, live states by name, the ready check and the pending invite.
  - Live state is dropped for members who go offline or leave.
  - An empty roster ends the ready check.

### Invite
- [x] Ways to invite: `/invite name` or `/inv name` (SLASH_INVITE), the target or group frame right-click entry "Invite" (`PARTY_INVITE`), or IPC `group invite`.
- [x] `GroupInvitePrompt` opens the `PARTY_INVITE` static popup "%s invites you to a group." (`INVITATION`) with Accept / Decline and a 60 s timeout (`StaticPopupTimeoutSec`).
  - Accept sends `RespondGroupInvite { accept: true }`.
  - Decline, Escape and timeout send `accept: false`, as the popup's `OnHide` does.
- [x] `GroupInviteCancelled` closes the popup. The server sends it when:
  - the invite expires (the inviter gets "%s declines your group invitation.");
  - the inviter logs out, joins another group, leaves the group or loses leadership.
  - This also fixes stale invites: accepting one can no longer join a group the inviter does not lead.
- [x] Server rejections use Retail `ERR_*` text, shown as system chat lines: BadPlayerName, InviteSelf, AlreadyInGroup, InvitePartyBusy, NotLeader, GroupFull, NotInGroup, TargetNotInGroup, NotInRaid, ReadyCheckInProgress.

### Party frame (raid-style `CompactPartyFrame`)
- [x] Raid-style party frames are the project default. This is a user decision: Retail's own default is `UseRaidStylePartyFrames = 0` (EMP:276), whose classic frame needs a portrait.
- [x] Placement: `CompactPartyFrame`, 98 × (14 + 5×44), at the Retail Modern preset: TOPLEFT on `CompactRaidFrameManager`'s TOPRIGHT at (0, −7) (EMP:290-295). The 222 × 140 manager starts collapsed at UIParent TOPLEFT (−200, −140) (Blizzard_CompactRaidFrameManager.xml:113, .lua:93, :335), so the frame's top-left is (22, −147) from UIParent's top-left and it grows downward.
  - Title "Party" (`PARTY`, `GameFontNormalSmall`, CRG title 50×14).
- [x] Members `CompactPartyFrameMember1..5`, each 98×44: the Edit Mode default 72×36 + (26, 8) (EMP:280-281).
  - Order: the player first, then the others in roster order (`CRFSort_Group`).
- [x] Member frame, Legacy layout (`DefaultCompactUnitFrameSetup`, CUF:1941-2069):

  | Element | Art / source | Placement and behaviour |
  |---|---|---|
  | Background | `raidframe-hp-bg-white` (7658229) | Whole frame |
  | Health bar | `RaidFrame-Hp-Fill` (7539072) | Inset 1; class colour (`RAID_CLASS_COLORS`) |
  | Power bar | `_RaidFrame-Resource-Fill` (7539067) over `_RaidFrame-Resource-Background` | 8 px, `PowerBarColor` (PowerBarColorUtil.lua:18-33) |
  | Role icon | `UI-LFG-RoleIcon-<Role>-Micro-GroupFinder` (5171843) | 17×17 at 3,2; hidden for no role |
  | Name | `GameFontHighlightSmall`, white | From the role icon's right edge |
  | Status text | `GameFontDisable` × component scale | "Offline" (`PLAYER_OFFLINE`) or "Dead" (`DEAD`); ghosts show Dead too (`UnitIsDeadOrGhost`, CUF:1100-1105) |
  | Selection highlight | `RaidFrame-TargetFrame` (7526019) | When the member is the current target |
  | Out of range | frame alpha 0.5 (CUF:1071) | Beyond 40 yd of the player, by the members' state positions |
  | Debuffs | spell icon + `UI-Debuff-Overlays` (130759) border tinted by `DebuffTypeColor` | Up to 5, 11 px, bottom-left above the power bar, 3 per row |

  - Offline members: grey health and power bars (CUF:656-658).
  - Before a member's first live state arrives, its bars are empty; no health is invented.
- [x] Left-click targets the member when their unit is replicated. Right-click opens the member menu.

### Static portrait party component (native, step 1)
- [x] An unwired `PartyFrame` screen renders up to four non-self member views: name/class colour, health and power fractions, leader/guide, assigned role, Offline/Dead and portrait slots. Modern uses Retail Party art; Forever deliberately selects Camelot's conditional CharacterFrameOnParty art. [Source geometry and verified set-1 atlas relation](../wiki/systems/portrait-party-frames.md).
- [x] Optional static party pets use Retail's half-scale member art and health-only frame; `show_pets` controls visibility and container spacing.
- [ ] Raster-correct static portrait family under both skins: offline four-member art/fills pass with local Retail and product-bound Forever sheets. Whole-family/reference parity and optional pet raster acceptance remain unproven; [bounded proof and exclusions](../wiki/systems/portrait-party-frames.md#acceptance-status--bounded-offline-artfill-pass-2026-10-05).
- [x] Connect portrait member names/class colour, health/power fractions and power type, leader, role, Offline/Dead to the existing group roster/live states. Exclude self; retain roster order and compact default. `LayoutSettings.use_raid_style_party_frames = Some(false)` selects portraits; absent/true selects compact. Raid presentation is unchanged. Settings UI remains step 4.
- [x] Offline health fills to maximum and desaturates; offline power fills to maximum and receives Retail's half-grey vertex tint, retaining the sampled atlas hue (not power desaturation). Crown/guide keep Retail's BOTTOM→TOP (-10,-6) anchor, not TOPLEFT; their above-frame extension is intentional. [Exact source rules and parity audit](../wiki/systems/portrait-party-frames.md#retail-parity-audit--2026-10-07).
- [x] Runtime portrait bindings (step 3): non-self roster names bind the existing masked M2 renderer to the four party slots under both skins. Leave/compact/raid/scene exit cancel pending loads and free owned viewports; reorder reuses surviving member resources when hosts survive. Every member's head comes only from `GroupMemberSnapshot.portrait` (race, canonical customization/visage and equipped head), including never-replicated, distant, cross-zone and offline members. Roster helm changes rebuild the portrait. Offline desaturates that roster head; dead/ghost/low-health tints follow Retail, with no range-driven portrait fade. Compact default renders no portraits; no replicated-appearance fallback. Presentation settings and live acceptance remain separate work.
- [ ] Live portrait-family acceptance. Static registry golden and offline preview are not live proof.

### Party presentation settings (step 4)
- [x] Options → HUD → Layout Settings → Party exposes Retail's complete Party setting inventory in both skins, plus the requested Show Pets checkbox. [Source/default/range table](../wiki/systems/party-edit-mode-settings.md).
- [x] Save each change through existing per-character `ui_layout.ron` Edit Mode layouts. Keep compact on and 98×44 unless explicitly overridden. Reset restores the preset. No drag-and-drop or frame movers.
- [x] Size, horizontal layout, background, border, sort and pets affect the active family. Compact aura organization, opacity, frame scale and icon size settings change rendering when their data is available.
- [x] Bounded step-4 offline proof: 1 native + 10 UI-model tests, both-skin settings/portrait captures, six real-engine cases per skin including actual border pixels, sorted head binding and viewport cleanup. [Evidence and exclusions](../wiki/systems/party-edit-mode-settings.md#bounded-verification--2026-10-06).
- [ ] Complete step-4/live acceptance: big-defensive classification remains blocked below; no full reference-image or clean global shutdown proof.
- [ ] Live big-defensive classification: existing AuraView/spell catalog lacks Retail's classification. Do not fabricate a defensive buff. Group-only updates also lack buffs/pets outside replication interest.

### Raid frame (`CompactRaidFrameContainer`)
- [x] Placement: 8 group columns × 5 at the native 72×36 = 576 × (14 + 180), centred above the cluster, bottom 215.
- [x] Group `n` sits in column `n` (subgroups 1–8, 5 each; `MAX_RAID_GROUPS`). Members fill it in roster order.
  - Empty groups are not drawn. Each drawn group has the title "Group n" (`GROUP_NUMBER`).
- [x] Frames `CompactRaidGroup{g}Member{m}` use the same member frame as the party.
- [x] Both containers are Edit Mode elements `party_frames` / `raid_frames` (`CompactPartyFrame` / `CompactRaidFrameContainer`, `HudAnchor::Bottom`).

### Menus (Retail `UnitPopup`)
- [x] Self (player frame or own group frame): "Set Role: Tank/Healer/Damage/None", then for the leader:
  - "Convert To Raid" (`CONVERT_TO_RAID`) in a party;
  - "Convert To Party" in a raid of at most 5;
  - "Ready Check".
  - Then "Leave Party" (`PARTY_LEAVE`).
- [x] Another player who is not in the group: "Invite" when ungrouped or leader.
- [x] Another member, leader only: "Promote to Leader" (`PARTY_PROMOTE`), "Uninvite" (`PARTY_UNINVITE`) and the role entries.
- [x] Group frames also offer Target and Inspect when the member's unit is replicated.

### Leadership and conversion
- The leader can move a raid member to subgroup 1–8 with `game-engine-cli group subgroup --name <member> --subgroup <1..8>` (`SetRaidSubgroup`). Frames follow the authoritative roster; permissions and full-group rejection remain server-owned.
- [x] Promote sends `PromoteGroupLeader`. The roster moves the leader, and "%s is now the group leader." is shown.
- [x] Convert to raid sends `ConvertGroupToRaid` ("Party converted to Raid"). Convert to party sends `ConvertGroupToParty`: all members go to subgroup 1, "Raid converted to Party".
  - A raid of 6 or more is rejected with `ERR_GROUP_FULL`.
  - A party is rejected with `ERR_NOT_IN_RAID`.
- [x] Leave sends `LeaveGroup`. Leadership passes to the first remaining member; a group left with 1 member disbands.

### Ready check
- [x] Start it with `/readycheck` or `/rc` (SLASH_READYCHECK) or the self-menu entry. The server marks the leader Ready; the timeout is 30 s.
- [x] `ReadyCheckFrame` (323×100 at CENTER 0,−10, RC) shows "%s has initiated a ready check." (`READY_CHECK_MESSAGE`) with Ready / Not Ready buttons (119×24). It shows only to members who still have to answer, not the initiator.
  - Buttons send `RespondReadyCheck`.
- [x] Member frames show `UI-LFG-ReadyMark/PendingMark/DeclineMark-Raid` at 20 × component scale, BOTTOM 0,h/3−4 (CUF:2038-2041).
  - After the check ends, members still waiting show Not Ready (`CompactUnitFrame_FinishReadyCheck`).
  - The marks clear 11 s later (`CUF_READY_CHECK_DECAY_TIME`).
- [x] "Everyone is Ready" / "Ready check finished" go to chat.

### Chat commands
- [x] `/invite` `/inv`, `/uninvite` `/un` `/u` `/kick`, `/promote` `/pr`, `/readycheck` `/rc`.
  - Retail has no slash command to leave a party: `/leave` is a chat channel command.

### Not implemented
- [ ] Party portraits for members never replicated (roster lacks race/customization/equipment), party-pet data outside replication interest and live acceptance; runtime member portraits and replicated pet health wiring exist above. `CompactRaidFrameManager` (the side panel with ready check, role poll and markers).
- [ ] Assistant / main tank / main assist.
- [ ] Group persistence across server restarts.
- [ ] Loot method wiring: the Loot Rules window stays client-side.
- [ ] Features that need data the client does not have:
  - Aggro highlight: needs threat data.
  - Heal prediction and absorbs.
  - Buff icons outside replication interest; group-only states carry harmful auras. Visible replicated buffs are rendered.
  - Dispellable-type icons at top-right: dispel ability is not known client-side.
- [ ] Leader icon: Retail CUF has none (CompactUnitFrame.xml; `DefaultCompactUnitFrameSetup`, CUF:1941-2069); only the classic `PartyMemberFrame` draws `UI-HUD-UnitFrame-Player-Group-LeaderIcon`.
- [ ] Targeting a member outside replication range: no client entity exists.
- [ ] Role enums: `shared::group::GroupRole`, `GroupRoleSnapshot` and `class_spec::Role` are still three separate types.

## Verified scope

- [x] Live (2026-09-25), three headless clients on an isolated server (:5060); evidence in `data/diagnostics/groups-20260925/` (final run at the top level, driver `run.sh`, scripts `a.js`/`b.js`/`c.js`):
  - `/invite` from IPC → `PARTY_INVITE` popup on Partyb (01), clicked Accept → party of two and three (02, 03).
  - Partyb targeted Partya by clicking her party frame (selection highlight, 05-b).
  - Partya dead at the graveyard ~60 yd away: "Dead", alpha 0.5 on b/c (04).
  - Partyc's fall damage 153 → 13 shown live on a/b (05).
  - Convert To Raid from the player frame menu → raid grid (06).
  - Ready Check from the menu → ReadyCheckFrame on b/c (07); answers Ready / Not Ready drawn as marks (08).
  - Promote from the raid frame menu (09); Partyc left via the player frame menu (10).
- [x] Out of interest range: in `run7-white-background/04-out-of-range-*`, Partya ~103 yd away (beyond the 100 yd interest radius) kept her live health and position on b/c.

- [x] Godot client, live (2026-09-30), two headless clients on a private server (UDP 5103); captures in `data/diagnostics/showbugs-2026-09-30/group/`: Fbshowbugs types `/invite Fbshowwar`; Fbshowwar's `PARTY_INVITE` popup (invitee-00) is clicked Accept; both show the party at the top left, the player first, with live health and mana (`inviter-01`, `invitee-01`); `/leave` removes it on both (`*-02`). Not covered live in Godot: target highlight, offline/dead, member menus, raid frames, ready check frame, frame clicks.

## Tests asserting this spec

- shared-protocol:
  - `src/group_tests.rs`: `Raid::new`, `collapse_to_party` including its 6-member rejection.
  - `src/protocol_group_tests.rs`: registration, `GroupMemberStates` wire round-trip, `ERR_RAID_CONVERTED_TO_PARTY`.
- game-server `crates/server/src/group/tests.rs`:
  - invite/accept/decline;
  - stale invites cancelled on join/leave/promote/logout/expiry;
  - convert to party (5 → ok, 6 → GroupFull, party → NotInRaid);
  - rejections;
  - ready check timeout with away members;
  - level-up resend;
  - member states carrying health/power/ghost/debuffs, sent only on change, resent after relog.
- game-engine:
  - `godot/ui-model/src/game/group_state_tests.rs`: roster/live-state lifecycle (a state ahead of its roster is kept until the roster decides), ready marks and decay, menu entries by role.
  - `godot/network/src/wire_tests.rs::native_bridge_receives_group_messages_in_channel_order`.
  - `godot/rust/src/party_frames.rs`: party order, live bars, Dead/Offline, range, target highlight; invite popup accept/cancel/timeout; `portrait_party_roster_tracks_join_leader_offline_death_and_leave` uses concrete roster transitions and inspects rendered registry names/class colours/bars/leader/status.
  - `godot/ui-model/tests/portrait_party_frame.rs`: compact-default/portrait selection and unchanged raid visibility under both skins, source-cited offline bars and crown rect.
  - `godot/rust/src/ui/parts_tests.rs::portrait_party_desaturated_health_projects_sampled_image_desaturation`: sampled-image desaturation reaches native shader rather than greying only vertex tint.
  - `godot/rust/src/chat_tests.rs::group_commands_become_group_requests`.
  - `src/game/networking/group_tests.rs`: inbox handling, invite cancel, chat lines, commands reaching the worker.
  - `src/scenes/group_frames/tests.rs`: frames, placement and clicks:
    - party order, bars, class colour, range, dead/offline, selection, debuffs;
    - raid columns and ready marks;
    - party placement;
    - click to target, menu Promote / Convert / Leave, ready check buttons;
    - `PARTY_INVITE` accept, timeout and cancel.
  - `src/rendering/ui/unit_frames.rs`: the target frame Invite entry and the player frame Leave / Convert entries.
  - `godot/ui-model/src/ui/chat_frame_tests.rs`: slash commands.
