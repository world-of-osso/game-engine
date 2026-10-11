# Scenario progression and tracker

Local Retail definitions drive copy-scoped stage state. Client skins consume the same authoritative replacement snapshot; contracts: [client](../../specs/scenarios.md), [server/data importer](../../../../game-server/docs/specs/scenarios.md).

## Flow

Server scenario definitions join ScenarioStep to CriteriaTree/Criteria and local TDB map/difficulty/faction bindings. Only ordinary, unconditional, supported main steps are enabled; bonus steps do not advance the main stage. Existing achievement evaluator supplies event matching, counters, tree completion and criterion labels. Scenario state is separate from character achievement history and keyed by InMap, so re-entering a live copy retains its progress and another copy starts fresh.

The achievement event boundaries notify scenario state for ordinary NPC death, scripted encounter defeat, quest reward and cast/target/skill events. Successful active-tree completion increments the stage once; the index beyond the final main step produces completion. No client criterion or completion requests exist.

InstanceChannel carries ScenarioProgress. Account dispatch clears on NewWorld/disconnect and applies copy-scoped replacements. ObjectiveTrackerState renders the scenario before quests and suppresses duplicate generic dungeon objectives. Authored Flags0x2 controls numbered-stage suppression. Retail atlas `evergreen-scenario-trackerheader` is resolved through the active skin.

## Coverage boundary

Gnomeregan649 has one main stage1176 (normal map90), KillCreature7800. Its four bonus steps are not synthesized into a sequential scenario. A two-stage service test uses explicitly synthetic composition of authentic criterion leaves; it is not proof of an authored two-stage scenario playthrough. Standalone scenario-map population, unsupported scripted/world-state conditions, bonus tracker, reward quests and transition animation/sound remain gaps. Live proof remains pending until both-skin private captures are recorded.

## Sources

- [Scenario implementation](../../../../game-server/crates/server/src/scenario.rs), [achievement evaluator](../../../../game-server/crates/server/src/achievement_catalog.rs)
- [Client reducer](../../../godot/ui-model/src/scenario_progress.rs), [tracker](../../../godot/ui-model/src/ui/screens/objective_tracker_component.rs)
- Local Retail `Blizzard_ScenarioObjectiveTracker.lua/.xml` and GlobalStrings.csv; source behavior recorded in the client contract.

## See Also

- [[boss-encounters]] — scripted boss credit uses its established completion boundary.
