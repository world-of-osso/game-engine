# Native Battlegrounds

Native Warsong queue, invitation, objective HUD, scoreboard/results and origin return share one Retail layout across Modern and Forever. Server content/policy authority is the [server contract](../../../../game-server/docs/specs/battleground-lifecycle.md); the [client contract](../../specs/battleground-lifecycle.md) owns UI acceptance. Verified: 2026-10-11.

## State and input

`PvpStateUpdate.snapshot.battleground` carries match identity, phase, acceptance, flags, team scores, roster and outcome. `AccountEvent::Pvp` delivers it to `PvpView`; `BattlegroundUi` mounts one RegistryUi canvas. Queue-window visibility is independent of invitations and the in-match HUD. Requests stay pending until server response; stale buttons echo their match identity rather than acting on another match.

Native source-named controls: `HonorFrameQueueButton`, `PVPReadyDialogEnterBattleButton`/`LeaveQueueButton`, `WorldStateScoreboardButton`, `PVPScoreboardTab1..3`, `PVPMatchLeaveButton`. The Group Finder micro button and `game-engine-cli pvp open` expose the bounded WSG view. All/Alliance/Horde filters retain server roster rows; pixel scrolling supports a full roster. Results open on the server's Ended transition even after closing the live scoreboard. Unknown server honor data is absent; uninstrumented combat columns show dashes, never measured-looking zeros.

## Live-discovered boundaries

- Type36 flags were excluded by the native game-object type dispatch, so no visual/usable object existed despite valid replication. `461f17ae0` admits the source flag type through normal display-ID model resolution and interaction. Live object-use succeeds after this fix.
- Closing the active scoreboard persisted its dismissal into match end. Same-order both-skin RED reproduced missing results; `374ab9647` resets dismissal on the authoritative Ended transition, while permitting later result dismissal.
- `e90c59497` removes the client-only Waiting-phase leave prohibition. An arrived native player left an accepted Waiting match while nine peers remained unaccepted, returning to the exact retained origin; server eligibility remains authoritative.
- Support peers initially sent WorldPortAck on S2C TerrainChannel. The fixture now uses source TransferChannel. Waiting-only captures are archived separately; no completed-match claim uses that run.

## Bounded live proof

Private UDP5710, diagnostics-owned world.db, ten `fb_bg*` accounts: one rendered Godot client and nine lightweight NetworkBridge peers. Heavy headless Godot peers were stopped after observed host overload. Both skins completed real UDP matchmaking and entry, native UseGameObject flag pickup, three captures, ten-player outcome snapshots and all-player origin return. Private admin position commands accelerated travel to the **authored** flag/capture points; no score, flag or outcome state was injected.

Modern additionally proves automatic results after closing the live scoreboard and native five-row Alliance/Horde filtering. Native invitation decline is proved in both skins. Forever results were opened through the native Scoreboard button before the transition fix; both-skin behavior tests cover the corrected transition. The rendered client's returned server coordinates match its retained origin exactly. Supporting peers prove origin-map NewWorld/WorldPortAck and cleared match state; exact all-player coordinate retention has separate server channel-fixture proof.

Evidence: `data/diagnostics/battleground-lifecycle-2026-10-11/` contains `*-final-*-pvp.json`, per-peer end/return snapshots, pointer ledger and targeted RED/GREEN logs. Six game-object tests and two both-skin UI tests pass; extension builds have no compile warnings. Sixteen personally inspected 1280×720 PNGs reside in `/syncthing/AgentShared/2026-10-11/battleground-lifecycle/`. Numeric timer/reward policy and exact PvP-specific texture parity remain explicit contract gaps; this is not full Retail parity certification. Scoped helper cargo check (Godot extension and CLI) and changed-Rust formatting checks pass at `e90c59497`. Lead owns integration review; full suites run periodically on master, not per-agent branches. No master merge or canonical DB write was performed.

## Source node maps — implementation proof 2026-10-12

The SpecificScrollBox retains a source queue identity rather than sending WSG for every row. Arathi/Gilneas use the existing isolated-match admission and origin-return path. Shared `NodeBattle` orders authored capture deadlines and resource ticks, pauses contested income and credits source assault/defense PvP stat IDs. Server snapshots carry source WorldState/value pairs plus measured objective counters. Native HUD and scoreboard choose node columns for those map definitions; combat/honor remain explicitly unknown. Type 42 now uses normal replicated game-object rendering and interaction.

Proof at this checkpoint: 82 scoped shared battleground tests; two actual HostClient channel fixtures cover each node map's queue/invite/source safe-location entry, physical capture points, timed captures/resource victory, roster/objective counters and all-player origin return; three focused native/UI tests include both skins. Arathi additionally has private UDP 5712 Modern/Forever proof: 16 real accounts, five physical authored node uses, actual capture/resource clocks, 1500 victory, automatic results and all-player return/clear; 14 inspected PNGs and `arathi-live-proof.json` live in the coverage diagnostics/publication paths. Gilneas has the same both-skin proof with 6 real accounts, three physical authored node uses and 14 inspected PNGs (`gilneas-live-proof.json`); its first attempt exposed an invitation-receipt harness race, so acceptance now waits until every peer holds the native match identity. Twin Peaks runs on the same flag path as WSG (shared `FlagBattle`, sourced 15-minute end) and has both-skin private proof (`twinpeaks-live-proof.json`, 16 PNGs). Eye of the Storm runs a separate shared `EyeBattle`: 1 s presence heartbeats over the four TDB control zones, a neutral flag scored at a held tower trigger, ground drop/expiry and respawn. Its live proof waits on M2 floor collision, because both start points are an M2 floating rock (`eye-start-floor-finding.json`). PvP ScrollBoxes now take host-routed wheel/stepper input with row pan extents. The scoreboard reads `pvp_stat_columns(map)` (PvpStat titles, PvpScoreboardLayout order) for every map, and the queue list offers each template whose map has objective objects. Source geometry, identities, numeric rules and lead-only promotion commands belong in the [server contract](../../../../game-server/docs/specs/battleground-lifecycle.md#node-map-implementation-verification-pending). Banner channels, graveyard resurrection and preparation/broadcast parity remain gaps there.

Live fixture support accounts are source-minimum rosters with required `BG_BATTLEGROUND_ID` and `BG_ACCOUNT_PREFIX=fb_*`; a native client supplies the remaining participant. Private ports remain 5710–5712. Remaining maps and the acceptance matrix are tracked in the [client coverage contract](../../specs/battleground-lifecycle.md#coverage-expansion--active2026-10-12).

## Kotmogu — bounded both-skin proof 2026-10-12

Source queue 699/map 998 has four required accounts, authored TC-scripted orbs and a separate shared `KotmoguBattle`: one orb per player, direct home return on death/departure, five-second region ticks and simultaneous-limit draw. Native type24 orb visuals/cursor, four possession WorldState cells and DB2-ordered Orb Possessions 418/Victory Points 419 share the same source layout in both skins. Server channel proof covers entry, physical use, death/respawn, measured counters, 1500 victory and all-player exact origin return.

Private 5712 Modern/Forever runs each use one rendered client plus three NetworkBridge peers, physical orange-orb use and natural center ticks through 1500. Both show measured418=1/419=1500, automatic Victory after closing the live scoreboard, exact native server-origin return and all-peer map/clear. Eighteen inspected1280×720 PNGs and `data/diagnostics/bg-coverage-2026-10-12/kotmogu-live-proof.json` retain evidence. Private fixture level40/alive/source Goldshire baseline setup precedes matching; admin only accelerates authored travel during the match. Combat aura/kill bonus, visible checker geometry and source NPC appearance/asset failures remain gaps, not full Retail/scenery certification.

The harness now waits for the server's next four-character periodic save after returns before quitting sessions and killing its private server. The dirty-save interval is300 ticks/15seconds; premature teardown had left saved battleground-map records that a fresh process could not re-enter. A same-map admin teleport updates position without NewWorld, so peer readiness must not wait for a nonexistent transfer packet. Canonical world.db remains lead-owned; [source importer commands](../../../../game-server/docs/specs/battleground-lifecycle.md#kotmogu-continuation--active-2026-10-12) are the publication contract.

## PvP progression integration — verified 2026-10-11

The same PvP view owns Quick Match/Rated/Premade categories and authoritative Honor/Conquest wallet text. Unknown season cap/rating are optional fields, not zero/default policy. ClassTalentsFrame contains the separate Mainline three-slot talent tray and War Mode control. Runtime source catalogs are mandatory; the [server progression contract](../../../../game-server/docs/specs/pvp-progression.md#lead-import-checklist) owns import instructions and remaining numeric gaps.

Live fresh-login discovery: unlike test fixtures, authenticated player spawn lacked CombatStatus. Required eligibility queries silently skipped the player, leaving both talent and War Mode snapshots absent. The actual character-selection regression fails before inserting the explicit noncombat component and passes afterward. Instance transfer regression applies a real Enlisted aura then moves to489/instance7 with no terrain; checking map identity before terrain removes the aura without erasing desired mode. Talent grants follow actual aura119 rather than the cached Enlisted snapshot.

Both-skin private5790 pointer/relog proof and remaining honor-level/XP/tooltip/drag boundaries are recorded once in the [client progression contract](../../specs/pvp-progression.md#bounded-acceptance--2026-10-11).

## Sources

- [PvP progression contract](../../specs/pvp-progression.md) — wallet, source talents, War Mode proof and unresolved lifetime honor policy.
- [Client contract](../../specs/battleground-lifecycle.md) — source XML/Lua paths, panel geometry, policy gaps.
- [Server contract](../../../../game-server/docs/specs/battleground-lifecycle.md) — source identity, content, objectives and transfer authority.
- [UI model](../../../godot/ui-model/src/battleground.rs) — authoritative view, actions, filters and results.
- [Native host](../../../godot/rust/src/battleground.rs) and [game objects](../../../godot/rust/src/game_objects.rs) — registry lifecycle and flag interaction.
- [UDP peers](../../../godot/network/examples/native_battleground_peers.rs) and [native input driver](../../../godot/tests/battleground_live.gd) — real-message and pointer proof.

## See Also

- [[battle-pets]] — separate PvP pet-battle queue; no shared battleground policy.
- [[networking]] — shared transport and replicated world state.
