# Quest map objectives

Quest objectives on the native world map, minimap and objective tracker. Retail defines layout and behavior; Forever changes art only. [Implementation and source coverage](../wiki/systems/quest-map-objectives.md).

## What it must do

- [ ] Prefer locally exported Retail POI geometry where present; use authored server POIs for other quest IDs. Never invent missing polygons.
- [ ] Show blue incomplete watched-objective areas and numbered objective pins on both maps. Completion removes areas and replaces the objective pin with a `?` turn-in pin.
- [ ] Match map numbers to objective tracker watch order; clicking a tracker quest super-tracks it and highlights its area.
- [ ] Show available-giver `!` pins when authoritative giver status and position exist.
- [ ] Clip minimap areas and icons to its mask; rotate map geometry and overlays together when rotation is enabled. Show the selected off-screen objective's direction arrow at the edge.

## How it works

- [Source ownership, rendering and unsupported records](../wiki/systems/quest-map-objectives.md)

## Implementation inventory

- `scripts/export_db2_csv.py`: pinned local-CASC DB2 exports.
- `godot/ui-model/src/quest_poi.rs`: local geometry catalog, visibility and minimap pins.
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

- [ ] Targeted GREEN and extension compile proof pending.
- [ ] Private real-quest screenshots in both skins pending.
- [ ] Conditional POIs require player-condition evaluation; excluded, never replaced with guessed shapes.

## Out of scope

- Quest content repair and synthesized objective shapes.
