# Minimap (Godot)

How the native Godot `MinimapCluster` works. Contract: [minimap spec](../../specs/minimap.md). The Bevy minimap (square, `data/minimap/` tiles, a height-map fallback) is described in [[ui-system]] and is reference only.

## Layers

- **Pure logic** lives in `godot/core/src/minimap_data.rs`, with no engine types:
  - `MinimapView { center, diameter, mask, rotation }`, built from the engine `(x, z)` and the zoom level;
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

By default the minimap is north-up: the arrow uses the world map's `arrow_rotation(yaw) = (yaw − π/2) mod 2π`, counter-clockwise. Optional rotation transforms map pixels, blobs and pins together; [quest map objectives](quest-map-objectives.md) owns that behavior and proof. Godot controls turn clockwise, so the projection negates the rotation. The fixture reads the drawn `Part0` rotation and checks that `(sin a, −cos a)` follows W movement on the map (right = +z, down = −x).

## Tiles and textures

For map2991, `Minimap::load_tile` reads the existing WDT7198644 MAID minimap slot; other maps resolve the listfile path through `CascListfileResolver`. [Zephras diagnosis/proof](../investigations/zephras-sky-minimap.md). It caches the file at `data/textures/<fdid>.blp`, and decodes it with `game_engine_core::blp::decode_rgba`. The result is cached per `(map, key)`, and a missing tile is remembered as `None`.

Northshire's `azeroth/map32_48.blp` is FDID 204493, a 256² tile. At zoom 0 the 198-unit map spans 466⅔ yards, about 224 source texels, so the composite is 256² and is rebuilt only after the player moves half a composite pixel, the zoom changes, or the map changes. Chrome and tracker art now read shipped extracted files; first-draw CASC extraction describes the historical port, not current UI policy (`86e5bb1ed`, `b4094d2b2`). See [shipped assets](shipped-assets.md) for development versus extracted-only world-asset behavior.

## Indoor status (2026-10-09)

Indoor WMO rendering is not implemented. `adc7d3a66` exports `WMOMinimapTexture`; `b79b82166` records the unresolved block-size/origin geometry. Local Stockade/cave tiles and DB2 rows do not prove world-position-to-texel mapping or zoom switching. The [indoor contract and blocker](../../specs/minimap.md#indoor-wmo-map-unimplemented) owns requirements and exact evidence.

## Quest-giver blips

The host keeps the set of NPCs it has queried. It sends one `QuestGiverStatusQuery` for newly mirrored `QUESTGIVER` NPCs, and drops NPCs that leave replication. `Account` stores each `QuestGiverStatusMultiple` entry; the server re-sends the statuses after every quest change. `Available` and `Reward` produce blips; the other statuses produce none, and trivial quests stay hidden as with Retail's default tracking.

## Quest objectives

[Quest map objectives](quest-map-objectives.md) owns client-local/server POI geometry, blue fill and selected rim art, numbered icons, super-tracking and completion updates. Its proof ledger includes both-skin private real-quest minimap captures.

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


## Zoom button source states — 2026-10-15

Retail `Minimap.xml:190-219` defines actual Buttons, normal/down/mouseover atlas members and desaturated normal art while disabled. `Minimap.lua:254-257,309-345` disables the corresponding control at zoom0/5 and enables the opposite direction. The old `zoom_button` emitted a clickable Texture with disabled alpha0.5: no held/hover state, and activation on press rather than cancellable release.

`minimap.rs` now emits source Buttons at unchanged rectangles. The scoped `parts.rs` projection preserves base art beneath a full ADD mouseover overlay and applies the existing sampled-image grayscale shader only while disabled. No toolkit/schema/server, map diameter, wheel/key binding, hover visibility, tooltip or sound changes.

Proof: `minimap_zoom_source_states_keep_full_opacity_and_disabled_grayscale` builds both real screens at zoom0/2/5 (RED static Texture → GREEN). `godot/tests/minimap_zoom_fidelity.gd` drives real native motion/press/outside release/inside release through each control: both skins pass, disabled controls emit nothing and do not change hover/press pixels. Offline RegistryUi requires `pop_action()` after each event to synchronize queued input before observing pixels; an earlier capture omitted that synchronization and is excluded from hover acceptance.

Both-skin before/after sheets and source-position context details were inspected at `AgentShared/2026-10-15/ui-questlog-minimap-zoom/`. At zoom2 the entire context is RGB-identical before/after; at limits all RGB changes remain within the two source zoom rectangles (six fullHD comparisons, zero outside changes). Native material inspection confirms disabled grayscale and full frame alpha. Baseline action-on-press/outside-release RED is retained, not accepted as a passing run.

Exact revisions, source/atlas/binary/PNG hashes and exclusions: `data/diagnostics/ui-questlog-minimap-zoom-2026-10-15/zoom-proof-ledger.json`. Supplied offline states prove production screen/input/projection only: live host zoom transitions, tile recomposition, tooltips, sound and whole-minimap fidelity remain outside this proof. Contract: [minimap](../../specs/minimap.md).
