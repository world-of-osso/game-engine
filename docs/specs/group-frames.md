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

### Ordinary PARTY_INVITE body shadow (bounded slice)
- [x] Ordinary StaticPopup body inherits black shadow at source (+1,−1), projected native (+1,+1), in both skins. Preserve current 14px FrizQuadrata, white text, geometry, Accept/Decline actions and timeout. [Bounded supplied-PARTY_INVITE proof](../wiki/systems/group-frames.md#ordinary-staticpopup-body-shadow--bounded-proof-2026-10-17).
- [ ] Effective user-font scaling and full-popup reference parity remain unqualified; supplied-state shadow proof does not qualify server/lifecycle behavior.

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
- [x] Runtime portrait bindings (step 3): non-self roster names bind the existing masked M2 renderer to the four party slots under both skins. Leave/compact/raid/scene exit cancel pending loads and free owned viewports; reorder reuses surviving member resources when hosts survive. Every member's head comes only from `GroupMemberSnapshot.portrait`: canonical race/customization/visage/head equipment for players, or the authoritative creature display ID for NPC followers, including never-replicated, distant, cross-zone and offline members. Creature heads use the existing authored NPC appearance renderer, never invented player customization. Roster helm changes rebuild the portrait. Offline desaturates that roster head; dead/ghost/low-health tints follow Retail, with no range-driven portrait fade. Compact default renders no portraits; no replicated-appearance fallback. Presentation settings and live acceptance remain separate work.
- [ ] Live portrait-family acceptance. Static registry golden and offline preview are not live proof.

### Party presentation settings (step 4)
- [x] Options → HUD → Layout Settings → Party exposes Retail's complete Party setting inventory in both skins, plus the requested Show Pets checkbox. [Source/default/range table](../wiki/systems/party-edit-mode-settings.md).
- [x] Save each change through existing per-character `ui_layout.ron` Edit Mode layouts. Keep compact on and 98×44 unless explicitly overridden. Reset restores the preset. No drag-and-drop or frame movers.
- [x] Size, horizontal layout, background, border, sort and pets affect the active family. Compact aura organization, opacity, frame scale and icon size settings change rendering when their data is available.
- [x] Bounded step-4 offline proof: 1 native + 10 UI-model tests, both-skin settings/portrait captures, six real-engine cases per skin including actual border pixels, sorted head binding and viewport cleanup. [Evidence and exclusions](../wiki/systems/party-edit-mode-settings.md#bounded-verification--2026-10-06).
- [ ] Complete step-4/live acceptance: big-defensive classification remains blocked below; no full reference-image or clean global shutdown proof.
- [ ] Live big-defensive classification: existing AuraView/spell catalog lacks Retail's classification. Do not fabricate a defensive buff. Group-only updates also lack buffs/pets outside replication interest.

### Raid frame (`CompactRaidFrameContainer`)
- [x] Placement: 8 group columns × 5 at the native 72×36 = 576 × (14 + 180). Both presets use Retail Mainline `EditModePresetLayouts.lua:315-320,1216-1221`: TOPLEFT on collapsed `CompactRaidFrameManager` TOPRIGHT (0, −5), resolving to UIParent TOPLEFT (22, −145). Grows downward; independent of player-frame geometry and explicit UI scale. Saved custom mover placements remain unchanged.
- [x] Group `n` sits in column `n` (subgroups 1–8, 5 each; `MAX_RAID_GROUPS`). Members fill it in roster order.
  - Empty groups are not drawn. Each drawn group has the title "Group n" (`GROUP_NUMBER`).
- [x] Frames `CompactRaidGroup{g}Member{m}` use the same member frame as the party.
- [x] Both containers are Edit Mode elements `party_frames` / `raid_frames` (`CompactPartyFrame` / `CompactRaidFrameContainer`), anchored TOPLEFT by default.

### Raid default overlap regression (2026-10-09)

- [x] Explicit UI scale 1: concrete Mailalpha/Mailbeta two-member raid and player rectangles do not intersect under either preset at 1920×1080 and 1280×720. RED reproduced Forever at 1920×1080: raid (672,779,576,86), player (510,780,240,60). Implicit 768-unit UIParent scaling concealed the defect.
- [x] Targeted native HUD layout tests: 31 passed. Offline native previews at 1920×1080 draw both named members and player under Forever/Modern, assert non-overlap, and retain PNGs at `/syncthing/AgentShared/2026-10-09/raidoverlap/{forever,modern}.png`. Evidence/logs: `data/diagnostics/raidoverlap-2026-10-09/`. No new live-network or full HUD-editor acceptance claim.

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
- [x] Native assistant assignment/revocation and leader/assistant ready-check/countdown permissions are server-authoritative; raid members cannot initiate either.
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
  - [x] Difficulty-qualified body follows `ReadyCheck.lua:31–48`: use the server's actual nonzero `WorldServerInfo.difficulty_id`, else its selected raid/dungeon difficulty. Append `\nRaid Difficulty: <CURRENT name>` only when local `Difficulty.csv.ToggleDifficultyID > 0`; names come from `Name_lang`, never the toggle target. Native client forwards/stores the existing message and clears copy context on transfer/reconnect. No protocol/server change; geometry, audio, answers and authority unchanged. Account rejection and real-UDP missing-forwarding REDs precede account1/wire1/UI8 GREEN (including both skins); engine/network check and Cargo fmt pass. [Bounded evidence](../wiki/systems/group-frames.md#readycheck-difficulty-qualified-body--implementation-2026-10-18); Native text assertions now pass both skins, the source-equivalent residual is **`READY_CHECK_MULTILINE_BODY_BUTTON_COLLISION_SOURCE_EQUIVALENT`**, not a source-justified geometry defect: the second line crowds/collides with the response-button tops. Conserved bounds are not readability acceptance. Retail uses intrinsic message height (`y=0`), not the captured binary's fixed28; no sourced frame growth or text-relative button anchors were found. [Native gap/source audit](../wiki/systems/group-frames.md#readycheck-native-difficulty-text--layout-gap-2026-10-18). Authenticated live-server acceptance remains unqualified.
  - [x] ReadyCheckListenerFrame false→actually-shown edge plays `SOUNDKIT.READY_CHECK`8960 through existing UiKits (70448 SoundKit gain1, one567478 Ogg). No replay on reliable/snapshot refresh; no sound for initiator/hidden/missing listener. Hide/scene exit rearms; record visibility before attempting audio so a decode failure does not retry every frame. Preserve visuals, authority, response actions and countdown audio. [Implementation/source evidence](../wiki/systems/group-frames.md#readycheck-listener-show-sound--implementation-2026-10-17).
  - [x] Bounded native decoded-dispatch acceptance: both skins, ten checkpoints each, listener/reopen only (two starts), actual Ogg567478 playing/duration5.020385265s/gain1/pitch1, natural completion and zero-player cleanup. Debug fixture uses production projected visibility/edge helper and real UiKits; not authenticated GameClient/server or hardware-audibility proof (Dummy audio). [Exact revisions and retained receipts](../wiki/systems/group-frames.md#readycheck-listener-show-sound--implementation-2026-10-17). Lead independently published/verified the Ogg; invalid flat-path model index withdrawn. UI audio ships through `sounds/`, not model indexes ([bundle contract](player-bundle.md)). FlashClientIcon/response-click audio remain outside this slice.
  - [x] ReadyCheck body explicitly uses source line spacing0 (`UI.xsd:762–769` default; no override in GameFontNormal/Roman font chain), in both native intrinsic measurement and drawing. Preserve font12, wrapping/no line cap, anchors/frame/buttons; other FontStrings retain their existing host theme. Missing-attribute RED→core3/Godot8/UI8 GREEN, engine/network/core check and both Cargo fmt scopes pass; Lead accepted native measurement on landed engine5d055f34d/toolkitdbbfda7: both skins11 phases verify spacing0 and body heights15/30 UI. Two-line glyph clearance−10.875px, shadow/combined−12.875px; ordinary combined+9.125px. This is measurement acceptance, **not layout acceptance**. Both named gaps remain open. [Exact native evidence](../wiki/systems/group-frames.md#readycheck-explicit-source-line-spacing--2026-10-18).
  - [ ] `FONT_LINE_HEIGHT_PARITY`: adopted historical raster contract now resolves17px lines/14px baseline at1080,p=1.40625; actual four-case native character-advance proof passes. Retail70448 ink equality is unverified. `READY_CHECK_MULTILINE_BODY_BUTTON_COLLISION_SOURCE_EQUIVALENT` preserves Retail's source geometry, whose34px allocation extends1.65625px into the buttons; no source-justified code fix. A real same-case Retail capture is required to compare glyph/shadow ink. [Source-equivalent audit](../wiki/systems/group-frames.md#readycheck-source-equivalent-multiline-overlap--2026-10-18).
  - [x] Listener body uses RC35–39's240px width, **intrinsic height** (`y=0`), TOP(+20,−37) and MIDDLE justification with Roman Friz12/shadow. Native host measurements determine height, not a fixed28px box or forced fitting. Frame remains100 high and response buttons remain bottom-anchored at y60. No frame growth, button move, font shrink or invented spacing. Host-measurement RED→native8/UI8 GREEN, engine/network check and Cargo fmt pass. Native clearance is measured in both skins; the source-equivalent residual above preserves original geometry rather than inventing clearance.
  - Listener message retains RC's240px width and TOP(+20,−37) origin, resolving to(61.5,37). Title height must not shift this parent-relative anchor. The earlier bounded origin proof preserved then-current text/font/color/28px height; the intrinsic-height contract above supersedes that height, not its parent-relative anchor. Historical captures do not qualify intrinsic sizing, shadow, wrapping or difficulty-content parity.
  - [x] Listener message retains the inherited Roman Friz12 black shadow at source(+1,−1) in both skins, at the historical240×28 box/anchor/text/buttons checkpoint; current intrinsic body height is governed above. This shadow-only contract does not certify source `NORMAL_FONT_COLOR`, intrinsic height, title/portrait, difficulty content or lifecycle. [Bounded proof](../wiki/systems/group-frames.md#readycheck-message-shadow--bounded-proof-2026-10-16).
  - [x] Listener portrait area uses `ReadyCheckPortrait`54×54 at source TOPLEFT(0,+3), native(0,−3), and `PortraitFrameTemplate` metal chrome. Bind the actual initiator from `GroupMemberSnapshot.portrait` through the existing native renderer; no new protocol, static head or replicated-unit fallback. Missing roster/race data fails explicitly and clears stale resources. Preserve existing frame/message bounds, colours/shadows, buttons/actions.
  - [x] Post458e native both-skin1920×1080 portrait/metal proof through the typed Ann/Bob roster fixture and production selector/renderer: actual initiator head, existing controls, exact head RGBA restoration and zero portrait viewports after completion/fixture exit.54×54 UI units at HUD scale1080/768=1.40625 occupy75.9375px and render76×76. Head-only changes4353 Modern/3941 Forever, zero outside portrait. No UDP/server lifecycle, exact mask/camera or full-popup reference-parity claim.
  - [x] Title origin is(58,6), width241 per RC45–58 (container left58/right24/top1, child TOP5/LEFT/RIGHT). Conserve current20px title control height, font/colour/shadow and every other production field; this does not qualify intrinsic title height/wrapping/frame level. [Bounded implementation](../wiki/systems/group-frames.md#readycheck-title-originwidth--implementation-2026-10-17).
  - [x] Post474e native both-skin1920×1080 title241×20@(58,6), preserving all other fields. Against retained/hash-verified actual61ef old323×20@(0,4) captures:1899 Modern/1914 Forever RGB changes, zero outside old/new title-shadow union. Same-binary diagnostic is origin-only at retained241px width, not an old323px-width capture; exact source RGBA restoration passes. Earlier proofs retain original geometry scope; intrinsic title height/full reference/server authority remain unqualified.
  - [x] Title inherits Roman Friz12 opaque black shadow at source(+1,−1), preserving every other field. Title layout remains a named gap; the separate colour contract below does not broaden this shadow proof. [Bounded implementation](../wiki/systems/group-frames.md#readycheck-title-shadow--implementation-2026-10-17).
  - [x] Title colour uses inherited `NORMAL_FONT_COLOR`, GlobalColor row1 ARGB0xFFFFD200 / RGB8(255,210,0), changing only `TITLE_COLOR`. Preserve shadow/font/geometry/body/buttons/actions; title layout/global palette/full-popup/lifecycle remain separate gaps. [Bounded implementation](../wiki/systems/group-frames.md#readycheck-title-colour--implementation-2026-10-17).
  - [x] Post-gate447e both-skin1920×1080 exact-title-colour proof: RGBA8(255,210,0,255),192 green-only lightened pixels/skin, zero outside title box/red-blue changes/darkening; exact source RGBA restoration. Existing title-shadow proof retains its original binary/approximate-colour scope.
  - [x] Post-gate436e native both-skin1920×1080 title-shadow proof: WoW(+1,−1), y up, projects to Godot(+1,+1), y down.462 darkened pixels/skin, zero outside title/shadow union; exact source RGBA restoration. Not full-dialog or lifecycle acceptance.
  - [x] Listener MESSAGE colour uses inherited `NORMAL_FONT_COLOR`: GlobalColor row1 ARGB0xFFFFD200 / RGB8(255,210,0). Change only `TEXT_COLOR`; preserve all other production fields/behavior. Title/global palette, intrinsic metrics, portrait/art, difficulty content and lifecycle remain separate gaps. [Bounded model/native proof](../wiki/systems/group-frames.md#readycheck-message-colour--implementation-2026-10-17).
  - Response buttons retain RC's bottom anchors: Yes BOTTOMRIGHT → parent BOTTOM (+11,+16), No BOTTOMLEFT → parent BOTTOM (+22,+16), resolving to (53.5,60)/(183.5,60). Both skins use inherited UIPanel Up/Down/Highlight/Disabled sheets130828/130825/130826/130824 with fixed12px caps and95px center. Physical release outside cancels; release inside emits only the corresponding response action. This button contract does not establish portrait/border/title/body/font/sound or whole-dialog fidelity.
  - Both Modern and Forever must project the Retail dark dialog background (6839810), nine-slice border (6795680), title, initiator message and both buttons through the native HUD. Answer and server timeout remove the popup; Ready / Not Ready answers appear on the leader's member frames.
- [x] Member frames show `UI-LFG-ReadyMark/PendingMark/DeclineMark-Raid` at 20 × component scale, BOTTOM 0,h/3−4 (CUF:2038-2041).
  - After the check ends, members still waiting show Not Ready (`CompactUnitFrame_FinishReadyCheck`).
  - The marks clear 11 s later (`CUF_READY_CHECK_DECAY_TIME`).
- [x] "Everyone is Ready" / "Ready check finished" go to chat.

### Pull countdown (Retail PlayerCountdown)
- [x] `/countdown N` requests a server-authoritative group countdown; `/countdown 0` cancels. No duration is invented when N is absent.
- [x] Server start/replacement/cancellation/expiry messages reach every online group member; leaving clears the countdown. Use the authoritative UTC deadline on receipt, not a new duration from receipt time. Client/server wall clocks must be synchronized; no alternate timing path.
- [x] Both skins use Timer.lua's player countdown: bar above 30 seconds, medium numbers 11–30, large centred numbers 1–10; Timer.xml's digit pulse and native sound kits.
- [x] Raid Manager Countdown requests 10 seconds (`Blizzard_CompactRaidFrameManager.lua:1262`).
- [x] Leader/assistant permissions remain server-owned.
- [x] Timeout reports `RAID_MEMBERS_AFK` and `READY_CHECK_YOU_WERE_AFK` for unanswered participants, not explicit Not Ready responses.

### Chat commands
- [x] `/invite` `/inv`, `/uninvite` `/un` `/u` `/kick`, `/promote` `/pr`, `/readycheck` `/rc`.
  - Retail has no slash command to leave a party: `/leave` is a chat channel command.

### Not implemented
- [ ] Party portraits for members never replicated (roster lacks race/customization/equipment), party-pet data outside replication interest and live acceptance; runtime member portraits and replicated pet health wiring exist above. Remaining `CompactRaidFrameManager` controls (role poll, settings, filters, markers, restrict-ping and bottom controls); ready/countdown shell is implemented in the current cycle.
- [ ] Main tank / main assist.
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

### Private two-client acceptance — 2026-10-09

Forever, pinned Godot 4.7.2, cage/Vulkan, private UDP5500 and `fb_mailraid_*` accounts, Mailalpha (warlock9) / Mailbeta. Inherited extension built at `5ab533364`; checkout fast-forwarded to `709a62431` without group implementation changes; server `5b04aa931`. [Proof ledger](../../data/diagnostics/mailraidlive-2026-10-09/proof-ledger.txt), native input journal and paired PNG/JSON in `data/diagnostics/mailraidlive-2026-10-09/`; inspected downscaled captures copied full-size to `/syncthing/AgentShared/2026-10-09/mailraidlive/`.

| Case / governing rule above | Observed vs expected | Result / PNG stems |
|---|---|---|
| `/invite`, `PARTY_INVITE`, Accept | A types `/invite Mailbeta`; B sees named inviter and accepts; popup closes. | PASS `13-invite-b`, `14-party-1`, `14-party-2` |
| Compact party (`CRFSort_Group`) | Both show self first and the other member, full live health; warlock mana and warrior zero rage. | PASS `14-party-1`, `14-party-2` |
| Convert To Raid (`CONVERT_TO_RAID`) | Self-menu conversion gives both the same Group1 grid, Mailalpha then Mailbeta; compact party removed. | PASS `15-raid-1`, `15-raid-2` |
| Ready (`READY_CHECK_MESSAGE`, CUF ready marks) | A types `/readycheck`; B gets named initiator and both buttons; Ready closes popup and draws both green marks plus Everyone is Ready on both clients. | PASS `16-readycheck-popup-b`, `17-ready-response-1`, `17-ready-response-2` |
| Not Ready (CUF ready marks) | Second check: B clicks Not Ready; both draw leader green/member red and Ready check finished. | PASS `18-notready-response-1`, `18-notready-response-2` |
| Owner pet and optional party pet | Native spellbook click casts Summon Imp688; owner sees Imp health frame/action bar. B enables Options→HUD→Layout Settings→Party→Show Pets; other-member pet health frame appears. | PASS `19-pet-raid-a`, `21-party-owner-a`, `21-party-pet-b` |
| Raid pet | Same live Imp and Show Pets enabled, but raid renders no pet. `raid_group` calls `compact_unit_frame` without pet data/settings; optional pets are wired only through `party_unit_frame`. | GAP `20-pet-raid-b`, `22-raid-pet-b`; raid-pet presentation unsupported in this path, not repaired |
| Leave/disband (`PARTY_LEAVE`) | Leader's self-menu Leave empties both group states and removes both raid grids; owner pet remains. | PASS `23-disband-1`, `23-disband-2` |

Native mouse/key events perform all group/pet actions; private admin only seeds level/spell setup. Raid self-menu centre overlaps the player/action bars in Forever; clicks use observed unobstructed frame/menu edges, preserving existing layout. No product fix or RED/GREEN test added. This is bounded two-member Forever proof, not full raid/subgroup/portrait/aura/out-of-interest pet, reference-pixel or general shutdown acceptance.

### Native ready-check repair (raidloop continuation)

`06d85e0d8`, pinned Godot 4.7.2, private UDP 5330 and two Weston/Dozen clients. Evidence: `data/diagnostics/raidloop-2026-10-07/proof-ledger.txt`; `readycheck-*` captures were inspected, with paired JSON and native IPC status/UI dumps. Other previously passed raid steps were not rerun.

| Case | Modern | Forever | Inspected captures (prefix `readycheck-<skin>-`) |
|---|---|---|---|
| Member sees Retail art, initiator text and both labeled buttons | PASS | PASS | `popup-b.png` |
| Mouse Ready marks member ready on leader frames | PASS | PASS | `ready-a.png` (completed summary; Forever IPC `2/2 finished=true`) |
| Second check, mouse Not Ready marks member not-ready | PASS | PASS | `notready-a.png` (IPC `1/2 finished=true`, member decline atlas) |
| Popup closes on either answer and on server timeout | PASS | PASS | `ready-closed-b.png`, `notready-closed-b.png`, `timeout-start-b.png` → `timeout-closed-b.png` |

The native process regression first failed on `79fe67663` with the live `Unconverted native frame decoration: ReadyCheckFrameBorder` error, then passed both skins with the registered Retail dialog nine-slice. Existing error rejection remains intact. Shared-style consumers (character reputation, bank/guild bank, auction, HUD editor and static popups) already register that style; no other missing consumer was found. Forever's menu centre overlaps its action bar at this position; proof clicked the visible left portion of the real menu, without changing approved layout or unrelated menu layering. Cached engine teardown leaks and full reference/general shutdown acceptance remain outside this bounded proof.

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

### Native countdown and ready-check acceptance — 2026-10-11

- Behavioral proof on protocol `dbc2134`, server `12935c9`, engine `d23b56c4d`: protocol countdown wire 1/1; server group 41/41; native UI countdown 4/4, AFK 1/1, assistant permissions 1/1; native party projection 5/5. Logs: `data/diagnostics/readycheck2-2026-10-11/`. The zero-match `raid_manager` invocation is not evidence.
- Fresh empty redb, private UDP54200, two newly created `fb_readycheck3_*` accounts: both skins' party/raid native input proved ready popups, Ready/Not Ready responses, all-ready/mixed/AFK summaries and unanswered-recipient chat, slash countdown bar/medium/large stages, manager10, cancellation and expiry. All 27 `rc3-*.png` captures individually inspected at 1280×720 in `/syncthing/AgentShared/2026-10-11/readycheck/`; paired JSON/input/audio observations in `data/diagnostics/readycheck3-2026-10-11/`.
- Earlier three-account assistant initiation/manager/countdown/cancellation proof: `data/diagnostics/readycheck2-2026-10-11/`, `forever-raid-assistant-*` / `modern-raid-assistant-*` captures in the same PNG directory. Both skins' assistant popup and mixed-response captures inspected again.
- Native audio events prove both clients playing tick file567474 and finish file567438. Sound-kit158565/158566 identifiers are not stream file IDs. Dummy output proves playback state, not human audibility; physical speaker/listening proof remains unclaimed.
- Backing tests: shared-protocol `src/protocol_group_tests.rs`; server `crates/server/src/group/tests.rs`; engine `godot/ui-model/tests/group_countdown.rs`, `godot/ui-model/tests/readycheck_afk.rs`, `godot/ui-model/src/game/group_state_tests.rs`, `godot/rust/src/party_frames.rs`.


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
  - `godot/tests/ready_check_projection.gd`: protocol ready-check update → native group HUD projection, both skins' dialog art/text/buttons and timeout removal (not only the UI-model tree).
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


### Historical raster allocation contract (2026-10-18)

ReadyCheck and other native FontStrings consume one toolkit raster resolver for intrinsic height and placement. Preserve nominal requests, source region/button geometry and whole-block justification. At1080,p=1080/768, nominal12 resolves to17px per line,34px/two lines and a14px face-derived baseline. Source spacing0 is not natural ascent+descent or Godot theme spacing. No arbitrary negative-gap squeezing or own-rectangle glyph clipping. `FONT_POSITIVE_SPACING_GAP_TRUNCATION` is a named future-value gap; no current authored positive multiline source spacing was found. Historical reference only: Retail70448 remains unverified. Post-rl564 native both-skin1920×1080 measurement passes22/22 phases and exact restoration, but two-line glyph−3.875px/shadow+combined−5.875px signed clearance is recorded as `READY_CHECK_MULTILINE_BODY_BUTTON_COLLISION_SOURCE_EQUIVALENT`: layout matches Retail source, allocation overlaps1.65625px, and real Retail70448 same-case ink equality is unverified. Keep geometry unchanged. [Implementation and proof boundary](../wiki/systems/ui-system.md#historical-raster-fontstring-allocation--2026-10-18).


### READY_CHECK_BODY_NO_OVERLAP_POLICY

User instruction (2026-10-18): “in all cases retail does not overlap so make it not overlap”. This is an explicit project layout policy, not a resize mechanism found in the Retail cache. Ordinary single-line geometry remains unchanged. For additional actual shaped/wrapped lines, grow the frame/chrome by measured body allocation minus one resolved line; keep the response buttons bottom-anchored, body TOP37 and nominal font12. No font shrink, upward text offset or content removal. Both skins must have nonnegative glyph, shadow and combined ink clearance above the response-button tops for ordinary, toggle-difficulty and supplied long-name cases. Native projection adds only the extra measured body allocation to frame/background/nine-slice height, keeps the frame centered and response bottoms16UI from the bottom. The allocated block gap equals the one-line case; combined ink clearance must be strictly positive, not identical13.125px (different glyph overhang/pixel snapping may change it). `godot/tests/ready_check_no_overlap.gd` records strict isolation scans/restoration and allocation-driven expected heights for all six cases. RED892b56782 measured both-skin toggle glyph−3.875/shadow−5.875px and three-line long-name−20.875/−21.875px; ordinary+15.125/+13.125px. GREEN rendered measurements remain pending the gate/window; no unmeasured positive clearance is claimed.
