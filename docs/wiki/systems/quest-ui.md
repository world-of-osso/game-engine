# Quest UI

> Root `src/` paths below describe the retired Bevy client; the Godot section describes the native host.

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

`godot/rust/src/quests.rs` reconciles the native quest giver frame, quest log and markers through the shared `quest_runtime`, `quest_view` and `quest_actions` models. Right-click sends normal interaction requests; greeting, detail, progress and reward pages follow server replies. L opens the log; abandon uses the confirmation popup. Tracker titles and POI buttons open the selected quest in the log.

- `godot/rust/src/objective_tracker.rs` feeds the shared `objective_tracker_component` through `ObjectiveTrackerState::from_watched`, using `Account.quest_log` entries in `Account.quest_watched` order.
- The two minimize buttons toggle `collapsed` and `quests_collapsed` locally.
- `godot/tests/world_quest_flow.gd` drives real NPC picks, clicks, combat, exploration, item use and inventory assertions on a private server. Admin supplies travel, revival and the eight gnoll armbands, never objective completion.
- Tracker placement and polygons: [[minimap]].
- Legibility: title `OBJECTIVE_TRACKER_COLOR.Header` = `OBJECTIVE_TRACKER_BLOCK_HEADER_COLOR` (GlobalColor 241, 0xBF9C00 = 0.75, 0.61, 0) and lines `Normal` 0.8 grey, both `ObjectiveTrackerLineFont` FRIZQT 12 with a black (1, −1) shadow (`Blizzard_ObjectiveTrackerFonts.xml:202-209`). Native labels drew no FontString shadow until the projection mapped `shadow_color`/`shadow_offset` to Godot's `font_shadow_color`/`shadow_offset_*` (y flipped), so the dark-gold title vanished on bright grass. Hover highlight (`HeaderHighlight`/`NormalHighlight`) is not implemented in either client.


## Native quest scrolling

At `af35f6e1`, quest-log text flowed in an auto-height `QuestLogDetailsContent` directly under the window, without a scroll-list ancestor. Native projection clips registered scroll lists, not arbitrary content frames, so growing descriptions crossed the parchment and action buttons. QuestFrame already had scroll frames, but its fixed paragraph heights and subsequent absolute offsets used estimates; the injected 1000-pixel native-height regression reproduced objectives landing inside the paragraph.

`quest_scroll.rs` builds a clipped viewport, auto-height child and skin-resolved MinimalScrollBar. After native shaping/layout, `RegistryModel` feeds the measured child height back into `QuestScrollExtent` and rebuilds the range/bar before drawing. Log and detail/progress/reward content use vertical flow; narrow log rewards use one column. Wheel/steppers pan 30 pixels, thumb drag reaches the full range; action buttons remain outside the viewport. Selecting another log quest resets its offset.

Retail source under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UIPanels_Game/Mainline/`: `QuestMapFrame.xml:765-783` puts details in `QuestMapDetailsScrollFrame` with a Contents scroll child; `QuestMapFrame.lua:999-1004` displays QuestInfo there and resets the scrollbar. Retail map rewards have their own clipped container (`QuestMapFrame.xml:721`), whereas the popup log puts `QUEST_TEMPLATE_LOG` in its scroll child (`QuestMapFrame.lua:2383-2385`). `QuestInfo.lua:103-117` reparents sections and anchors each below its predecessor. Dialog scroll frames use `QuestFrameTemplates.xml:158-167`, with detail/reward `QuestInfo_Display` at `QuestFrame.lua:556-558,127-128`.

Proof: `godot/rust/src/ui/quest_scroll_tests.rs`, `godot/tests/quest_overflow_capture.gd`, and the existing `forever_quest_windows::capture_base_trees` recorder. Capture fixture compares content-visible/hidden pixels outside the viewport and drives native wheel/thumb input for long/short log, detail, progress and reward pages in both skins. Evidence directory: `data/diagnostics/questoverflow-2026-10-06/`; consult its proof ledger for current acceptance, including paragraph-rendering checks and renderer teardown warnings. Options/menu tree suffix is byte-identical in the recapture.

## Live fixture diagnostics

Evidence: `data/diagnostics/questrun-2026-10-05/` in the canonical game-engine data tree. Native extension `9d8a1040`, private server binary `c8cd38f`; both presets completed the gameplay flow, including real mine exploration and both fixed rewards. Long quest text still overflows the parchment; visual acceptance is separate from gameplay assertions.

### Jasperlode

The original mouth failure already has ground fixes in `310a1e0a` and the ramp route in `19f9d1d9`: terrain holes must not interpolate a surface over the WMO floor; slow movement frames need collision substeps. The old start stood under the entrance rock below the floor. Four real-asset collision tests cover the ramp/tunnel at 60 and 2 FPS, the terrain hole, and no falling through the hillside.

The rerun crossed the mouth, but the warrior died farther inside. `modern4-client.log` records health zero, authoritative position `(-9140.282,57.63335,595.6155)` and predicted position `(-9082.92,59.81815,552.94)`: local prediction was not a living server walk. Fixture `092aef00` rejects dead walks and revives after travel; `8901eafe` resumes at the authoritative death position only outside the trigger plus a four-yard margin. It retains subsequent walking into the trigger, rather than reviving inside it or awarding credit administratively. Modern completed with one outside-trigger recovery; Forever needed two.

### Milly

Historical missing-mirror failure did not reproduce. Milly remained server entity `4294952020`, was re-created after the vineyard return, and both presets received one each of items `57247` and `11475`. Three additional vineyard/Milly interest exits and returns passed per preset. In Modern, her visual request preceded the player's revival. A dead-player probe also mirrored her normally but could not open gossip. Death is therefore an interaction blocker, not established evidence for the old mirroring failure. The referenced October 1 diagnostic files were absent on this host; historical cause remains unresolved. No server repair is claimed.

### Slow-frame input and capture

Six-frame approach bursts skip melee range at 2 FPS. `994dfbc3` polls each frame and stops movement before turning. Key turning at 2.5 rad/s can oscillate beyond its tolerance; `1a3de850` calibrates actual right-drag camera input, and `b375ab6e` handles a GUI-consumed first press with bounded retries and released buttons.

The error overlay holds a line for three seconds, then fades for half a second. Predicate settling plus another draw can lose the required message: actual quest capture instrumentation saw opacity change from `1.0` to `0.334`. `9dc775f7` saves the first post-draw frame while the line is opaque. The real button/timer/rendered-image regression at 2 FPS reproduced the old empty capture and preserved 721 red text pixels with the corrected pipeline. `is_visible_in_tree()` alone is not rendered proof. The fixture uses a 1920×1080 viewport and rejects a clipped tracker.

## Sources

- [Quest UI spec](../../specs/quest-ui.md) — requirements and test inventory.
- [Native quest host](../../../godot/rust/src/quests.rs) — dialogue, log and markers.
- [Quest scroll viewport](../../../godot/ui-model/src/ui/screens/quest_scroll.rs), [native extent feedback](../../../godot/rust/src/ui/mod.rs) and [offline clipping/input capture](../../../godot/tests/quest_overflow_capture.gd) — bounded native scrolling behavior.
- [Live fixture](../../../godot/tests/world_quest_flow.gd), [movement regression](../../../godot/tests/quest_flow_movement.gd), [capture regression](../../../godot/tests/quest_flow_error_capture.gd) — observable input, life and rendered output.
- [Native ground](../../../godot/rust/src/ground.rs), [collision regressions](../../../godot/rust/src/wmo/collision_tests.rs) — Jasperlode geometry and slow-frame movement.
- [Error lifetime](../../../godot/ui-model/src/ui/ui_errors_data.rs) — hold and fade durations; proof commands and revisions are in the evidence directory's `proof-ledger.txt`.

## See Also

- [[minimap]] — objective polygons and tracker placement.
- [[ui-system]] — native frame projection and font rendering.
