# game-engine - Plan

## Current Blocker

- None.

## Active TODO

### UI parity and tooling

- [ ] Match `wow-ui-sim` object tree output with `game-engine --dump-ui-tree` for shared screens.
- [ ] Decide the compatibility boundary: adapt `game-engine` dump formatting only, or move shared tree/object concepts into `ui_toolkit`.
- [ ] Expand the UI dump to cover the metadata we still miss for parity (regions, layers, draw order, inherited templates, script hooks, and other widget-specific fields as needed).
- [ ] Build a repeatable comparison flow for login / char select / char create trees so regressions are obvious.

### UI runtime

- [ ] Keep filling out WoW-style UI behavior in the screen/frame system where the current toolkit is still thin.
- [ ] Verify the live CharSelect delete button/icon path against a freshly restarted client process. Current tests prove `DeleteChar` is built with empty text and a nested `DeleteCharIcon`, so the remaining gap is runtime verification of the running process rather than screen construction.
- [ ] Replace the `src/ui/wasm_host.rs` stub with a real host, or remove/defer it until addon execution is a real milestone.

### Data pipeline

- [ ] Replace ad-hoc runtime parsing of WoW CSV/DB2 tables with generated game-engine-specific data blobs for customization, outfit, and equipment lookup paths.
- [ ] Define small engine-owned structs for derived runtime lookups instead of carrying raw DB2/CSV-shaped records through the renderer.
- [ ] Add an offline conversion step that imports source tables once and writes versioned generated assets under `data/`.
- [ ] Keep RON for hand-authored config and debugging dumps only; do not use it as the main runtime format for large derived lookup tables.
- [ ] Avoid SQLite for the main client lookup path unless we later need ad-hoc querying or external editing; the current runtime mostly needs fast typed loads, not relational queries.

### Options menu (Blizzard-inspired)

#### Phase 0: Foundation

- [ ] Reuse the existing `GameMenu` overlay as the entry point for the options modal instead of introducing a separate screen/plugin flow.
- [ ] Define a dedicated overlay/options state resource covering current view, selected category, modal position, pressed frame, drag capture, and draft-vs-committed values.
- [ ] Identify all current engine-backed settings/resources that the first options pass can control directly: sound, camera, HUD visibility, and debug/FPS toggles.
- [ ] Decide the persistence file path and format for client options (`data/ui/options_settings.ron`).

#### Phase 1: Pointer/Input Model

- [ ] Replace the current click-only `game_menu_screen.rs` input path with a press/move/release pointer state machine.
- [ ] Add click-vs-drag threshold handling so drags do not accidentally fire button actions.
- [ ] Support explicit pointer capture modes: none, window drag, and slider drag.
- [ ] Fire button actions on release only when the interaction remained a click.

#### Phase 2: Drag Handling

- [ ] Make the options modal draggable from the header/title area only.
- [ ] Clamp modal movement to the visible window bounds.
- [ ] Reuse the action-bar drag pattern where practical for grab offset tracking and release cleanup.
- [ ] Implement real slider drag behavior: thumb drag, track click-to-position, step quantization, and live value updates while dragging.

#### Phase 3: Blizzard-Style Shell

- [ ] Expand `Options` from a single button action into a full modal layered inside the existing game menu flow.
- [ ] Build a Blizzard-inspired larger panel with category list, content pane, section headers, and footer buttons (`Defaults`, `Apply`, `Cancel`, `Okay`, `Back`).
- [ ] Keep the main game menu intact and support returning from options back to the menu without tearing down the overlay.
- [ ] Mirror Blizzard structure broadly even when some categories are placeholders.

#### Phase 4: Functional Categories

- [ ] Implement live `Sound` settings: mute, music enabled, master/music/ambient/footstep volume.
- [ ] Implement live `Camera` settings: look sensitivity, invert Y, zoom speed, follow speed, min/max distance.
- [ ] Implement live `HUD` / `Interface` settings backed by explicit visibility/toggle resources for minimap, action bars, target frame, nameplates, health bars, and FPS overlay.
- [ ] Add broad but initially partial categories for graphics, controls, accessibility, keybindings, macros, addons/social, support/about, and advanced/debug.
- [ ] Replace hardcoded in-world gameplay keys with a persisted `InputBindings` model loaded through client options.
- [ ] Build an interactive Keybindings editor with section tabs, rebind/cancel/clear flow, and conflict swap behavior.
- [ ] Add real `Auto-Run` input handling and expose it in the Keybindings category.
- [ ] Render non-functional rows as disabled or clearly labeled placeholders instead of silently omitting them.

#### Phase 5: Persistence And Apply/Revert

- [ ] Load committed options at startup and initialize runtime resources from them.
- [ ] Keep draft values while the options modal is open, separate from committed values.
- [ ] `Apply` commits draft values without closing; `Okay` commits and closes; `Cancel` reverts to the state from modal open.
- [ ] Save committed options and modal position to disk on commit, not every frame.

#### Phase 6: Tests And Runtime Verification

- [ ] Add screen/component tests for options modal layout, category switching, and live value labels.
- [ ] Add input tests for header drag, slider drag, click-vs-drag threshold, release behavior, and escape/back-navigation.
- [ ] Add subsystem tests for camera option wiring, sound option wiring, HUD visibility toggles, and settings persistence round-trip.
- [ ] Add serialization and runtime tests for input bindings, autorun, and keybinding editor capture flow.
- [ ] Run targeted runtime verification against `--screen inworld` / `Escape -> Options` after the implementation lands.

### Rendering / world

- [ ] Fix in-world micro-freezes during terrain streaming.
- [ ] Budget streamed tile application per frame instead of draining every completed tile in one `receive_loaded_tiles` update.
- [ ] Move more tile-finalization work off the main thread, especially first-use texture decode/extract and heavy doodad/WMO spawn paths.
- [ ] Profile and reduce minimap recomposite cost while moving; current composite/crop path rewrites image buffers whenever the player's minimap pixel changes.
- [x] Particle effects from M2 data (bevy_hanabi GPU particles, textured, color/size gradients).
- [ ] Particle Phase 2: parse drag from M2 emitter, add `LinearDragModifier`. Use area_length/area_width in position modifiers.
- [ ] Particle Phase 3: flipbook animation — `FlipbookModifier` driven by age/lifetime to cycle texture atlas frames.
- [ ] Particle Phase 4: velocity-oriented particles — `OrientModifier(AlongVelocity)` when tail flag set.
- [ ] Refactor app startup: each screen should own its full App configuration instead of main.rs building a monolithic app with all plugins always registered. Debug scenes (particledebug, debugcharacter) should not load game networking, login UI, etc.
- [ ] Cross-tile ADT stitching: raw MCNK heights differ by up to 50 units at tile borders (same as intra-tile). Need to stitch border chunks when adjacent tiles are loaded. See `adt_seam_tests.rs` for diagnostics.
- [ ] Find the root cause of the white/grey terrain bands in the in-world ADT renderer.
- [ ] Investigate bare `Camera` entity warnings during CharSelect -> InWorld transition.
- [ ] Add object colliders for WMOs and large doodads.
- [ ] Add server-side terrain slope validation using the shared walkable-slope rules.
- [ ] Implement `HelmetAnimScaling` / `ChrRaces.HelmetAnimScalingRaceID` so runtime helms scale correctly for races that opt into that DB2 path.
- [ ] Finish helm-driven scalp hair hiding: `HelmetGeosetVis` / `HelmetGeosetData` and `CharHairGeosets.Showscalp` are wired, but the client still lacks the actual trigger that requests hidden hair group `0` for visible helms like display `1128`.

### Investigation notes

- Micro-freeze investigation on 2026-03-16 points to `src/terrain.rs` tile streaming as the primary hitch source.
- `receive_loaded_tiles` currently collects all completed background jobs with `try_iter()` and applies them in one frame, so multiple finished tiles can burst onto the main thread together.
- A completed tile still performs expensive main-thread work: terrain texture decode/material setup, doodad M2 spawn, WMO root/group reads, and first-use asset extraction through `src/asset/casc_resolver.rs`.
- `src/minimap.rs` also adds steady CPU pressure by recompositing/cropping the minimap image whenever the player's minimap pixel changes, which can amplify streaming hitches.
- CharSelect delete-button investigation on 2026-03-16: added a `ui-toolkit` regression test proving textures nested inside buttons create child frames and render quads, and added `game-engine` tests proving `DeleteChar` syncs with empty text plus `DeleteCharIcon` as a child texture when a character is selected.
- Generated delete icon assets live under `output/imagegen/delete-trash-icon.{png,ktx2}`. The remaining discrepancy is between the tested screen definition and the user's live running process, not the screen builder or toolkit child-frame diff path.

### Networking / game state

- [ ] Replace remaining placeholder status snapshots with authoritative replicated data.
- [ ] Handle `InWorld` disconnects without closing the client or bouncing through `Login`.
- [ ] Add a `Reconnecting` state or in-world overlay that keeps the current scene visible while reconnect is in progress.
- [ ] Detect loss of `Connected` during `InWorld`, freeze local input, and show `Reconnecting...`.
- [ ] Add an explicit client-side `reset_network_world()` path instead of relying on scattered disconnect cleanup hooks.
- [ ] On reconnect, clear only network-derived state:
  replicated entities, local-player/network tags, and transient replicated snapshots/resources that must be rebuilt.
- [ ] Reconnect using the saved auth token and resume the normal auth/select-enter-world flow without showing the login screen.
- [ ] Rebind the local player cleanly after reconnect so stale name/entity matches do not leak across sessions.
- [ ] Hide the reconnecting overlay only after replication and initial world/terrain data have been received again.
- [ ] Audit reconnect-sensitive client state for explicit reset/resubscribe handling:
  targeting, terrain/world streaming state, chat/status resources, UI snapshots, and any other network-backed resources.

## Parked

- [ ] NPC / mob AI and pathfinding.
- [ ] Full addon compatibility beyond the targeted UI/runtime work above.
