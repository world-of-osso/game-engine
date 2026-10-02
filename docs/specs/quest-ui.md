# Quest UI

Retail objective tracker, quest log and quest giver frame on the live server quest runtime (quest phase of the [in-game UI plan](../plans/2026-09-23-ingame-ui.md)). Contract: shared-protocol `protocol/quest_messages.rs`, `protocol/interaction_messages.rs`, `QuestLogSnapshot`; server semantics in game-server `docs/specs/quests.md`. How it works: [quest UI](../wiki/systems/quest-ui.md).

## What it must do

- [x] Right-clicking an NPC in range sends `InteractNpc`; `InteractionOpened` opens the quest giver frame (gossip text and options, plus the NPC's quests from `QuestGiverHello` when it has `NPCFlags::QUESTGIVER`); `InteractionFailed` shows its Retail text in `UIErrorsFrame`; `InteractionClosed` closes the frame.
- [x] Quest giver frame `QuestFrame` (338×496 `ButtonFrameTemplate` chrome, `QuestBG-Parchment`): greeting (Current Quests above Available Quests, gossip options), detail (Accept/Decline), progress (`QuestGiverRequestItems`, Continue disabled until `can_complete`, Cancel), reward (Complete Quest always enabled; with several choices and none chosen it shows "You must choose a reward.", a lone choice is taken; QuestFrame.lua:145). NPC-driven Panel in slot L; Escape (`CloseAllWindows`, with the quest log) or eviction sends `CloseInteraction`. Godot: QuestFrame and the quest log are `toplevel` (QuestFrame.xml:36) and raise on press. Hovering a reward item (`QuestInfoItem{n}`: choices, then fixed items) in the frame or the log shows its item tooltip through the shared GameTooltip at `ANCHOR_RIGHT` (QuestInfo.lua:1240-1258).
- [x] Quest text tokens substituted client-side: `$N`/`$n`, `$C`/`$c`, `$R`/`$r`, `$B`, `$Gmale:female;`.
- [x] Turn-in: `QuestGiverQuestComplete` posts `<title> completed.`, `Experience gained: N.`, `Received <money>.` and item lines to chat; a chain offer (`QuestGiverQuestDetails`) in the same batch stays open. Accepts post `Quest accepted: <title>`.
- [x] Objective tracker `ObjectiveTrackerFrame` at the Retail default anchor (TOPRIGHT −110, −275): "All Objectives" and "Quests" headers with collapse buttons, one block per watched quest in watch order with POI button, title, `current/required text` lines (dash; finished lines checked and grey), finished quests show only their completion log text or "Ready for turn-in"; hidden with nothing watched; clicking a title or POI opens the quest log on that quest. Edit-mode movable by name.
- [x] Quest log `QuestLogFrame` (L, Panel): `Quests: n/35`, groups by `QuestSortID` (area name) with collapsible headers, selected quest details (title, objectives text, objective lines, description and rewards when shown by a giver this session), Abandon behind the Retail `ABANDON_QUEST` popup, Track/Untrack via `SetQuestWatched`.
- [x] Markers: every mirrored NPC with `QUESTGIVER` is queried with `QuestGiverStatusQuery`; `interface/buttons/talktome*.m2` floats above it (yellow `!` Available, yellow `?` Reward, grey `!` Unavailable, grey `?` Incomplete; trivial `LowLevelAvailable` hidden as with Retail's default tracking), animated, facing the camera.
- [ ] Quest log description and rewards for quests accepted in an earlier session: `QuestEntrySnapshot` carries neither (protocol gap).
- [ ] Negative `QuestSortID` headers (class/profession sorts) show "Unknown": no `QuestSort` table in `data/`.
- [x] Objective areas: watched quests' unfinished objectives' `QuestPOI` polygons drawn gold (brighter rim) on the minimap and the world map canvas; the world map pins each objective (`QuestObjective`).
- [ ] NPC portrait in the frame's portrait ring; scroll frames for texts taller than the parchment; super-tracking.

## Tests asserting this spec

- `src/game/quest_runtime_tests.rs` — log deltas and accept lines, quest list per NPC, reward choice bounds, turn-in lines, token substitution.
- `src/game/networking/quests_tests.rs` — message handlers through inboxes: gossip → Hello request, turn-in + chain offer in one batch, errors, IPC status, markers.
- `src/scenes/quest_ui/{actions,view}_tests.rs`, `tests.rs` — click actions → requests, abandon popup, track, window reconcile, view models.
- `src/ui/screens/{objective_tracker,quest_frame,quest_log_frame}_component_tests.rs` — rendered frames, text, anchors, actions.
- `tests/unit/target_tests/world_camera.rs` — right-click ray → `NpcInteractionRequest::Interact`.
- Live evidence: `data/diagnostics/quest-ui-20260924/` (headless client, shared dev server).
- `godot/ui-model/tests/quest_flow.rs` — the Godot host's models: greeting → detail → accept, progress, reward choice error and lone choice, log track/abandon, markers, objective areas.
- Godot live: `godot/tests/world_quest_flow.gd` — Northshire 28766 → 28774 (markers, accept, kills, XP/money turn-in, follow-up, abandon), 62 → 76 exploration by walking into the mines' area triggers, 239 → 11 reward choice (armbands by admin `grant-item`), 26391 fixed items (fires put out with Milly's Fire Extinguisher) on a private server; no objective is completed by admin. Full pass 2026-10-01 before area-trigger and SmartAI support (`data/diagnostics/quests-godot-2026-10-01/`). 2026-10-01 rerun (`data/diagnostics/questsrv3-2026-10-01/`): six real worg kills credited, turn-in, follow-up and abandon pass; 62 explored by walking; 26391 reaches 8/8 by spraying. Open: 76 stops at the Jasperlode Mine's south mouth (the client's WMO wall check refuses the step in, `stuck-76-mine-mouth.png`); after the fires Milly Osworth is not found among the mirrored units, so the 26391 turn-in is unproven.
- Godot: `godot/tests/world_minimap_quest.gd` — native objective tracker block, objective line, anchor and collapse on a private server (`data/diagnostics/minimapquest-2026-09-29/`); see [minimap](minimap.md).
