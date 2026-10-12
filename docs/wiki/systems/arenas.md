# Native Arenas

Source-backed skirmish2v2/3v3 and TC-reference rated groups share the existing PvP invitation/entry/results/origin transport. Requirements live in the [client contract](../../specs/arenas.md) and [server contract](../../../../game-server/docs/specs/arenas.md). Verified: 2026-10-12 (code and behavioral fixtures, not rendered live acceptance).

## Authority and lifecycle

Pinned BattlemasterList12.1.0.69933 and Map rows join TDB1210 start locations/weights. The lead-owned importer intersects them with successful local CASC WDT receipts, then promotes only arena object spawns/templates. Eighteen arena WDTs are present; sixteen current arenas have authored gates/orbs. Retired559/562 retain their source condition/level/placement boundaries, never synthetic replacements.

Dead entrants receive source battlefield-port full resurrection before transfer (TC BattleGroundHandler.cpp:380-384). Arena preparation waits for actual worldport readiness, lasts the TC60s delay, and applies/removes source aura32727. Racial factions cannot override match teams; attacks are blocked before engagement and against teammates. Source doors play147Closed/148Open, with per-map5s removal or persistent opening; Empyrean deletes its collision walls. Elimination retains scoreboard rows, stops combat/casts and fully resurrects defeated players before exact origin return.

Shadow Sight uses source spell34709, trap radiusData2/2 and spawn360s respawn. It enables after90s. Consumed objects **hide**, not despawn: deleting their authoritative entity caused aura removal through CasterGone. GameObjectVisible preserves ownership while the client removes the visual. Initial orb visibility is disabled before the entrant observes the new copy.

## Rated reference and scores

Full max-level groups queue through their leader. TC a352b1fa ArenaTeam/Queue and worldserver.conf supply the formula, starting0personal/1500MMR,150window and600s discard. This is explicitly a TC reference, not certification of Blizzard's proprietary Retail MMR. Results persist only after a played outcome; old stored ratings without sourced MMR stay absent from projections. No fabricated match payout, season reset/cap or rating-only queue award exists.

Combat log events populate arena damage, effective healing and killing blows, excluding ATTR7_DO_NOT_COUNT_FOR_PVP_SCOREBOARD. Source combat deaths remain separate from post-match resurrection and living forfeits; departing players can cause elimination without adding a death. Both skins share Quick Match/Rated controls, source-sized98×44 CompactArenaFrames and result tables; Forever only supplies art.

## Proof boundary

Twelve scoped server tests include real HostClient four- and six-player skirmish/rated invitations, authored starts/acks, source preparation and physical orb/aura lifetime, HP-primed real melee kills, score/outcome/full resurrection, persisted rating/MMR and exact all-player origins. Four both-skin registry/action tests and nine native object tests pass. Diagnostics: data/diagnostics/arena-coverage-2026-10-12/. Private both-skin native/UDP3v3 pure elimination passes: ForeverHook Point1825 damage12,880/KB3; ModernTiger's Peak1134 damage12,904/KB3; all6 restored health and returned origins. FreshForever2v2 Robodrome2167 damage8,648/KB2/all4returns, plus ModernRuins572 source living-forfeit0deaths. Only private actor positions were staged, never HP/scores/outcomes/ratings. PNGs<=1280 inspected in /syncthing/AgentShared/2026-10-12/arena-coverage/. All-map/source-consumer/final gates remain in the handoff. Real-UDP startup exposed missing gate/orb codecs; the native fingerprint fixture now passes. Native captures exposed an opaque enemy-target button; transparent hitboxes retain class-colored health/name art. Client attacks now use arena teams, including summoned-pet controllers; roster targets survive normal AOI absence.

## Source-policy audit (2026-10-12)

[Original Blizzard Shuffle preview](https://worldofwarcraft.blizzard.com/en-gb/news/23745827/eternitys-end-preview-solo-shuffle) establishes six rounds, two healers/four damage dealers, healer/DPS permutations and first-kill outcomes. It explicitly described an experimental Brawl, not a finalized rated ruleset. [Ghosts of K’aresh notes](https://worldofwarcraft.blizzard.com/en-us/news/24226698/ghosts-of-karesh-content-update-notes) subsequently changed consecutive teammate ordering and Protection Paladin roles. Complete 12.1 round preparation/reset and current rated/leaver policy are not established by these sources; Shuffle remains disabled rather than guessed.

[December 2025 hotfixes](https://worldofwarcraft.blizzard.com/en-us/news/24245107/hotfixes-december-19-2025) acknowledge incorrect dampening speed without specifying the intended current schedule. Historical pre-expansion rates do not certify 12.1 timing/ramps/caps. No dampening timer is synthesized. Downloaded primary pages and SHA256 receipts: diagnostics/arena-coverage-2026-10-12/policy-sources/; search receipts identify the remaining exact gaps.

## Sources

- [Server contract/import checklist](../../../../game-server/docs/specs/arenas.md).
- [Client contract](../../specs/arenas.md).
- [Arena queues](../../../../game-server/crates/server/src/pvp_arena.rs), [rated queue](../../../../game-server/crates/server/src/pvp_arena_rated_queue.rs), [results](../../../../game-server/crates/server/src/pvp_arena_rated_results.rs).
- [Source orb lifecycle](../../../../game-server/crates/server/src/pvp_arena_sight.rs), [gate lifecycle](../../../../game-server/crates/server/src/pvp_arena_objects.rs), [combat scores](../../../../game-server/crates/server/src/pvp_arena_scores.rs).
- [Native UI](../../../godot/ui-model/src/arena_ui.rs) and [object renderer](../../../godot/rust/src/game_objects.rs).
- Pinned source files and SHA256 receipts: data/diagnostics/arena-coverage-2026-10-12/tc-a352b1fa/sources.json.

## See Also

- [[battlegrounds]] — shared PvP screens and transfer ownership.
- [[networking]] — transport and replicated world-object state.
