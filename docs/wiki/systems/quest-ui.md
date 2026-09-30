# Quest UI

Requirements: [quest UI spec](../../specs/quest-ui.md). Server side: game-server `docs/wiki/systems/quests.md`, `npc-interaction.md`.

## Data flow

- `game_engine::quest_runtime::QuestRuntime` (lib) is the client quest model: log (`QuestEntrySnapshot`s in server order), watch list, marker per NPC (server entity bits), and the open `QuestDialog` (`Greeting` / `Detail` / `Progress` / `Reward{choice}`). Reducers are pure methods returning Retail chat lines.
- `networking_quests::QuestNetworkPlugin` (`src/game/networking/quests.rs`) registers two dispatcher handlers: log (`QuestLogSnapshot`, `QuestLogUpdate`, `QuestGiverStatusMultiple`, also refreshing the IPC `QuestLogStatusSnapshot`) and dialog (interaction + quest giver messages). Within one batch the dialog handler applies `QuestGiverQuestComplete` before `QuestGiverQuestDetails`, because a turn-in's chain offer can arrive in the same batch.
- UI and IPC write `NpcInteractionRequest` messages; `send_interaction_requests` turns them into protocol messages. `Interact(Entity)` maps the main-world entity to its server entity with `ReplicationMirrorMap`.
- `NpcFlags` is mirrored from the replication worker (`network_runtime/replication.rs`). `query_new_quest_givers` queries markers for newly mirrored quest givers (queued until connected); the server pushes changes afterwards.

## Screens

`src/scenes/quest_ui/` builds three `Screen`s on one `SharedContext`: tracker (`objective_tracker_component`), log (`quest_log_frame_component`) and giver frame (`quest_frame_component`). View models come from `view.rs`; `actions.rs` maps `onclick` actions (`quest_tracker:*`, `quest_log:*`, `quest_frame:*`) to state changes and requests. `sync_quest_giver_window` keeps the `WindowId::QuestGiver` window and the dialog together (dialog opens window; window closed by the manager → `CloseInteraction`; server close → window closes). The log is `WindowId::QuestLog`, toggled by `InputAction::ToggleQuestLog` (L).

Art: `quest_art.rs` (atlas members with FDIDs, rock background, close button, panel buttons). The metal border is the shared `metal_frame` panel style (`ui/panel_styles.rs`): the toolkit NineSlice takes one texture with per-part UVs, so startup composes the `PortraitFrameTemplate` pieces from atlases 1390/1394/1395 into one 364×246 sheet (3×3 grid, columns 150|64|150, rows 150|32|64 atlas px, shown at half size). Retail's 32-wide bottom corners sit under 75-wide side edges, so each bottom corner cell continues with bottom-edge art. Windows put the style on a frame `METAL_FRAME_OUTSET` (13, 16, 4, 3) larger than themselves. Wrapped text: the toolkit has no `justify_v` attribute and centres a fontstring's first line vertically, letting wrapped lines flow below unclipped, so text frames are one line tall and layout advances by `wrapped_text_height` (greedy wrap over the original spacing with `measure_text`, line pitch from the font metrics).

Markers: `nameplate.rs` spawns the talktome M2 with its animation; `indicator_facing` yaws the glyph's +X face toward the camera (the `?` model is flat and reads as `!` edge-on).

## Gotchas

- InWorld automation clicks must set the cursor at UI position × `ui_scale` (fixed in `automation_inworld.rs`); `--screen inworld` with `--run-js-ui-script` keeps the auto-login.
- `game-server-admin set-position` only moves an online character server-side; the client keeps its own position, so move characters while logged out.
- The quest log's description and rewards come from a session cache of detail pages (`QuestLogData.details`): the log snapshot has neither.

## Godot client

The native client ports only the objective tracker so far.
- `godot/rust/src/objective_tracker.rs` feeds the shared `objective_tracker_component` through `ObjectiveTrackerState::from_watched`. The input is the `Account.quest_log` entries in `Account.quest_watched` order. `from_runtime` is the Bevy-only wrapper, under `cfg(not(godot_host))`.
- The two minimize buttons toggle `collapsed` and `quests_collapsed` locally.
- Clicking a title or POI button (`quest_tracker:open:<id>`) does nothing, because the native QuestLogFrame does not exist.
- There is no native QuestFrame yet. The fixture accepts a quest through `GameClient.accept_quest_from(npc_name, quest_id)`, which sends `QuestGiverAcceptQuest` directly.
- The tracker sits under the minimap at the same Edit Mode anchor as Bevy; see [[minimap]].
- Legibility: title `OBJECTIVE_TRACKER_COLOR.Header` = `OBJECTIVE_TRACKER_BLOCK_HEADER_COLOR` (GlobalColor 241, 0xBF9C00 = 0.75, 0.61, 0) and lines `Normal` 0.8 grey, both `ObjectiveTrackerLineFont` FRIZQT 12 with a black (1, −1) shadow (`Blizzard_ObjectiveTrackerFonts.xml:202-209`). Native labels drew no FontString shadow until the projection mapped `shadow_color`/`shadow_offset` to Godot's `font_shadow_color`/`shadow_offset_*` (y flipped), so the dark-gold title vanished on bright grass. Hover highlight (`HeaderHighlight`/`NormalHighlight`) is not implemented in either client.
