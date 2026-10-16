# World-quest availability and native tasks

Verified:2026-10-11 (local date). The user supplies deterministic rotation and unknown-duration defaults as [World of Osso policy](../../specs/world-quests.md), not Retail activation data. Authenticated POI polygons now drive area entry, availability transport and native presentation. Whole coverage remains open at unsupported objective shapes; controlled private completion does not prove natural acquisition or living-character gameplay.

## Current POI census

Pinned local DB2 `12.1.0.69933` QuestInfo.Modifiers&256 and the TDB1210 archive agree on4,207 definitions. QuestPOIBlob contains4,312 matching blobs for3,408 quests. QuestPOIPoint supplies ordered world coordinates. Reject incomplete, fewer-than-three-distinct-point and zero-area shapes; never invent a circle around a point.

| Population | Count |
| --- | ---: |
| World quests | 4,207 |
| No POI blob | 799 |
| Point-only pins, no polygon | 3,164 |
| Authentic polygon quests | 244 |
| Imported existing-system-compatible quests | 239 |
| Authentic polygon quests excluded by unresolved objectives | 5 |
| Imported authentic-duration / policy-duration quests | 0 / 239 |
| Imported quests without supported reward data | 12 |

Dynamic objective vectors now preserve every authored row, order, optional/hidden/contributor/sequenced flags, Flags2 and weights, removing the4NPC/6item slot loss. Types0/1/2/3/9/14/15/21 are admitted with required source joins. All3 criteria trees have one unconditional amount-one leaf; task objective counters persist fresh kill/script or GainAura completion rather than reading lifetime achievements. Remaining5 quests:3 area-trigger tasks, plus86938/86945 whose labels4346/4348 have no local CreatureLabel rows. Whole coverage remains open; unresolved objectives are never silently dropped.

Four definitions have authentic ManagedWorldState durations:52847→113,52848→12,54895→114,54896→115, all604800 seconds. All four have only point pins, so none is currently polygon-admitted. Policy durations cover the imported subset; authentic values would take precedence for any polygon-admitted definition with that join.

Receipt and immutable private database: `/home/osso/.worktrees/world-quests-data/diagnostics/poi-census-v7.json` and `polygon-world-v7.db`. [Server notes](../../../../game-server/docs/world-quests.md) own the reproducible lead import command; canonical world.db is unchanged.

## Runtime and source rewards

`content_world_quest_area` retains quest/blob identity, MapID/UiMapID and ordered vertices. Server `Area.contains` performs polygon membership after `terrain_height::bevy_to_wow` converts native engine axes. Entry accepts and tracks; departure only untracks; wall-clock expiry removes the task/watch. Signed QuestSortID must stay signed during definition loading: negative categories caused the first full-import startup to fail before that repair.

Existing objective credit and quest settlement handle admitted tasks. Completed accepted tasks settle even outside the original polygon; network giver Complete/Choose requests cannot settle tasks. Sequencing gates apply to credit, including progress-bar prerequisites and derived bar completion. Successful validated NPC openings credit TalkTo; authoritative combat death credits player kills. CreatureLabel3055733/layout25B2B438 joins TDB difficulty-row CreatureDifficultyID to entry/mode; only real tapped deaths credit those labels. Label membership is not a KillCredit alias. Quest-bound item Flags2=1 uses persisted counts; valid loot updates them without creating bag items for a single non-source bound objective and reward settlement leaves unrelated bag copies intact. Real-message/temp-redb RED→GREEN tests cover those paths; other item-acquisition routes remain unverified. Completion history uses the exact active window, not permanent/daily one-time gating. Items/money and imported currency/reputation persist with quest history in the reward transaction. Quest XP uses pinned QuestXP and the TDB difficulty/multiplier instead of guessed XP. Raw TDB templates remain inspectable; TDB addon/request/reward children replace corresponding imported rows without AzerothCore reward substitution.

Local CASC QuestFactionReward FDID1282622/layout572521898 supplies two signed16-bit difficulty arrays. TrinityCore Player.cpp6627–6638 uses RewardFactionOverride/100 or signed RewardFactionValue to select difficulty row1/2. The importer follows that source. Unknown currency definitions cannot be manufactured by the existing wallet implementation; all239 currently imported real templates have zero currency quantities. Only45068 pays reputation; source cap7 suppresses positive rewards once Exalted (`Player.cpp:6645`) without blocking losses. Persisted RED→GREEN regression covers that source case. Merged profession knowledge awards use the staged numeric wallet to avoid same-transaction overwrite; ordinary master knowledge-only batch behavior was not changed.

`WorldQuestAvailability` carries per-character active IDs/title, map/zone/UiMapID, polygon-derived pin position, expiry, items/money/XP/currency/reputation previews and server time. Completed windows disappear from the character's availability list. Native bridge/account reducers retain task identity for tracker/toast routing.

Type19 uses `AreaTrigger::HandleUnitEnter` and `GetTemplate()->Id.Id` (`reference/AreaTrigger.cpp:167–170,1017`), not the static-position DB2 namespace. Required entries32539/35987–35990/40181 lack template/create-property bindings in inspected data. Their Amount0 is now loaded as an unset boolean requiring1 (RED→GREEN); production entry credit remains unwired pending authentic bindings. Local AreaTriggerCreateProperties1937227 decodes156 rows, with none of the six required IDs in any raw field. No task polygon is substituted for trigger geometry. Tasks82468/86610/91804 remain excluded pending authentic bindings.

## Native presentation and private proof

Mainline sources: Blizzard_SharedMapDataProviders/WorldQuestDataProvider.lua, Blizzard_WorldMap/WM_WorldQuestDataProvider.lua, Blizzard_ObjectiveTracker/Blizzard_WorldQuestObjectiveTracker.lua and Blizzard_BonusObjectiveTracker.xml. Art comes from local CASC: default worldquest-icon on atlas743/FDID1339312; bonus-objective title/bar art1113812; UI-StatusBar137012.

Native map pins are distinct from ordinary numbered quest pins. Auto-watched tasks open their authored UiMapID, including phased task maps, instead of a conflicting generic map assignment. Hover uses the shared tooltip renderer. The World Quests tracker module retains counted objective text without synthesizing an aggregate bar. Only an unfinished authored progressbar objective creates a percentage bar, retaining its authored text. Low-time warnings use installed GlobalStrings below fifteen minutes; task POIs are unnumbered. Native acceptance/completion toasts use bonus-objective art. Forever changes the same elements' skin/approved scale.

Private UDP5861, account fb_worldquests: real quest86941 Elemental Infusion, objective235666 Shadow Infused Shard x6. Modern Wquestfresh31 and Forever Wqforever32 show0/6→2/6→automatic completion, consumed objective items and11765 XP. Authentic items were administrator-supplied through the existing inventory path; no `.quest complete` was used. Modern replay grant leaves six items in bags, no task reacceptance or new completion toast. UI trees prove both skins' timer/bar/tooltips and completion banners; inspected1280px PNGs are in `/syncthing/AgentShared/2026-10-11/world-quests/`.

**Boundary:** this is controlled bag-objective/settlement proof, not natural gathering/combat. Historical captures expose dead actors without establishing their cause; ordinary-world physical gameplay/living-character certification is not claimed. Prior attempts against86948's enslaved-inferno objective did not establish natural credit. See the [private terrain destinations investigation](../../../../game-server/docs/wiki/investigations/private-terrain-destinations.md). All owned server/client PIDs and slice were stopped;5861 is free. UDP5000 was untouched.

## Continued controlled creature proof

Following both master integrations, matching extension/CLI/server/protocol binaries on private5861 complete86948 Twilight Bonds with Wqvmodern30 and Wqvforever31 (`fb_worldquests`). Real authored Enslaved Inferno46327 deaths produce0/8→7/8→automatic completion and11765XP. Deaths use explicit administrator damage6000, not `.quest complete` or natural player attacks. Both actors are Alive at capture; the existing revive lifecycle restores full level80 fixture health. NPC/source-position terrain is retained, with no collision or terrain-code fix.

Earlier `live-v6` also verifies a private clock+86400 next-window reacceptance, and an extra creature death does not reaccept/reward Modern in the completed window. Current `live-current` Modern receives completion/XP, then relog confirms no task; its immediate post-toast snapshot still shows the prior7/8 state, so that capture is not proof of instantaneous tracker clearing. Forever current capture waits for the empty log and cleared tracker. Inspected earlier map screenshots show available86959 Dragor's Revenge in Modern and86948 in Forever; do not describe both as the same quest tooltip.

Current bounded receipts:23 server WQ tests,12 importer tests,1 both-skin UI integration test,1,733 full protocol tests; native builds/check/fmt pass. Independent bounded artifact verification passes with whole coverage still open at5 source/producer gaps. Natural acquisition/player combat and all-item-acquisition routes remain uncertified. All owned clients/server and the sanitized `agents-world_quests.slice` are stopped;5861 free. Current proof inputs remain under `world-quests-data/live-current`; durable receipts are copied into canonical engine diagnostic storage, never committed.

## 2026-10-16 navigation and authored Deathwing hazard

Verified: 2026-10-16. Fresh private 70391 import reproduces 4,207 tasks, 244 polygons and 239 admitted tasks, all with the explicit 86,400-second realm policy. Existing server windows, polygon entry and transactional settlement were already implemented; the new gap was clicking available map pins without a navigation action and missing available-task minimap markers.

Received active pin clicks now toggle local selection without accepting remotely. Selection survives before polygon entry; minimap markers use received coordinates/map and only the selected offscreen task gets a rim arrow. Ordinary numbered minimap POIs exclude world quests. First-in-world map opening loads actual quest blob art before drawing: previously WorldMap updated before Quests initialization and closed the first map with `Quest blob art missing`. This minimap composition is project presentation, not an opaque Retail-renderer parity claim.

A controlled 86948 attempt initially found all ten Enslaved Infernos (46327) dead before administrator damage. Instrumented clean startup proves they spawn and remain at 5552/5552 before player entry. Player grid activation wakes authentic NPC44744 Deathwing (spawn20383796): addon83508 triggers Molten Flame94644 every second; its authored effect2 is Instakill with a 400-yard source-area-entry radius. Actual LastDamager records Deathwing killing all ten, not terrain, zero-health initialization, ContentTuning or SummonLevels. Source phase169 is DEFAULT_PHASE, and the inspected source has no event/spawn-group membership or94644 implicit-target conditions. No NPC health or quest credit was injected; temporary instrumentation was archived and removed.

Pinned TrinityCore a352b1fa `SpellEffects.cpp::EffectInstaKill` and `Spell.cpp::WorldObjectSpellTargetCheck` do not establish an exception for those nearby NPCs. Do not invent immunity, delete Deathwing from product data or change source content policy merely to pass a fixture. Named gap: **World-event hazard instakills objective NPCs; Retail phasing or scripting source not present in allowed sources**. Lead approved a controlled private fixture omitting only spawn20383796; the original authored database and ten objective actors remain unchanged. The fixture asserts Deathwing is absent before real administrator-driven NPC deaths. This proves settlement only, not ordinary combat or a fix for the authored hazard. Diagnostic inputs/logs: `/home/osso/.worktrees/world-quests-proof/npc-{before-player.json,native9-server.log,native10-server.log}`; captured old binaries remain explicitly historical.

## Multi-area pin continuation — implementation checkpoint

A fresh70391 census has239 admitted tasks and309 area blobs, with no missing XY map assignments. Task82521 has lower blob514681 on2256 and main-map514720 on2255, both world2601. The old producer published only the first area although acceptance checked any area. Raw QuestPOIPoint.csv also supplies source Z: those blobs have mean heights -1591.7/-1607. The importer discarded Z, and the old renderer supplied invented Z0 to a3D map lookup;2256's ceilings -1360/-1487 then hid its own pin. Source-XYZ projection has zero assignment misses across all309 areas. UI pin loss, server second-area loss and importer discarded/missing-Z acceptance were reproduced before their fixes.

[Availability wire](../../../../shared-protocol/docs/world-quests.md) now retains every area under one task/window/reward identity. Importer, runtime areas and POI wire now retain source XYZ. The renderer passes that actual height through the existing3D map lookup; the unverified2D workaround was removed rather than weakening floor discrimination. A displayed map considers its assigned area maps or navigable descendants, not unrelated siblings with overlapping bounds; multiple matching areas produce one pin in published area order. The horizontal minimap chooses the nearest published area on the current world map; watched-map opening retains the first authored map. These are project presentation rules, not opaque Retail C API parity. Existing XY polygon acceptance, rotation and settlement are unchanged. Reimport old XY-only area data before running the new schema; no zero-height fallback is added.

Verification is pending for this continuation; the following3c08 native checkpoint does not certify the new wire/projection. New fixture `godot/ui-model/tests/fixtures/world_quest_pins_70391.json` records actual239-task/309-area inputs and source DB provenance. Reward choice is not applicable: the exact six-field JSON query over239 imported source templates finds zero nonzero choice IDs; no unsourced choice path is added.

## Current controlled navigation/window/reward proof

Engine production3c08c72fd, fixture347ea2088 and clean server2dadf94e run against the explicitly hazard-absent fixture (SHA256 `17869243c785452f075285cb1ddbacf98eac07299c4e515bfb22f547d161c117`). Real private accounts `fb_wqnavm5`/character36 Wqnavmodfive (Modern) and `fb_wqnavf3`/character37 Wqnavforthri (Forever) retain their actual saved skin profiles. Level80 and outside-polygon teleport are controlled setup, not natural character progression.

Both skins pass actual native pin hover/reward preview, physical pin selection without quest acceptance, persistent pre-entry minimap direction, polygon acceptance/unnumbered arrival marker, seven real administrator-driven46327 deaths yielding7/8, eighth death automatically removing the log and delivering `XP=0->11765 delta=11765` plus the exact quest XP acknowledgement. Returning outside/inside does not replay the completed task. Separate runs advance the real private realm clock to a9-minute warning, then across the half-open window end: the task, pin and local selection disappear. The fixture asserts an actual empty NPC44744 query before any controlled deaths; no health restoration or quest-credit injection is used.

All26 original1920×1080 PNGs were individually inspected through private1280px views: `/syncthing/AgentShared/2026-10-16/world-quests/3c08c72fd-window16/` and `3c08c72fd-controlled17/`. Minimap detail crops confirm the selected rim arrow and unnumbered arrival marker. Outside-start camera/background occlusion is visible; this proof does not certify terrain/camera fidelity or ordinary combat. Exact binaries, fixtures, original/controlled databases, profiles, capture hashes and action logs live in `/home/osso/.worktrees/world-quests-proof/{runtime-*-inputs.json,final-capture-manifest.json,window16-*.log,controlled17-*.log,proof-ledger.md}`. Earlier failed captures retain their original identities.

Targeted source proof: navigation RED0/3→GREEN, clock/bar-text RED2/4→GREEN4/4, tracker number/abbreviation RED3/4→GREEN4/4, retained-POI-background RED3/4→GREEN4/4. Nine ordinary numbering/hover/minimap controls remain GREEN; current extension build, registered-helper check and changed-crate format check exit0. This closes the bounded navigation/window/controlled-settlement gate, not the five unsupported objective sources or the named authored-hazard gap.

## Historical activation audit

TDB quest pools/events do not provide a Retail activation calendar:282 quest_pool_members/63 quest_pool_template rows and zero matching world quests; matching creature/gameobject quest events, event conditions and seasonal relations also zero. ResetByScheduler is not an epoch; all4,207 templates have TimeAllowed0. ManagedWorldState CurrentStageWorldStateID has no matching TDB world_state default.

Local-CASC follow-through decoded ManagedWorldState1587159, input1587151, buff1587155, AreaPOI1000630 and AreaPOIState1000631. Inline AreaPOI continent relationships were validated instead of appended twice;3,909 POIs/1,070 states/154 buffs yielded zero joins for the initial four duration-defined tasks. Historical ScheduledWorldState payloads were absent from the local root. QuestV2CliTask1028735 and PlayerCondition1045411 supplied task/condition metadata, but did not establish a Retail activation calendar. The user-approved polygon/default-duration policy supersedes the earlier four-definition-only admission boundary.

## Sources

- TDB1210 archive `/home/osso/Projects/world-of-osso/game-server/data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql`, SHA256608534d1dc97535c92d756914a02fe36264e484b519f6e54f6d10cf56c6f4d96; read-only world.db and pinned DB2 CSVs.
- [Server importer/runtime](../../../../game-server/docs/world-quests.md); shared-protocol `protocol/quest_messages.rs`; engine `world_quest_ui.rs`, `objective_tracker_component.rs`, native `quests.rs`/`world_map.rs`.
- Primary local Retail Lua/XML under `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/`; pinned TrinityCore Player.cpp in `/home/osso/.worktrees/world-quests-data/reference/`.
- `/home/osso/.worktrees/world-quests-data/diagnostics/` proof logs and `/home/osso/.worktrees/world-quests-data/live/` private inputs, IPC trees and captures. Raw DB2s are retained under private `db2/`.

## See Also

- [World-quest contract](../../specs/world-quests.md)
- [World-map contract](../../specs/world-map.md)
- [[quest-ui]] — existing objectives and quest UI
- [[quest-share-resets]] — persistent quest clocks/history
