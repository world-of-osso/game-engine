# Stormwind Hilly Plaza

In Stormwind's Trade District the ground was hilly cobblestone instead of a flat paved plaza. Heights, holes and textures were all correct. Portal culling was hiding every group of `sw_tradedistrict` (322057), so the ADT terrain under the buildings showed instead. That terrain is authored with hills and a cobble layer because the district WMO normally covers it. Fixed in `5763a9ea`.

## Symptom

Near Auctioneer Fitch (WoW `-8818, 660, 97`) the player stood on rolling cobblestone. There were no Trade District buildings, and the auctioneers stood in dips. Evidence: the user's `user-hilly-plaza.png`, reproduced on master `9de26672` as `before.webp`.

## Hypotheses

- **WMO groups hidden: the cause.** `before-tree.txt` shows all 62 groups hidden in each of the four `wmo_322057` copies.
- **Terrain holes not cut: refuted.** The nine MCNKs around the spot (tile `azeroth_30_48`, chunks 11–13 × 7–9) have flags `0x18040`, so high-res holes are in use, and every high-res hole mask is 0. The mesh already skips hole quads (`build_mcnk_indices`).
- **Texture scale or layer choice: refuted.** The cobble layer is authored terrain. Retail doesn't show it here because the WMO paving covers it.
- **Height decode: refuted.** Decoded terrain on a 10 yd grid around the spot never rises above the WMO ground that covers it. For example, at `-8810, 650` the terrain is 99.4 and the `tradeNW` paving is 99.4. At `-8820, 660` the terrain is 94.7 and the auction house floor is 98.1. Where the terrain rises to 99–103, trade district geometry sits above it at most of the sampled points. A few samples have no trade district geometry; other district WMOs were not checked there.

## Root cause

`wmo_portal_cull_system` had three faults:

1. **Antiportal AABB occlusion.** Group 31 (`antiportal`) is a set of vertical wall slabs inside the buildings, and its bbox is about 259 × 345 × 48 yd. A group was hidden when the segment from the camera to its center hit an antiportal's AABB. A segment that starts inside the box always hits it, so every group was hidden. WebWowViewerCpp does not cull with antiportals; it only draws them as a debug overlay (`renderAntiPortals`).
2. **Camera group from any bbox.** The camera group was the first group whose bbox contained the camera. The camera at `-8818, 645.7, 100.9` is inside the bboxes of groups 31, 35, 36, 37, 42, 53 and 60. The only group with geometry below it is 36, which is exterior paving at 94.9. The camera is outside the auction house walls, even though it is inside the bbox of the auction house interior (group 53). The BFS then started from whichever group matched first.
3. **Stale visibility outside.** When no bbox contained the camera, the system returned early, so groups kept their last visibility.

## Fix

The traversal now follows WebWowViewerCpp (`Map::checkExterior`, `WmoObject::startTraversingWMOGroup`, `getGroupWmoThatCameraIsInside`):

- `WmoGroup.is_exterior` holds MOGP `EXTERIOR` (0x8). Groups without it are interiors.
- Interior, non-antiportal groups carry `WmoInteriorFloor`, their batch-mesh triangles in WMO-local space. The camera is inside an interior group when the group's bbox contains it and a triangle lies below it. If several qualify, the closest floor wins.
- **Inside an interior:** BFS through in-frustum portals. If that reaches an exterior group, every exterior group is drawn, plus interiors behind their in-view portals.
- **Otherwise:** every exterior group is drawn, and interiors are drawn through in-view portals.
- Antiportal groups are never drawn and no longer occlude anything.

## Proof

- **Real-data tests.** `portal_culling.rs` (`src/rendering/terrain/terrain_objects_wmo_tests/`) spawns the real MODF placement and all 62 groups through `spawn_wmo_group_entity`, and runs `CullingPlugin` with a 60° perspective frustum.
  - `trade_district_exterior_groups_are_drawn_from_the_auction_house_plaza` uses the user's camera. RED: group 36 `Hidden`. GREEN: 36, 37 and 60 visible, antiportal 31 hidden.
  - `trade_district_auction_house_interior_is_drawn_from_inside` uses a camera on the auction house floor. RED: 53 `Hidden`. GREEN: 53 visible.
- **Synthetic tests.** `camera_outside_interiors_draws_exterior_groups` and `interior_portal_onto_exterior_draws_every_exterior_group` in `culling_tests.rs`.
- **Live, headless.** Screenshots on the shared :5000 server as `Cityground` (`city_ui`), spawned at `-8818, 660, 97`:
  - `before.webp`: master; 62 of 62 groups hidden, hilly terrain.
  - `after.webp`: fix; 37 shown and 25 hidden, from the portal traversal.
  - `after-plaza.webp`: `-8790, 630`; flat paving and buildings.
  - Scene and tree dumps sit beside each capture, and `capture.sh` reproduces them.

## Still open

- **No WMO floor support.** At the auctioneer spot the player stands on terrain at 94.7, under the auction house floor at 98.06, so `after.webp` looks up at the floor from below. The auctioneers stand on the floor above. This is the missing WMO vertical collision described in [[collision-system]]. Camera collision now hits the visible WMO, so the after camera sits closer than the before camera.
- **Interior test uses render triangles.** Retail uses the group BSP (MOBN/MOBR) with collision faces only, and it compares against exterior floors too. Here a closer exterior floor inside an interior's bbox does not win.
- **Portal test is coarse.** A portal counts as visible when any of its vertices is in the frustum. There is no portal-clipped frustum.
- **Duplicate placements.** `sw_tradedistrict` is still spawned once per referencing tile (4×). See [[stormwind-dark-render]].

## Sources

- `../../../data/diagnostics/cityground-20260925/`: captures, dumps, `capture.sh`, `create-char.js`
- WebWowViewerCpp `wowViewerLib/src/engine/objects/wmo/wmoObject.cpp` and `objects/scenes/map.cpp` (read with `gh api`): camera group selection, exterior and interior traversal, antiportals used as debug-only

## See Also

- [[stormwind-dark-render]]: the same district WMOs, material path
- [[abbey-interior-black-world]]: exterior groups hidden from inside by the old traversal
- [[wmo-format]]: group flags and portals
- [[collision-system]]: missing WMO floor support
