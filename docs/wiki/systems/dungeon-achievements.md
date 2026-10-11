# Dungeon objectives and AchievementFrame

Native client consumes copy snapshots and on-demand catalog pages; the server remains authoritative. [Feature contract](../../specs/dungeon-achievements.md).

## State and rendering

`godot/network/src/lib.rs` subscribes to DungeonProgress on InstanceChannel and catalog/live state on AchievementChannel. `account.rs` dispatches decoded messages into `DungeonObjectives` and `AchievementWindow`, and queues page continuations. The frame tick sends requests through NetworkBridge on AchievementChannel.

Dungeon snapshots carry no dungeon name: account dispatch resolves `MapName_lang` from the local pinned Map.csv. The first snapshot fixes map/copy/difficulty identity until a transfer. The Scenario-style module precedes watched quests; their order and the container anchor stay unchanged. Empty snapshots and disconnect clear objectives. Unknown optionality is not labeled mandatory.

AchievementWindow retains categories, achievement and criteria pages. The native RegistryUi inserts the complete cache into SharedContext; generation updates rebuild its screen. Categories use parent/order, achievements use exclusive ID cursors, criteria merge by tree ID and sort by authored order. Navigation shows bounded local slices with explicit Previous/More controls; catalog pages arrive only on demand. Earned dates render UTC; absent dates stay unknown. Summary is the initial view. Its points, full eligible counts, category-ancestor totals and newest three earned rows come from the server, independently of loaded pages. All/Account/Character tabs use the authentic 0x20000 account flag. Live updates invalidate Summary and cached categories and request the visible category, selected own-state detail or active search again.

Tracking uses a selected row's native Track/Stop tracking control; only unearned achievements can be watched (maximum ten). The native ObjectiveTracker inserts its Achievements module after quests, with server criterion counters and unevaluated labels. Account/realm/character-local preferences store IDs only; re-entry requests authoritative rows, and live updates refresh watched IDs even when AchievementFrame is closed. Newly earned rows stop tracking.

Statistics uses the authentic flag1 catalog and raw, unclamped supported counters. Unsupported statistic types remain Unevaluated. Search queries the full faction/scope-eligible server catalog by title/description (including statistics), not client-loaded pages; results are term/scope correlated and ID-cursor paged. Native links in the earned chat line and toast open the viewer's own server state. Shift-click inserts ID/name markup at the existing chat caret; no invented GUID/comparison payload. Comparison has no server endpoint.

Launcher and micro-menu share AchievementMicroButton dispatch. ToggleAchievements uses persisted bindings. CloseAllWindows owns Escape. The Godot branch previously had no achievement subscription/toast host; completed wire payloads now queue six-second native alerts.

Modern uses local UI-Achievement-Category-Background (130652), AchievementBackground (235397) and Shields (130666; plural file named by Mainline XML, not the different singular130665 asset). Forever uses active-skin flat panel chrome and its metal sheet. Earned alerts use Mainline's 300×101 template, atlas1771/FDID4035524 regions, authoritative earned icon, 52×52 icon, point shield and centered title/name; both skins preserve the same elements. The legacy uncropped FDID130650 sheet is not an alternate alert path. Category button backgrounds, icon frames, point shields and achievement row backgrounds crop the Mainline XML sheet regions; a category-button strip is never stretched across the sidebar. Summary captions are children of their bars, drawn after fills so completed pixels cannot cover text. Assets are local CASC/cache only.

## Proof

Verified: 2026-10-06. Code `ada3c295`, read-only protocol `172fd266`, read-only server `dc7df543`.

- Locked local helper, packages `game-engine-godot`, `game-engine-ui-model`, `game-engine-network`, filter `dungeonclient`: **8/8 pass** (5 UI-model, 2 account, 1 real ephemeral UDP), `/tmp/claude/dungeonclient-last-tests.out`. Covers both-skin Stockade kill/leave rendering, catalog and criteria continuation, launcher request, live refresh during an outstanding query and reopening, account dispatch/toast, actual AchievementChannel wire order.
- Native extension build passes, `/tmp/claude/dungeonclient-last-build.out`; SHA-256 `7b05abaac3e84fb33e17110924e1a9152d87268014603a4e5f1b857c0ae218d1`. Changed Rust files pass format checking. No broad suite or live server test was run.
- Existing `capture_ui_screen.gd` generates `data/diagnostics/dungeonclient-2026-10-06/{modern-tracker,forever-tracker,modern-achievements,forever-achievements,achievement-toast}.png`. All five exit 0 and were inspected: ordered three-boss mid-run block, first boss checked; both panel skins, category parent/child, icon, name, description, 10 points, UTC date and Hogger criterion readable; alert visible. Approved tracker anchor unchanged. Initial low-contrast Modern capture retained as `modern-achievements-low-contrast.png`; rejected and corrected with dark row ink.
- Captures use the available trial Godot `4.7.2.stable.pr123946.ed1daf0bf`, private headless Weston/Dozen and local CASC. Owned compositor stopped by exact PID after capture. Capture teardown reports texture/font RID leaks; clean-resource shutdown is not proven. Live instance gameplay and server-triggered grant-to-toast end-to-end proof remain untested (offline task).

Lightyear's typed MessageSender queues messages per type before serialization (`lightyear_messages-0.28.0/src/send.rs:90–150`): method-call order across types is not wire order. The UDP fixture flushes server sends separately and pauses the client worker to reproduce one-frame mixed-type reception, then asserts the actual channel sequence. A single client relay sorts message IDs before account dispatch.

## Live Stockade proof (2026-10-06)

Client **70bed793**, server **a53edf9**, protocol **f1d0452**: a fresh private database copied from the approved offline backup, UDP5292, account `fb_stockade`, level90 Holy paladin **Fbpalrun**. One map34 copy: player killed Randolph Moloch, Lord Overheat, then Hogger using normal attacks and self-healing. Admin operations supplied only character/gear setup and positioning/revive; no admin boss damage. Server files and UDP5000 remained untouched.

Inspected [run2 screenshots](../../../data/diagnostics/stockaderun-2026-10-06/run2/): `01-entry.png` has three unchecked objectives; `02-randolph-killed.png`, `03-overheat-killed.png`, `04-hogger-killed.png` show cumulative checks. `05-final-toast.png` shows the live server-triggered 10-point Stockade alert. `06-achievement633-earned.png` shows **Earned 2026-10-06 (UTC)** and Hogger1/1. `08-left-inworld-cleared.png` shows no dungeon tracker after transfer to map0. Combat logs and read-only boss health observations are in the same directory.

The run also reproduced a [waist binding defect](../investigations/npc-stance-gear.md#animated-and-nonzero-geoset-waist-follow-up-2026-10-06) that hid the paladin model; these original screenshots prove objective/achievement behavior, not correct character rendering. Current-revision fix proof and process cleanup are tracked in that directory's `proof-ledger.txt`.

## Achievement UI continuation — verified 2026-10-11

Protocol `7c4c21a`, server `97d589f`, client implementation `2729912df`, branch `achievements-ui`; no master merge. Private UDP5730, existing `fb_achievementsui` / `Fbachui`, isolated diagnostic redb. No world.db writes or shared realm operations.

- Filtered proof:15 UI-model cases,3 native account/persistence cases and1 ephemeral UDP case across current relevant scoped runs;13 real-catalog server cases. Native extension/CLI/server builds, targeted cargo checks and all three format checks pass. No full suite or CI certification claimed.
- Current Modern/Forever `*-v8-*` PNGs show Summary/scopes, raw statistic1068 quantity0, full-catalog633 search, Shift insertion and own-state chat-link navigation. Header60 points, All6/8078, Account0/4613, Character6/3465 after documented setup. Progress caption pixels remain visible over filled bars; corrected shield is the XML's plural FDID130666, not singular130665.
- Modern `modern-v6-*` and Forever `forever-v7-*` retain authentic50/60 criterion progress, tracker after close, server-triggered earned alert and popup navigation to earned own-state details. Explicit private admin set-level is setup, not gameplay-leveling proof. Modern session relog restored watch8; completion removes saved watches in both skins. Both preferences end with empty ID lists.
- Evidence: [diagnostics](../../../data/diagnostics/achievements-ui-2026-10-11/), user PNGs `/syncthing/AgentShared/2026-10-11/achievements-ui/`. Full captures1280×720; small review crops/downsized copies (<1MiB, dimensions≤1280) were inspected. Earlier full-file tool reads were truncated and are not pixel-inspection evidence.

Known data boundary: unsupported criterion/statistic types remain unevaluated. Comparison has no authoritative endpoint; InspectSnapshot carries equipment and talents only. The plain native draft edit box displays link markup; sent chat displays a clickable name.

## Sources

- Retail `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:202,379–416`: dungeon/stage metadata and complete checks/incomplete nubs; `Blizzard_ObjectiveTrackerManager.lua:196`: Scenario precedes Quests.
- Retail `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua:572,1814,1952`: categories, achievement metadata and criterion text/completion/counters.
- Read-only server `docs/dungeon-achievements-client.md`, protocol `achievement_catalog_messages.rs`, pinned shared-protocol 172fd26.
- Client `godot/ui-model/src/achievements.rs`, `dungeon_progress.rs`, `godot/rust/src/achievements.rs`.
- Retail `Blizzard_FrameXML/Mainline/AlertFrameSystems.xml:173–330`, `AlertFrameSystems.lua:348–419` — earned plate/icon/shield layout, click navigation and queued duration.
- Retail `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.xml:632,1091,1113,2002` — category-button and row sheet UVs; local community-listfile maps plural Shields to130666.
- Retail `Blizzard_AchievementUI.lua:1616–1643,2235,3447–3460,3632–3648` and `AchievementObjectiveTracker.lua:97–193` — tracking, statistic quantity and search/navigation.

## See Also

- [Quest UI](../../specs/quest-ui.md) — shared tracker and panel primitives.
- [Build hosts](../../remote-builds.md) — locked local helper and private rendering boundary.
