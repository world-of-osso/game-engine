# World-quest availability source boundary

Verified: 2026-10-11. Local inputs supply four authentic managed-state durations but no Retail activation calendar. The user's 2026-10-11 decision resolves scheduling with an explicitly authored [World of Osso rotation policy](../../specs/world-quests.md#world-of-osso-rotation-policy). Rotation/import/expiry foundations now exist; Retail task-area/eligibility integration and private completion acceptance remain open.

## Source census

Read-only `world.db` joins classify quests using pinned `QuestInfo.Modifiers & 0x100`; the world-quest QuestInfo IDs match the local `12.1.0.69933/QuestInfo.csv` exactly. The TDB1210 archive contains 4,207 matching quest templates. The canonical runtime `content_quest_template` contained zero matching quests in the source receipt. The continuation imported four into a new private SQLite backup only; canonical data was not changed.

The actual TDB `quest_pool_members` has 282 rows and `quest_pool_template` has 63 rows. **Zero world quests belong to these quest pools.** `pool_members.type=2` describes child spawn pools, not quests; numeric overlap with quest IDs is not quest availability evidence.

| Inspected TDB source | Rows | World-quest matches |
| --- | ---: | ---: |
| quest_pool_members | 282 | 0 |
| game_event_creature_quest | 235 | 0 |
| game_event_gameobject_quest | 101 | 0 |
| game_event_quest_condition | 0 | 0 |
| game_event_seasonal_questrelation | 690 | 0 |

No `world_quest`-named table exists in this dump. Four matching quests have nonzero `ManagedWorldStateID`: 52847→113, 52848→12, 54895→114, 54896→115. Another 1,127 templates set `ResetByScheduler`, but that flag supplies no activation epoch, period, or expiry. All 4,207 have `TimeAllowed=0`; a quest-log time limit is not an availability window anyway. Absence of a scheduler flag does not establish permanent availability.

## Local CASC follow-through

Missing CSV exports were not treated as missing source content. Extracted from local Retail build `dcfc90fffd79ba00406ae46f5f657592` (`12.1.0.69933`), without CDN access:

| Local DB2 | FDID | Records / layout |
| --- | ---: | --- |
| ManagedWorldState | 1587159 | 9 / A4E9EA9F |
| ManagedWorldStateInput | 1587151 | 8 / 16683306 |
| ManagedWorldStateBuff | 1587155 | 154 / B8C3D344 |
| AreaPOI | 1000630 | 3,909 / A8930CBB |
| AreaPOIState | 1000631 | 1,070 / 60943110 |

Decoded ManagedWorldState rows12/113/114/115 all define `UpTimeSecs=604800` and `DownTimeSecs=1`. Their `CurrentStageWorldStateID` values are15622/16033/16707/16708 respectively; **none has a TDB world_state default**. ManagedWorldState IDs are not WorldState IDs. Duration definitions do not establish a current stage or an activation epoch. No guessed epoch or initial stage was assigned.

ScheduledWorldState1327083, ScheduledWorldStateGroup1327084 and ScheduledWorldStateXUniqCat1388724 failed local extraction with no matching root content/path. Their historical community-listfile entries alone do not establish current build payloads. This limitation is retained in `casc-extract.log`; it was not worked around by downloading game data.

The initial pinned CSV directory contains QuestInfo, QuestV2, QuestPOIBlob, QuestPOIPoint, QuestXP, QuestMoneyReward and quest-line catalogs. Raw DB2s and exact headers/hashes are retained beside the census receipt. The continuation's reproducible [source-area audit](../../../../game-server/docs/world-quests.md#source-area-audit) decodes the matching AreaPOI/AreaPOIState/ManagedWorldStateBuff layouts and checks the four quest IDs, managed-stage IDs and boss names. It does not claim coverage of every Retail task-area mechanism.

## Retail distinction

Local Retail `AddOns/Blizzard_SharedMapDataProviders/WorldQuestDataProvider.lua:207–250` obtains tasks from `GetTasksOnMapCached(mapID)`, adds matching world-quest pins and removes pins absent from that task list. `AddOns/Blizzard_WorldMap/WM_WorldQuestDataProvider.lua:7–13` filters to the displayed map. Static quest identity and POIs do not substitute for that active-task list.

`AddOns/Blizzard_ObjectiveTracker/Blizzard_WorldQuestObjectiveTracker.lua:33–48` reads `C_TaskQuest.GetQuestTimeLeftMinutes`; that dynamic remaining time is not encoded by the inspected template fields. Existing native fixtures named `world_quest_flow.gd` and `world_quest_progress.gd` test ordinary NPC-given quests, not this feature; see [quest UI](../systems/quest-ui.md).

## Continuation after the scheduling decision

Server `world_quests.rs` calculates configurable deterministic windows using the four authentic durations. The private-copy importer maps TDB templates, objectives and static pins without cross-product reward substitution, and retains the full source reward/template JSON. `quest_time.rs` removes task logs and watches at availability expiry. [Server import/runtime notes](../../../../game-server/docs/world-quests.md) give the lead command; the spec owns policy and acceptance.

Further local-CASC follow-through extracted QuestV2CliTask1028735/layoutD6D31C39 and PlayerCondition1045411/layout1D743D99. The four readable task rows reference conditions63859/63860/67252/67254; Darkshore tasks also reference completed quest51722. The source task/condition joins are not wired into runtime eligibility. Missing-key sections were recorded once in the extraction logs; selected task rows remain readable. Historical QuestPOIPointCliTask1028732 and QuestObjectiveCliTask1028734 are absent from this build's local root.

All four templates have AreaGroupID=0 and no TDB `areatrigger_involvedrelation` row. Each DB2 task pin is a single point. None of their four boss entries has a TDB creature spawn or a promoted runtime template. QuestSortID9734/10288 identifies an authored category/zone; it does not establish a Retail acceptance radius. No circle was guessed around a map pin and no zone-wide auto-accept policy was silently substituted.

AreaPOI decoding initially produced25 values against24 declared columns: the decoder appended a continent relationship already stored inline. The source schema declares the inline parent; the repaired decoder verifies it against the relationship map instead of duplicating it. All3,909 rows validate, alongside1,070 AreaPOIState and154 ManagedWorldStateBuff rows. Joins by quest IDs/current-stage IDs/boss names produce zero AreaPOIs, zero linked states and zero matching managed buffs. TrinityCorea352b1fa's `HandleRequestWorldQuestUpdate` is explicitly unimplemented; it cannot supply the remaining active-task/area behavior.

Area-entry acceptance, automatic currency/reputation/spell reward settlement, availability transport and native pins/tooltips/task tracking remain unimplemented. No UDP5861 server/client was started and no PNG proof was produced. Scheduling is no longer the blocker; the remaining Retail task-area/eligibility boundary is unresolved. This page does not mark the whole goal complete.

## Sources

- `/home/osso/Projects/world-of-osso/game-server/data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql`, SHA-256 `608534d1dc97535c92d756914a02fe36264e484b519f6e54f6d10cf56c6f4d96`.
- Read-only `/home/osso/Projects/world-of-osso/game-server/data/world.db` and pinned `/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933/QuestInfo.csv`; matching quest ID/title/managed-state/scheduler/time-limit fields also checked directly against the TDB dump.
- Raw local CASC files under diagnostic `db2/`, `casc-extract.log`, and exact-layout primary [ManagedWorldState definition](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/ManagedWorldState.dbd) (A4E9EA9F) establish the duration-field meanings; no quest/availability data was sourced online.
- Reproducible read-only census: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/world-quests-2026-10-11/audit_sources.py`; machine-readable receipt: adjacent `source-census.json`. Run with `python3 <audit path>`; it writes only the adjacent receipt.
- Local Retail Lua paths cited above under `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/`.

## See Also

- [World map contract](../../specs/world-map.md)
- [Quest sharing, resets and timers](../systems/quest-share-resets.md)
