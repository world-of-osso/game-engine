# World quests

Retail Mainline world quests with authoritative availability, POI-area acceptance, existing quest objective progress, automatic completion and authentic rewards. Modern and Forever share behavior; Forever is art only. Deterministic rotation and duration defaults are explicit World of Osso policy, not Retail data. User decisions:2026-10-11.

## What it must do

- [ ] Every world quest with authentic QuestPOIBlob/QuestPOIPoint polygon geometry and authentic objectives can rotate. Single-point pins are not polygons; missing blobs are ineligible and counted. Use blob MapID/UiMapID for task area and map pin.
- [x] Entering an active task polygon auto-accepts/tracks it. Leaving untracks without losing progress. Expiry removes active task and watch state.
- [x] Complete counted objectives through the existing quest system; automatically settle authentic TDB quest_template/addon/reward items, money, XP, currency and reputation only. No invented rewards. Quests without reward data complete without reward and are counted. One completion per quest per active window.
- [x] Availability snapshots carry active IDs, map/zone, authored POI position, expiry and rewards preview.
- [x] Native Mainline-style world-map pins/tooltips show time left/rewards; task tracker entries show progress bar/timer; acceptance/completion produce toasts. Local CASC art, both skins.
- [x] Controlled private UDP5861 proof completes real86941 with administrator-supplied authentic items and real86948 with administrator-driven authored NPC46327 deaths, in Modern/Forever using fb_* accounts. Current matching binaries show7/8→automatic completion/11765XP and living actors; Modern relog has no task. Inspected1280px PNGs under `/syncthing/AgentShared/2026-10-11/world-quests/`. Natural gathering/player combat is not certified.
- [x] Integrate origin/master, targeted tests/check/fmt, full protocol suite when changed, explicit branch push and final census/SHAs/import command. Canonical world.db and UDP5000 untouched; protocol overrides uncommitted; patch blocks retained.

## Active-task navigation and source tracker

Received, unexpired world-quest pins accept a plain click as a client-local super-tracking toggle, without accepting a quest or granting progress/rewards. Selection survives while the received task remains available, including before polygon entry; completion/expiry removes it. Minimap placement uses the received authentic map/position, rotation and skin mask; nearby tasks have an unnumbered world-quest marker, while only the selected offscreen task has a rim direction arrow. Accepted tasks do not also become ordinary numbered quest POIs. This minimap composition is an engine presentation using source art, not a claim about the opaque Retail minimap renderer. Manual/Automatic remote world-quest watch semantics are not invented by the local navigation toggle.

The World Quests tracker shows counted objectives as text, not a synthesized aggregate bar. Only an unfinished authored progressbar objective creates a percentage-only bar while retaining its authored objective text. Task tracker POIs use the unnumbered normal WorldQuest icon, not the ordinary quest-number plate. Availability windows are not ordinary quest countdowns: show a low-time warning below fifteen minutes, after objective text/before a progressbar, rather than a day-long minute counter. Low-time abbreviations follow the installed GlobalStrings `MINUTES_ABBR` (`Min`); tooltip durations use source day/hour/minute boundaries and at most two units. Existing realm polygon acceptance and actual completion/reward settlement remain authoritative.

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

The existing-system representational boundary is an implementation gap, not absence of authentic objectives. Extending it or approving exclusions needs reconciliation with the user's existing-system-only instruction; no narrower goal has been accepted. Natural objective acquisition/player combat is not certified by administrator-seeded completion. Living actors are now demonstrated during controlled creature-credit completion, not ordinary unassisted gameplay.

## Sources and implementation

- [Source/implementation investigation](../wiki/investigations/world-quest-availability.md)
- [Server import command and runtime](../../../game-server/docs/world-quests.md)
- [Existing quest UI](../wiki/systems/quest-ui.md)

## Tests

Server `world_quests::tests`, `world_quests::expiry_tests`, and `quest::tests::world_quest_area_entry_progress_reward_window_replay_and_expiry`; Python `test_import_world_quests`; protocol `world_quest_availability_round_trip_preserves_window_area_and_authentic_rewards`; UI integration `world_quests::world_quests_clock_tooltip_progress_and_expiry_both_skins`.

## Out of scope

Invented rewards/geometry, claiming policy scheduling/durations as Retail data, canonical database writes, UDP5000 operations, committed protocol overrides, PLAN edits and full server/engine suites.
