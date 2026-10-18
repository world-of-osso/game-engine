# World quests

Retail Mainline world quests with authoritative availability, POI-area acceptance, existing quest objective progress, automatic completion and authentic rewards. Modern and Forever share behavior; Forever is art only. Deterministic rotation and duration defaults are explicit World of Osso policy, not Retail data. User decisions:2026-10-11.

## What it must do

- [ ] Every world quest with authentic QuestPOIBlob/QuestPOIPoint polygon geometry and authentic objectives can rotate. Single-point pins are not polygons; missing blobs are ineligible and counted. Use blob MapID/UiMapID for task area and map pin.
- [x] Entering an active task polygon auto-accepts/tracks it. Leaving untracks without losing progress. Expiry removes active task and watch state.
- [x] Complete counted objectives through the existing quest system; automatically settle authentic TDB quest_template/addon/reward items, money, XP, currency and reputation only. No invented rewards. Quests without reward data complete without reward and are counted. One completion per quest per active window.
- [x] Availability snapshots carry active IDs/zone, every authored area's world map/UI map and representative point, shared expiry and rewards preview. Multiple areas remain one task/window/reward.
- [x] Native Mainline-style world-map pins/tooltips show source time units/rewards; task tracker entries retain counted text, only authored progressbars and low-time warnings; acceptance/completion produce toasts. Local CASC art, both skins.
- [x] Historical2026-10-11 controlled private UDP5861 proof completes real86941 with administrator-supplied authentic items and real86948 with administrator-driven authored NPC46327 deaths, in Modern/Forever using fb_* accounts. Those historical matching binaries show7/8→automatic completion/11765XP and living actors; Modern relog has no task. Inspected1280px PNGs under `/syncthing/AgentShared/2026-10-11/world-quests/`. Natural gathering/player combat is not certified.
- [x] Integrate origin/master, targeted tests/check/fmt, focused protocol wire/layout checks when changed, explicit branch push and final census/SHAs/import command. Canonical world.db and UDP5000 untouched; protocol overrides uncommitted; patch blocks retained.

## Active-task navigation and source tracker

Received, unexpired world-quest pins accept a plain click as a client-local super-tracking toggle, without accepting a quest or granting progress/rewards. Selection survives while the received task remains available, including before polygon entry; completion/expiry removes it. Minimap placement uses the received authentic map/position, rotation and skin mask; nearby tasks have an unnumbered world-quest marker, while only the selected offscreen task has a rim direction arrow. Accepted tasks do not also become ordinary numbered quest POIs. This minimap composition is an engine presentation using source art, not a claim about the opaque Retail minimap renderer. Manual/Automatic remote world-quest watch semantics are not invented by the local navigation toggle.

The World Quests tracker shows counted objectives as text, not a synthesized aggregate bar. Only an unfinished authored progressbar objective creates a percentage-only bar while retaining its authored objective text. Task tracker POIs retain the source normal POI background beneath the WorldQuest icon, without ordinary quest-number text. Availability windows are not ordinary quest countdowns: show a low-time warning below fifteen minutes, after objective text/before a progressbar, rather than a day-long minute counter. Low-time abbreviations follow the installed GlobalStrings `MINUTES_ABBR` (`Min`); tooltip durations use source day/hour/minute boundaries and at most two units. Existing realm polygon acceptance and actual completion/reward settlement remain authoritative.

## Multiple authored areas and floor maps

- Preserve all admitted source areas and their source X/Y/Z, not only the first area or its horizontal coordinates. Task82521 has lower2256/main2255 map points and negative source heights under one identity, deadline and settlement.
- Map projection retains its real-height floor discrimination, using the received source Z rather than inventing zero or bypassing the height check. Pins use the area's assigned UI map or its navigable parent, not an unrelated sibling with overlapping world bounds; a displayed map gets at most one pin per task, using published area order when multiple areas qualify.
- Horizontal minimap navigation chooses the nearest published area on the player's current world map, with no duplicate task markers. Opening a watched task retains its first-published authored map. These are project presentation policies, not claims about opaque Retail C API area selection.
- Rebuild private/runtime area data through the importer before running the new schema. Old XY-only area data must fail explicitly, not receive a fallback Z.
- Keep other active tasks when a snapshot removes one; exact expiry and realm-policy gaps remove tasks until the next real window. Reconnection receives current windows and all current areas.
- Reward choice is not applicable to the239 imported tasks: all six source `RewardChoiceItemID` columns are zero. Do not build or infer a world-quest choice path without a sourced quest and selection authority.

## Current controlled proof — 2026-10-16

- [x] Actual Modern/Forever accounts36/37 prove physical available-pin selection without acceptance, minimap direction, polygon arrival, real46327 deaths7/8→8/8 and automatic11765XP delivery with exact acknowledgement, log clearing and same-window replay rejection. Separate real-clock runs prove low-time warning and expiry removal. Engine3c08c72fd/server2dadf94e, private5771;26 individually inspected fullHD PNGs under `/syncthing/AgentShared/2026-10-16/world-quests/{3c08c72fd-window16,3c08c72fd-controlled17}/`.
- This is an explicitly controlled fixture omitting only Deathwing spawn20383796, approved by the lead after actual LastDamager diagnosis. Objective actors, positions and death/settlement pipelines are unchanged; no NPC health restoration or credit injection. Product data keeps Deathwing. **World-event hazard instakills objective NPCs; Retail phasing or scripting source not present in allowed sources.** Natural combat/world-terrain parity remains uncertified; the earlier item proof predates current quest-bound Flags2 semantics. [Inputs and limits](../wiki/investigations/world-quest-availability.md#current-controlled-navigationwindowreward-proof).

## World of Osso policy

- `WORLD_QUEST_EPOCH`: Unix seconds, default1767225600 (2026-01-01 UTC). Nothing active before it.
- `WORLD_QUEST_ORDER`: ascending default or descending quest IDs within signed QuestSortID groups (positive zones, negative categories).
- `WORLD_QUEST_ACTIVE_PER_ZONE`: positive concurrent capacity, default1; round-robin independent lanes, capped by candidate count.
- `WORLD_QUEST_GAP_SECONDS`: nonnegative gap, default86400. A lane cycles half-open `[start,start+duration)` windows separated by gaps.
- Import `--policy-duration`: positive seconds, default86400. Authentic ManagedWorldState UpTimeSecs takes precedence; otherwise this realm policy supplies duration. Never label it Retail data.

## Coverage and remaining whole-goal gap

| Capability | Current coverage | Proof |
| --- | --- | --- |
| Polygon admission/rotation | Source polygons, pinned duration or policy default | Python import and deterministic rotation tests |
| Objective integration | Dynamic TDB0monster/1item/2game object/3TalkTo/9player kills/14single-leaf criteria/15weighted bar/21label kills; authored ordering, flags, Flags2 and weights | 23 bounded server behavior tests,12 importer tests; controlled real item/creature completion |
| Remaining authentic objectives |239/244 admitted;3 area-trigger quests and2 absent labels remain excluded; **whole coverage is open**, not an approved exclusion policy | [Exact census](../wiki/investigations/world-quest-availability.md#current-poi-census) |
| Completion/replay/expiry | Stored task history; items/money/numeric rewards transaction; pinned QuestXP | Host-message/persistence tests; controlled both-skin live completion |
| Native presentation | Pins/tooltips, separate World Quests module, timer/bar/toasts | Both-skin UI test plus inspected native PNGs |

The existing-system representational boundary is an implementation/source gap, not absence of authentic objectives or an approved content exclusion. The2026-10-16 continuation requires each of the five gaps to be resolved from allowed sources or explicitly named; [the server source audit](../../../game-server/docs/world-quests.md#five-admission-gaps--verified-2026-10-16) records each quest and missing binding/producer. Natural objective acquisition/player combat is not certified by administrator-seeded completion. Living actors are demonstrated during controlled creature-credit completion, not ordinary unassisted gameplay.

## Sources and implementation

- [Source/implementation investigation](../wiki/investigations/world-quest-availability.md)
- [Server import command and runtime](../../../game-server/docs/world-quests.md)
- [Existing quest UI](../wiki/systems/quest-ui.md)

## Tests

Server `world_quests::tests`, `world_quests::expiry_tests`, and `quest::tests::world_quest_area_entry_progress_reward_window_replay_and_expiry`; Python `test_import_world_quests`; protocol `world_quest_availability_round_trip_preserves_window_areas_and_reward_payloads`; UI integration `world_quests::world_quests_clock_tooltip_progress_and_expiry_both_skins`.

## Out of scope

Invented rewards/geometry, claiming policy scheduling/durations as Retail data, canonical database writes, UDP5000 operations, committed protocol overrides, PLAN edits and full server/engine suites.
