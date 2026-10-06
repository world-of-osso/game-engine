# Dungeon objectives and AchievementFrame

Native client consumes copy snapshots and on-demand catalog pages; the server remains authoritative. [Feature contract](../../specs/dungeon-achievements.md).

## State and rendering

`godot/network/src/lib.rs` subscribes to DungeonProgress on InstanceChannel and catalog/live state on AchievementChannel. `account.rs` dispatches decoded messages into `DungeonObjectives` and `AchievementWindow`, and queues page continuations. The frame tick sends requests through NetworkBridge on AchievementChannel.

Dungeon snapshots carry no dungeon name: account dispatch resolves `MapName_lang` from the local pinned Map.csv. The first snapshot fixes map/copy/difficulty identity until a transfer. The Scenario-style module precedes watched quests; their order and the container anchor stay unchanged. Empty snapshots and disconnect clear objectives. Unknown optionality is not labeled mandatory.

AchievementWindow retains categories, achievement and criteria pages. The native RegistryUi inserts the complete cache into SharedContext; generation updates rebuild its screen. Categories use parent/order, achievements use exclusive ID cursors, criteria merge by tree ID and sort by authored order. Navigation shows bounded local slices with explicit Previous/More controls; catalog pages arrive only on demand. Earned dates render UTC; absent dates stay unknown. The points header sums only loaded earned rows, not an invented account total. Live updates invalidate cached categories and request the visible one again.

Launcher and micro-menu share AchievementMicroButton dispatch. ToggleAchievements uses persisted bindings. CloseAllWindows owns Escape. The Godot branch previously had no achievement subscription/toast host; completed wire payloads now queue five-second native alerts.

Modern uses local UI-Achievement-Category-Background (130652), AchievementBackground (235397), Shield (130665) and Alert-Background (130650). Forever uses active-skin flat panel chrome and its metal sheet. Assets are local CASC/cache only.

## Proof

Verified: 2026-10-06. Code `ada3c295`, read-only protocol `172fd266`, read-only server `dc7df543`.

- Locked local helper, packages `game-engine-godot`, `game-engine-ui-model`, `game-engine-network`, filter `dungeonclient`: **8/8 pass** (5 UI-model, 2 account, 1 real ephemeral UDP), `/tmp/claude/dungeonclient-last-tests.out`. Covers both-skin Stockade kill/leave rendering, catalog and criteria continuation, launcher request, live refresh during an outstanding query and reopening, account dispatch/toast, actual AchievementChannel wire order.
- Native extension build passes, `/tmp/claude/dungeonclient-last-build.out`; SHA-256 `7b05abaac3e84fb33e17110924e1a9152d87268014603a4e5f1b857c0ae218d1`. Changed Rust files pass format checking. No broad suite or live server test was run.
- Existing `capture_ui_screen.gd` generates `data/diagnostics/dungeonclient-2026-10-06/{modern-tracker,forever-tracker,modern-achievements,forever-achievements,achievement-toast}.png`. All five exit 0 and were inspected: ordered three-boss mid-run block, first boss checked; both panel skins, category parent/child, icon, name, description, 10 points, UTC date and Hogger criterion readable; alert visible. Approved tracker anchor unchanged. Initial low-contrast Modern capture retained as `modern-achievements-low-contrast.png`; rejected and corrected with dark row ink.
- Captures use the available trial Godot `4.7.2.stable.pr123946.ed1daf0bf`, private headless Weston/Dozen and local CASC. Owned compositor stopped by exact PID after capture. Capture teardown reports texture/font RID leaks; clean-resource shutdown is not proven. Live instance gameplay and server-triggered grant-to-toast end-to-end proof remain untested (offline task).

Lightyear's typed MessageSender queues messages per type before serialization (`lightyear_messages-0.28.0/src/send.rs:90–150`): method-call order across types is not wire order. The UDP fixture flushes server sends separately and pauses the client worker to reproduce one-frame mixed-type reception, then asserts the actual channel sequence. A single client relay sorts message IDs before account dispatch.

## Sources

- Retail `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:202,379–416`: dungeon/stage metadata and complete checks/incomplete nubs; `Blizzard_ObjectiveTrackerManager.lua:196`: Scenario precedes Quests.
- Retail `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua:572,1814,1952`: categories, achievement metadata and criterion text/completion/counters.
- Read-only server `docs/dungeon-achievements-client.md`, protocol `achievement_catalog_messages.rs`, pinned shared-protocol 172fd26.
- Client `godot/ui-model/src/achievements.rs`, `dungeon_progress.rs`, `godot/rust/src/achievements.rs`.

## See Also

- [Quest UI](../../specs/quest-ui.md) — shared tracker and panel primitives.
- [Build hosts](../../remote-builds.md) — locked local helper and private rendering boundary.
