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

## PvP progression integration — verified 2026-10-11

The same PvP view owns Quick Match/Rated/Premade categories and authoritative Honor/Conquest wallet text. Unknown season cap/rating are optional fields, not zero/default policy. ClassTalentsFrame contains the separate Mainline three-slot talent tray and War Mode control. Runtime source catalogs are mandatory; the [server progression contract](../../../../game-server/docs/specs/pvp-progression.md#lead-import-checklist) owns import instructions and remaining numeric gaps.

Live fresh-login discovery: unlike test fixtures, authenticated player spawn lacked CombatStatus. Required eligibility queries silently skipped the player, leaving both talent and War Mode snapshots absent. The actual character-selection regression fails before inserting the explicit noncombat component and passes afterward. Instance transfer regression applies a real Enlisted aura then moves to489/instance7 with no terrain; checking map identity before terrain removes the aura without erasing desired mode. Talent grants follow actual aura119 rather than the cached Enlisted snapshot.

Both-skin private5790 pointer/relog proof and remaining honor-level/XP/tooltip/drag boundaries are recorded once in the [client progression contract](../../specs/pvp-progression.md#bounded-acceptance--2026-10-11).

## Arena integration — verified 2026-10-12

[Arenas](arenas.md) share this transport and view but use elimination, source preparation/gates/orbs, enemy frames and played-match TC-reference rating. Arena combat columns are measured; WSG's existing uninstrumented columns remain dashes. Arena source/runtime/live acceptance is tracked separately, not inferred from the WSG proof.

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
