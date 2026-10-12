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
| Imported existing-system-compatible quests | 236 |
| Authentic polygon quests excluded by unresolved objectives | 8 |
| Imported authentic-duration / policy-duration quests | 0 / 236 |
| Imported quests without supported reward data | 12 |

Dynamic objective vectors now preserve every authored row, order, optional/hidden/contributor/sequenced flags, Flags2 and weights, removing the4NPC/6item slot loss. Types0/1/2/3/9/15/21 are admitted with required source joins. Remaining8 quests:3 criteria trees,3 area-trigger tasks, plus86938/86945 whose labels4346/4348 have no local CreatureLabel rows. Whole coverage remains open; unresolved objectives are never silently dropped.

Four definitions have authentic ManagedWorldState durations:52847→113,52848→12,54895→114,54896→115, all604800 seconds. All four have only point pins, so none is currently polygon-admitted. Policy durations cover the imported subset; authentic values would take precedence for any polygon-admitted definition with that join.

Receipt and immutable private database: `/home/osso/.worktrees/world-quests-data/diagnostics/poi-census-v5.json` and `polygon-world-v5.db`. [Server notes](../../../../game-server/docs/world-quests.md) own the reproducible lead import command; canonical world.db is unchanged.

## Runtime and source rewards

`content_world_quest_area` retains quest/blob identity, MapID/UiMapID and ordered vertices. Server `Area.contains` performs polygon membership after `terrain_height::bevy_to_wow` converts native engine axes. Entry accepts and tracks; departure only untracks; wall-clock expiry removes the task/watch. Signed QuestSortID must stay signed during definition loading: negative categories caused the first full-import startup to fail before that repair.

Existing objective credit and quest settlement handle admitted tasks. Completed accepted tasks settle even outside the original polygon; network giver Complete/Choose requests cannot settle tasks. Sequencing gates apply to credit, including progress-bar prerequisites and derived bar completion. Successful validated NPC openings credit TalkTo; authoritative combat death credits player kills. CreatureLabel3055733/layout25B2B438 joins TDB difficulty-row CreatureDifficultyID to entry/mode; only real tapped deaths credit those labels. Label membership is not a KillCredit alias. Quest-bound item Flags2=1 uses persisted counts; valid loot updates them without creating bag items for a single non-source bound objective and reward settlement leaves unrelated bag copies intact. Real-message/temp-redb RED→GREEN tests cover those paths; other item-acquisition routes remain unverified. Completion history uses the exact active window, not permanent/daily one-time gating. Items/money and imported currency/reputation persist with quest history in the reward transaction. Quest XP uses pinned QuestXP and the TDB difficulty/multiplier instead of guessed XP. Raw TDB templates remain inspectable; TDB addon/request/reward children replace corresponding imported rows without AzerothCore reward substitution.

Local CASC QuestFactionReward FDID1282622/layout572521898 supplies two signed16-bit difficulty arrays. TrinityCore Player.cpp6627–6638 uses RewardFactionOverride/100 or signed RewardFactionValue to select difficulty row1/2. The importer follows that source. Unknown currency definitions cannot be manufactured by the existing wallet implementation; all currently imported real templates have zero currency quantities.

`WorldQuestAvailability` carries per-character active IDs/title, map/zone/UiMapID, polygon-derived pin position, expiry, items/money/XP/currency/reputation previews and server time. Completed windows disappear from the character's availability list. Native bridge/account reducers retain task identity for tracker/toast routing.

## Native presentation and private proof

Mainline sources: Blizzard_SharedMapDataProviders/WorldQuestDataProvider.lua, Blizzard_WorldMap/WM_WorldQuestDataProvider.lua, Blizzard_ObjectiveTracker/Blizzard_WorldQuestObjectiveTracker.lua and Blizzard_BonusObjectiveTracker.xml. Art comes from local CASC: default worldquest-icon on atlas743/FDID1339312; bonus-objective title/bar art1113812; UI-StatusBar137012.

Native map pins are distinct from ordinary numbered quest pins. Auto-watched tasks open their authored UiMapID, including phased task maps, instead of a conflicting generic map assignment. Hover uses the shared tooltip renderer. The World Quests tracker module displays timer and count-derived progress; native acceptance/completion toasts use bonus-objective art. Forever changes the same elements' skin/approved scale.

Private UDP5861, account fb_worldquests: real quest86941 Elemental Infusion, objective235666 Shadow Infused Shard x6. Modern Wquestfresh31 and Forever Wqforever32 show0/6→2/6→automatic completion, consumed objective items and11765 XP. Authentic items were administrator-supplied through the existing inventory path; no `.quest complete` was used. Modern replay grant leaves six items in bags, no task reacceptance or new completion toast. UI trees prove both skins' timer/bar/tooltips and completion banners; inspected1280px PNGs are in `/syncthing/AgentShared/2026-10-11/world-quests/`.

**Boundary:** this is controlled bag-objective/settlement proof, not natural gathering/combat. Captures expose dead actors from the existing private terrain/spawn-state limitation; ordinary-world physical gameplay/living-character certification is not claimed. Prior attempts against86948's enslaved-inferno objective did not establish natural credit. See the [private terrain destinations investigation](../../../../game-server/docs/wiki/investigations/private-terrain-destinations.md). All owned server/client PIDs and slice were stopped;5861 is free. UDP5000 was untouched.

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
