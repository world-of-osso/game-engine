# Quest UI

Requirements: [quest UI spec](../../specs/quest-ui.md). Server side: game-server `docs/wiki/systems/quests.md`, `npc-interaction.md`.

## Data flow

- `game_engine::quest_runtime::QuestRuntime` (lib) is the client quest model: log (`QuestEntrySnapshot`s in server order), watch list, marker per NPC (server entity bits), and the open `QuestDialog` (`Greeting` / `Detail` / `Progress` / `Reward{choice}`). Reducers are pure methods returning Retail chat lines.
- `networking_quests::QuestNetworkPlugin` (`src/game/networking/quests.rs`) registers two dispatcher handlers: log (`QuestLogSnapshot`, `QuestLogUpdate`, `QuestGiverStatusMultiple`, also refreshing the IPC `QuestLogStatusSnapshot`) and dialog (interaction + quest giver messages). Within one batch the dialog handler applies `QuestGiverQuestComplete` before `QuestGiverQuestDetails`, because a turn-in's chain offer can arrive in the same batch.
- UI and IPC write `NpcInteractionRequest` messages; `send_interaction_requests` turns them into protocol messages. `Interact(Entity)` maps the main-world entity to its server entity with `ReplicationMirrorMap`.
- `NpcFlags` is mirrored from the replication worker (`network_runtime/replication.rs`). `query_new_quest_givers` queries markers for newly mirrored quest givers (queued until connected); the server pushes changes afterwards.

## Screens

`src/scenes/quest_ui/` builds three `Screen`s on one `SharedContext`: tracker (`objective_tracker_component`), log (`quest_log_frame_component`) and giver frame (`quest_frame_component`). View models come from `view.rs`; `actions.rs` maps `onclick` actions (`quest_tracker:*`, `quest_log:*`, `quest_frame:*`) to state changes and requests. `sync_quest_giver_window` keeps the `WindowId::QuestGiver` window and the dialog together (dialog opens window; window closed by the manager → `CloseInteraction`; server close → window closes). The log is `WindowId::QuestLog`, toggled by `InputAction::ToggleQuestLog` (L).

Art: `quest_art.rs` (atlas members with FDIDs, `ButtonFrameTemplate` metal NineSlice drawn as eight textures, rock background, close button, panel buttons). Wrapped text: the toolkit has no `justify_v` attribute and centres a fontstring's first line vertically, letting wrapped lines flow below unclipped, so text frames are one line tall and layout advances by `wrapped_text_height` (greedy wrap over the original spacing with `measure_text`, line pitch from the font metrics).

Markers: `nameplate.rs` spawns the talktome M2 with its animation; `indicator_facing` yaws the glyph's +X face toward the camera (the `?` model is flat and reads as `!` edge-on).

## Gotchas

- InWorld automation clicks must set the cursor at UI position × `ui_scale` (fixed in `automation_inworld.rs`); `--screen inworld` with `--run-js-ui-script` keeps the auto-login.
- `game-server-admin set-position` only moves an online character server-side; the client keeps its own position, so move characters while logged out.
- The quest log's description and rewards come from a session cache of detail pages (`QuestLogData.details`): the log snapshot has neither.
