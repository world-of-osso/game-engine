# World map

The in-world World Map: a Retail-style windowed `WorldMapFrame` toggled by the
`ToggleWorldMap` binding (default `M`) that shows the player's current map from the
Retail `UiMap` DB2 data, zooms out zone → continent → world, and marks the player
and the data-backed points of interest. The view model is shared by the Godot and
Bevy hosts. How it works: [world-map system](../wiki/systems/world-map.md).

## What it must do

### Map data
- [x] The map opened for a position is the smallest world-system `UiMap` of type Zone whose `UiMapAssignment` on the player's map contains the position, else its Continent (Goldshire → Elwynn Forest, Northshire Abbey → Northshire, open sea → Eastern Kingdoms).
- [x] Map positions match Retail coordinates on zone, continent and world maps (Goldshire flight master at 41.8, 64.6 on Elwynn Forest).
- [x] Zooming out follows the `UiMap` parent chain through maps that have art (Elwynn Forest → Eastern Kingdoms → Azeroth).
- [x] The child map under a canvas point, and its rect, come from the parent's assignments (Elwynn on Eastern Kingdoms; Eastern Kingdoms on Azeroth, clipped to where Azeroth draws map 0).
- [x] Map art is the phase-0 `UiMapArt` layer-0 tile grid, with tiles past the layer edge trimmed (Elwynn 12 tiles over 1002×668, continents 150).

### Frame
- [x] The frame is a metal portrait window titled "World Map" with a breadcrumb bar from the root map to the displayed map and a close button; it fits the viewport keeping the 1002×668 canvas aspect.
- [x] Breadcrumbs navigate to their map; the close button closes the frame.
- [x] Pressing the toggle binding in world opens the frame on the player's map (Godot: Stormwind City for a character standing in Stormwind); pressing it again, or Escape, closes it. Escape closing the map does not open the game menu.
- [x] Godot: dragging the title's top 24 logical units, excluding the close button, moves and clamps `WorldMapFrame`; release saves its top-left for the selected server character in canonical `ui_layout.ron`. Reopen and a fresh process restore that position; Reset Window Positions restores the Wide slot for that character only.
- [x] Right-click (or wheel down) on the canvas zooms out one level; left-click (or wheel up) zooms into the child map under the cursor; hovering a child map highlights it with its `UiMapArt` highlight texture (additive) and names it.
- [x] Keyboard movement continues while the frame is open; mouse input over the frame does not drive the camera.
- [x] Art tiles missing from the local CASC install are left undrawn and reported (warning per FDID); a missing texture never ends the session.

### Markers
- [x] The player arrow sits at the player's map position on every level whose assignments contain the player, and points where forward movement goes.
- [x] Flight masters (`TaxiNodes` with the Alliance/Horde map flags) of the player's faction (`ChrRaces.Alliance` of the selected character's race) show on zone maps with the `TaxiNode_Alliance/Horde/Neutral` icon.
- [x] Quest-log entries with POIs on the displayed zone or continent show one pin each: the numbered objective area (`UI-QuestPoi-QuestNumber`) while incomplete, the turn-in (`UI-QuestPoi-QuestBangTurnIn`) once complete. The world map shows no pins.
- [ ] Quest pins appear in the Godot client from the live server quest log (subscription wired; no runtime proof with a character holding POI quests).

## How it works
- [world-map system](../wiki/systems/world-map.md)

## Implementation inventory
- `src/ui_map_data.rs` — Bevy-free `UiMap` catalog: hierarchy, assignments, art tiles, best map, map positions, child lookup.
- `src/world_map_view_data.rs` — shared view model: navigation, breadcrumbs, tiles, highlight, player arrow, flight and quest pins; `TaxiNodes`/`ChrRaces` readers.
- `src/ui/screens/world_map_frame_component.rs` — shared `rsx!` frame, layout, postsetup (arrow rotation, additive highlight), texture list.
- `src/ui/screens/world_map_frame_art.rs` — Retail atlas crops and textures for chrome and pins.
- `godot/rust/src/world_map.rs` — Godot host: binding, pointer navigation, CASC texture caching, `world_map_state()` fixture query.
- `godot/rust/src/ui/{parts,projection}.rs` — texture rotation and additive blend projection.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs` — quest log subscription and `QuestLogUpdate` merge.
- `src/scenes/world_map_frame/mod.rs` — Bevy host over the same view model.

## Tests asserting this spec
- `src/ui_map_data_tests.rs`
- `src/world_map_view_data_tests.rs`
- `src/ui/screens/world_map_frame_component_tests.rs`
- `godot/tests/world_map_flow.gd` (dev server `127.0.0.1:5000`, run under the shared dev-server lock)
- `godot/rust/src/world_map.rs` pointer and open-reset state tests; `/tmp/claude/world-map-owned-fixture-ac44cc2e.log` proves scaled drag/release, character-scoped save/reset retention, reopen at the Wide slot, and a second-process read without a live server.
- `godot/rust/src/ui/parts_tests.rs` (rotation and additive parts), `godot/rust/src/account.rs` (quest log merge)
- `src/scenes/world_map_frame/automation_tests.rs` (Bevy host)

## Known gaps (current cycle)
- [x] Every art tile resolves from the enUS root record. Map tiles are localized: each FDID has one root record per locale, and only enUS content is in the install. The resolution cache used to keep the last locale's record, so 133,495 FDIDs (Northshire, all of Elwynn Forest and Westfall among them) resolved to absent content keys and drew black. Fix: asset-resolver `a626003`/`bbafd03` (branch `uigaps`). Proof: the `keybinds` fixture reports `missing=0` for Northshire, Elwynn Forest and Westfall (12 tiles each).
- [ ] Bevy host: no hover highlight, quest pins or faction (it passes none).
- [ ] `WorldMapFrame` is the only native managed window. The open-map reset is a state test; simultaneous Options-plus-map UI is not keyboard-reachable because Escape closes the map. Other native windows and generic window-manager parity remain unconverted.

## Out of scope
- World quests, bounties/emissaries: the server sends no world-quest message and its runtime quest table has no `QuestType=3` rows; `TaskInfo`/`QuestPOIBlob`/`QuestPOIPoint` DB2 are not exported.
- Area POIs, events, invasions, rares/vignettes, dungeon/raid entrances: `AreaPOI`, `Vignette`, `JournalInstance` DB2 are not exported and the server sends no such data.
- Fog of war / explored overlays: `WorldMapOverlay(Tile)` DB2 are not exported, and the server never sends discovered zones for players (players carry no `Zone` component).
- Party member pins: `GroupMemberStates` carries positions without a map id and the Godot client does not subscribe to group messages.
- Innkeepers/vendors: NPC flags and spawn positions are server-only.
- Canvas scroll-zoom within a level, floors (`WMOGroupID` maps), map legend and tracking filters.
