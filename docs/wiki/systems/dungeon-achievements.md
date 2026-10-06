# Dungeon objectives and AchievementFrame

Native client consumes copy snapshots and on-demand catalog pages; the server remains authoritative. [Feature contract](../../specs/dungeon-achievements.md).

## State and rendering

`godot/network/src/lib.rs` subscribes to DungeonProgress on InstanceChannel and catalog/live state on AchievementChannel. `account.rs` dispatches decoded messages into `DungeonObjectives` and `AchievementWindow`, and queues page continuations. The frame tick sends requests through NetworkBridge on AchievementChannel.

Dungeon snapshots carry no dungeon name: account dispatch resolves `MapName_lang` from the local pinned Map.csv. The first snapshot fixes map/copy/difficulty identity until a transfer. The Scenario-style module precedes watched quests; their order and the container anchor stay unchanged. Empty snapshots and disconnect clear objectives. Unknown optionality is not labeled mandatory.

AchievementWindow retains categories, achievement and criteria pages. The native RegistryUi inserts the complete cache into SharedContext; generation updates rebuild its screen. Categories use parent/order, achievements use exclusive ID cursors, criteria merge by tree ID and sort by authored order. Navigation shows bounded local slices with explicit Previous/More controls; catalog pages arrive only on demand. Earned dates render UTC; absent dates stay unknown. The points header sums only loaded earned rows, not an invented account total. Live updates invalidate cached categories and request the visible one again.

Launcher and micro-menu share AchievementMicroButton dispatch. ToggleAchievements uses persisted bindings. CloseAllWindows owns Escape. The Godot branch previously had no achievement subscription/toast host; completed wire payloads now queue five-second native alerts.

Modern uses local UI-Achievement-Category-Background (130652), AchievementBackground (235397), Shield (130665) and Alert-Background (130650). Forever uses active-skin flat panel chrome and its metal sheet. Assets are local CASC/cache only.

## Sources

- Retail `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:202,379–416`: dungeon/stage metadata and complete checks/incomplete nubs.
- Retail `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua:572,1814,1952`: categories, achievement metadata and criterion text/completion/counters.
- Read-only server `docs/dungeon-achievements-client.md`, protocol `achievement_catalog_messages.rs`, pinned shared-protocol 172fd26.
- Client `godot/ui-model/src/achievements.rs`, `dungeon_progress.rs`, `godot/rust/src/achievements.rs`.

## See Also

- [Quest UI](../../specs/quest-ui.md) — shared tracker and panel primitives.
- [Build hosts](../../remote-builds.md) — locked local helper and private rendering boundary.
