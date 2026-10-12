# Native Arenas

Source-backed skirmish2v2/3v3 and TC-reference rated groups share the existing PvP invitation/entry/results/origin transport. Requirements live in the [client contract](../../specs/arenas.md) and [server contract](../../../../game-server/docs/specs/arenas.md). Verified: 2026-10-12 (code and behavioral fixtures, not rendered live acceptance).

## Authority and lifecycle

Pinned BattlemasterList12.1.0.69933 and Map rows join TDB1210 start locations/weights. The lead-owned importer intersects them with successful local CASC WDT receipts, then promotes only arena object spawns/templates. Eighteen arena WDTs are present; sixteen current arenas have authored gates/orbs. Retired559/562 retain their source condition/level/placement boundaries, never synthetic replacements.

Dead entrants receive source battlefield-port full resurrection before transfer (TC BattleGroundHandler.cpp:380-384). Arena preparation waits for actual worldport readiness, lasts the TC60s delay, and applies/removes source aura32727. Racial factions cannot override match teams; attacks are blocked before engagement and against teammates. Source doors play147Closed/148Open, with per-map5s removal or persistent opening; Empyrean deletes its collision walls. Native active GO meshes keep the camera layer and add a dedicated live-object layer for player floor/wall probes. The original camera-only bodies did not participate in baked-ground prediction: real Cage movement fell through its closed, rendered trapdoor. Actual6339134/6339702 collision-hull prediction and private closed-support→open-drop proof cover the integration; no render geometry or fake platform is used. Elimination retains scoreboard rows, stops combat/casts and fully resurrects defeated players before exact origin return.

Shadow Sight uses source spell34709, trap radiusData2/2 and spawn360s respawn. It enables after90s. Consumed objects **hide**, not despawn: deleting their authoritative entity caused aura removal through CasterGone. GameObjectVisible preserves ownership while the client removes the visual. Initial orb visibility is disabled before the entrant observes the new copy.

## Authoritative live collision (2026-10-12)

A native-only closed-platform proof was insufficient: at the exact Cage walking point443.2981/-24.38845/-323.9937 the static-only server fell to-41.949425 while native feet remained on the trapdoor. Server ground and normal movement now combine static floors with enabled authored live-GO M2 collision keyed by exact `InMap`; controlled knockback and displacement consume the same floors, and movement/LOS consume enabled live walls. Cached model/placement BVHs follow replicated collision, visibility and entity lifetime. No GeoBox/render-mesh collision substitution. TC reference is `GameObject::EnableCollision`/`DynamicMapTree`, pinned a352b1fa.

Behavioral source fixtures prove closed Cage support for60ticks, then opening→drop120ticks; actual Dalaran waterfall and Empyrean wall collision state, hiding, despawn and copy isolation. The explicit server **query microbenchmark** loads all source objects, simulates six nearby query positions, runs twelve floor queries/six wall movements plus live-state synchronization per sample, and warms static geometry before1200samples. Active-waterfall state is included. Combat, replication and whole-frame/tick time are not measured. Historical pre-merge p95 was89/48/120µs; a subsequent under-load verifier measured224/118/312µs at58113673/0ef8c50e3/8c0d4b8. Performance is hardware/load-dependent; final exact-revision receipts, not a universal range, are authoritative. Native proof action `benchmark_collision` is also a synchronous query microbenchmark: actual Godot live-floor/WMO/live-wall rays at six nearby simulated positions,120warmup samples discarded,1200measured samples, with loaded-object and active-hull counts. New final-revision rendered receipts/performance are tracked separately from prior captures in `arena3-proof-ledger.jsonl`/`arena3-native-ledger.json`; new PNGs use `/syncthing/AgentShared/2026-10-12/arena/`.

Named gaps remain:254013pet relocation; TC96538 handler defined but unregistered;1784/aura443 source mismatch; Cage static approach/WMO material/appearance gaps; unreproduced583s frame stall. Collision ignores phase because the server has no production phasing; all imported arena GOs are phaseMask1. TC `GameObjectModel.cpp:143–149` rejects out-of-phase objects. An import-data regression rejects non1 phase masks; collision must consume a real phasing system when one lands, not introduce a collision-only phase model. A single bounded fresh dynamic-arena run checks for recurrence; absence does not certify the old stall fixed.

## Preparation and detection consumers

TC Player::ResetAllPowers restores full health and only the primary pool: mana/energy full, rage/runic/lunar0, other and secondary pools unchanged. Preparation applies it on entry, strictly after5000ms, and when gates open. Source aura72 school/power-mask percentages precede spell-cost modifiers;158 permits PvP talent replacement outside rest areas with ordinary death/combat/cooldown restrictions;215 reconciles the replicated preparation unit flag. Aura18/type4 exposed a visibility bug: reciprocal unit detection was incorrectly required for doors, hiding all static arena objects until opening. [Typed invisibility](../../../../game-server/docs/wiki/systems/auras.md#typed-invisibility-and-spell-area-eligibility-2026-10-11) owns the source object/unit distinction and real-channel reproduction.

Source stealth16/154, detection17 and Shadow Sight228 now feed per-observer interest, spell targets, melee admission and swings. TC Object.cpp checks combat reach, then front-half arc, then type-specific228, then level/value range capped30yd for player observers. Stationary aura/facing/level changes invalidate visibility. Behavioral fixture uses actual Prowl5215 and physical-orb spell34709. Local Rogue1784 has aura443, whereas pinnedTC443 is MOD_LEECH; no stealth interpretation is manufactured for that incompatible source row. NPC model-specific combat-reach overrides are not certified by the player-focused arena proof.

Additional pinned source files/SHA256 receipts are in tc-a352b1fa/. Live native damage/returns do not by themselves prove stealth detection or power/talent behavior; those are separately asserted by source-backed behavioral tests.

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
