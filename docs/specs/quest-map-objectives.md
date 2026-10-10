# Quest map objectives

Quest objectives on the native world map, minimap and objective tracker. Retail defines layout and behavior; Forever changes art only. [Implementation and source coverage](../wiki/systems/quest-map-objectives.md).

## What it must do

- [x] Prefer locally exported Retail POI geometry where present; use authored server POIs for other quest IDs. Never invent missing polygons.
- [x] Show blue incomplete objective areas and numbered objective pins on both maps. World-map blobs show only the super-tracked quest (plus explicit hover/focus quests when supported), not every watched quest. Completion removes areas and replaces the objective pin with a `?` turn-in pin.
- [x] Match map numbers to objective tracker watch order; clicking a tracker quest super-tracks it. Normal and super-tracked blobs use fill128/border192, never stronger selected opacity.
- [x] Show available-giver `!` pins when authoritative giver status and position exist.
- [x] Clip minimap areas and icons to its mask; rotate map geometry and overlays together when rotation is enabled. Show the selected off-screen objective's direction arrow at the edge.

## How it works

- [Source ownership, rendering and unsupported records](../wiki/systems/quest-map-objectives.md)

## Implementation inventory

- `scripts/export_db2_csv.py`: pinned local-CASC DB2 exports.
- `godot/ui-model/src/{quest_poi,world_map_view_data}.rs`: local geometry, visibility, authoritative giver-offer projection and map pins.
- `godot/ui-model/src/game/quest_{runtime,actions}.rs`: watch numbering and super-tracking.
- `godot/core/src/{quest_area_data,minimap_data}.rs`: authored-art rasterization and shared map transform.
- `godot/rust/src/{quests,world_map,minimap}.rs`: live snapshot hydration and native map hosts.

## Tests asserting this spec

- `godot/ui-model/src/quest_poi.rs` tests: real quest28766 geometry and completion/edge direction.
- `godot/ui-model/tests/quest_flow.rs`: tracker selection and both-skin numbered POI buttons.
- `godot/tests/quest_map_objectives.gd`: private quest28766 acceptance, selected quest, live completion and rendered both-skin captures.
- `godot/ui-model/tests/minimapblips.rs`: visible numbered objective text on both minimap skins.
- `godot/ui-model/src/world_map_view_data_tests.rs`: projection and completed-objective removal.
- `godot/core/{src/quest_area_data.rs,tests/minimap_data.rs}`: blue fill, clipping and rotation/inverse projection.

## Known gaps (current cycle)

- [ ] Quest-log/tracker hover/focus blob selection and hover visuals: no world-map highlight texture/atlas or native highlight alpha found in cached Retail UI. No invented highlight applied.
- [ ] Retail minimap native blob selection is not exposed in cached Lua. QuestPOIs is an always-on tracking filter, not proof of which polygons native code draws; retain watched polygons pending native evidence. The authored minimap OutsideSelected border remains, at unchanged128/192 opacity.

- [ ] Conditional POIs require player-condition evaluation; excluded, never replaced with guessed shapes. Missing authored geometry remains absent; [source coverage](../wiki/systems/quest-map-objectives.md#source-ownership).
- [ ] Independent verification unavailable (Claude OAuth expired). Workspace format check has two unchanged baseline failures; changed-file format check passes.

Current proof: 41 distinct targeted tests; native extension/CLI build and cargo check pass. Real quest28766 accepted on private UDP5518; authoritative completion updates both maps live. Twelve final 1920×1080 PNGs (both skins, incomplete/complete, world map/minimap/rotated minimap) were FFmpeg-decoded and visually inspected before publication to `/syncthing/AgentShared/2026-10-10/quest-poi/`. Off-screen edge arrows have pure-model proof, not live screenshot proof.

## Out of scope

- Quest content repair and synthesized objective shapes.
