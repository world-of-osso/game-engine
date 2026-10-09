# Minimap (Godot)

How the native Godot `MinimapCluster` works. Contract: [minimap spec](../../specs/minimap.md). The Bevy minimap (square, `data/minimap/` tiles, a height-map fallback) is described in [[ui-system]] and is reference only.

## Layers

- **Pure logic** lives in `godot/core/src/minimap_data.rs`, with no engine types:
  - `MinimapView { center, diameter }`, built from the engine `(x, z)` and the zoom level;
  - `tiles()`, the bounding box of tile keys the view touches;
  - `compose(view, size, tile)`, the RGBA8 circle;
  - `blip_offset`;
  - `AreaCatalog` (AreaTable name, parent, `FactionGroupMask` and sanctuary flag) and `parse_race_faction_groups` (ChrRaces `Alliance`).
- **Screen**: `godot/ui-model/src/minimap.rs` builds the rsx `MinimapCluster` in cluster-local units. Chrome rects follow Retail anchors in `Minimap.xml`; blips use map-relative offsets and the service menu uses local checkbox geometry. `apply_minimap_postsetup` sets the arrow's `TextureData.rotation` and points `MinimapDisplay` at the host's dynamic texture.
- **Host**: `godot/rust/src/minimap.rs` runs the per-frame "Minimap" step. It sends quest-giver queries, handles actions (zoom and world-map toggle), builds the state, and recomposites. `RegistryUi::show_minimap` registers a 1×1 dynamic texture. `set_minimap` updates its pixels in place, and the projection re-reads dynamic textures on every sync.

## Coordinates

Engine `x` is Retail world X (north) and engine `z` is world −Y (east); see [[world-map]] for `engine_to_world`.

A minimap tile key is `(floor(32 + z/533⅓), floor(32 − x/533⅓))`. This is the same pair `terrain_height_data::bevy_to_tile_coords` gives, and the ADT file name uses it: `<map>_FF_SS`. The first index runs west to east and is the tile image's x. The second runs north to south and is the image's y. The listfile pads both indices to two digits (`map03_07.blp`).

Composite pixel `(px, py)` of a `size`² image maps to `x = cx + (size/2 − py − ½)·d/size` and `z = cz + (px + ½ − size/2)·d/size`, where `d` is the view diameter in yards.

The arrow uses the world map's `arrow_rotation(yaw) = (yaw − π/2) mod 2π`, counter-clockwise, because the minimap is also north-up. Godot controls turn clockwise, so the projection negates the rotation. The fixture reads the drawn `Part0` rotation and checks that `(sin a, −cos a)` follows W movement on the map (right = +z, down = −x).

## Tiles and textures

For map2991, `Minimap::load_tile` reads the existing WDT7198644 MAID minimap slot; other maps resolve the listfile path through `CascListfileResolver`. [Zephras diagnosis/proof](../investigations/zephras-sky-minimap.md). It caches the file at `data/textures/<fdid>.blp`, and decodes it with `game_engine_core::blp::decode_rgba`. The result is cached per `(map, key)`, and a missing tile is remembered as `None`.

Northshire's `azeroth/map32_48.blp` is FDID 204493, a 256² tile. At zoom 0 the 198-unit map spans 466⅔ yards, about 224 source texels, so the composite is 256² and is rebuilt only after the player moves half a composite pixel, the zoom changes, or the map changes. Chrome and tracker FDIDs are copied out of CASC the first time they are drawn. FDIDs 4618654 (vertical header edges) and 4618663 (calendar) were not on disk before this port.

## Quest-giver blips

The host keeps the set of NPCs it has queried. It sends one `QuestGiverStatusQuery` for newly mirrored `QUESTGIVER` NPCs, and drops NPCs that leave replication. `Account` stores each `QuestGiverStatusMultiple` entry; the server re-sends the statuses after every quest change. `Available` and `Reward` produce blips; the other statuses produce none, and trivial quests stay hidden as with Retail's default tracking.

## Member, target and service tracking blips

`godot/ui-model/src/minimap_units.rs` projects the account's `GroupState.live` positions, not replica-interest positions, so out-of-interest online members still have direction arrows. Roster class selects the shared class palette; character ID names the member frame. The target instead requires a current replicated `Position`. Separate prefixes allow its overlay to coexist with the same member's dot. Modern clamps along a circle; Forever clamps along its usable square, below the opaque header. Registry rotation is counter-clockwise; north-pointing group art uses `-atan2(right, -down)` before the Godot projection negates it.

`minimap_tracking.rs` holds menu selections and maps supported `NpcFlags` to Retail service art. The host supplies only current replicated positions; there is no static-world reconstruction or new server feed. Tracking-button clicks open the menu, native checkboxes update local selections, and outside clicks close it. Supported filters, unsupported spell sources, art provenance and bounded proof live in the [minimap contract](../../specs/minimap.md), not this page.

## Sources

- [Minimap contract](../../specs/minimap.md) — cached FrameXML and pinned DB2 art references, behavioral tests and private live proof.
- `godot/{rust,ui-model}/src/minimap.rs`, `godot/ui-model/src/{minimap_units,minimap_tracking}.rs` — host projection, rendering and local selection.
- `shared-protocol/src/protocol/{group_messages,interaction_messages,tooltip_messages}.rs` — group positions, service flags and query-only creature type.

## Gotchas

- `QuestLogSnapshot` and `QuestLogUpdate` carry `watched_quest_ids`, and the tracker order comes from that list; see [[quest-ui]]. Before this port, `Account` kept only the entries.
- A second `shared-protocol` commit breaks the handshake between an already-built private server and a rebuilt client ("the message protocol doesn't match"). Rebuild the server whenever the sibling protocol changes.
