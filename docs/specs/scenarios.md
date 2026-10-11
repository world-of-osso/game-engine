# ScenarioObjectiveTracker

Retail Mainline owns behavior and elements for both Modern/Forever. Forever supplies art only.

Server copy-scoped ScenarioProgress replaces the current scenario and clears it outside supported scenario copies. Current stage, title, criteria counters and completion are authoritative. Character achievement history never initializes a scenario run. NewWorld/disconnect clears client state; late snapshots from another map/copy are rejected.

The tracker appears before quests, replacing the generic dungeon objective list when a scenario snapshot exists. Header uses Scenario.Name except Type3 (dungeon display), which uses `Dungeon`. Stage block uses Retail evergreen scenario header art, current numbered stage or `Final Stage`, stage name and criterion check/nub rows. Scenario.Flags bit0x2 suppresses numbered stage text and displays the stage title alone. CurrentStage>NumStages shows `Dungeon Complete!` for Type3, otherwise `Completed!`, and hides criteria.

Chosen local scenario: Gnomeregan649/map90/difficulty1, authored one main stage1176. No fabricated stage splitting. Imported four bonus steps are not main-stage transitions; separate bonus-objective tracker, scenario reward quests, transition animation/sound and widget-driven scenarios remain unimplemented. Source: local Retail Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua and .xml; GlobalStrings.csv. Data/import contract lives in game-server docs/specs/scenarios.md.

Acceptance: targeted both-skin registry test, owner account message dispatch and actual private UDP5810 entry/kill/completion with inspected PNGs. Registry/model evidence is not a live playthrough.
