# Stockade Entrance

At the Stockade entrance in the Stormwind canals (WoW `-8774, 838`, map 0) the instance portal was not drawn, and walking down the stairs the camera ended up outside the stairwell, under the steps, with the stairwell gone and open Stormwind terrain in view. Evidence and captures: `../../../data/diagnostics/stockadeentrance-20260926/`.

## Instance portal

- **Source.** `sw_magicdistrict` (321999) MODD 1112, `instanceportal.m2` (197007), in group 59 `Jail01` beside area trigger 101. No gameobject spawn and no ADT MDDF places it.
- **Root cause.** `sw_magicdistrict` has MODI but no MODN. The engine looked MODD `name_offset` up as a MODN byte offset, found nothing, and spawned no doodad of any MODI-only WMO, which includes every Stormwind district. With MODI present `name_offset` indexes MODI (WebWowViewerCpp `wmoObject.cpp`: `doodadFileDataIds[doodadDef->name_offset]`). Fixed in `8c0d0fcd`.
- **Particles: fixed** in `50d40c1f`. WMO doodads and every ADT doodad now spawn their M2 emitters; before, WMO doodads spawned none and ADT doodads only for waterfall backdrops (`030090f3`). The portal gets its six emitters and a moving mist in the doorway (`doodadparticles-20260926/final-portal.png`). The graphics particle toggle still gates them. Trade District fps on a loaded host (load 10–15): 1.4–3.7 before, 3.4–4.4 after; that is noise, not a measured cost or gain.

## Camera through the stairwell walls

The stairwell is group 58 `BigJailRoom01` (interior). Portal 42 joins it to `Jail01` at the doorway. A real-data test walks the player down the stairs through `CullingPlugin` + `camera_follow` over every `sw_magicdistrict` group mesh (`terrain_objects_wmo_tests/camera_collision.rs`).

1. **Collision followed render culling (the escape).** Camera smoothing dipped the camera about 0.08 yd under a step. Portal culling found no stairwell floor below it, classified it as outside, and hid group 58. The camera ray only hit visible meshes, so it swung out through the hidden walls to full distance and stayed outside, with 58 culled for good. That matches the user's screenshot. Fix `df515f77`: the camera collides with the batches of portal-culled, non-antiportal WMO groups.
2. **Smoothing path unchecked.** The collision ray validated only the target pose, and the lerp cut under stair noses. That hid 58 on 16 of 91 walk frames. Fix `c55ae4c5`: the smoothed position is ray-checked from the eye.
3. **Portal corner test.** A portal counted as visible only when a corner was in the frustum. A camera 0.15 yd past the doorway plane closed portal 42. Fix `91b14529`: the polygon is clipped by the frustum (WWV `MathHelper::planeCull`), and a portal within 2.25 yd of the camera stays open (WWV `dotepsilon`).

With all three fixes, a 36-pose yaw × pitch matrix of the walk culls the stairwell on no frame.

## Still open

- **Player walls: fixed** in `0c3003bb`. Movement collision casts with `RayCastVisibility::Any`, so portal-culled and off-screen walls block the player.
- **Antiportals: fixed** in `f446b317`. They come from the MOGP flag 0x4000000, not the MOGN name; `sw_magicdistrict` group 62's name did not match, so after `df515f77` its occluder slabs blocked the camera. Antiportal batches carry no `WmoCollisionMesh`.
- **Terrain in interiors: fixed** in `4eadf9b1`. With the eye inside a WMO interior group, the camera skips the terrain clamp, as on WMO-only maps. The canal-street terrain rising through the tunnel (87.2–92.5) had pulled the camera from 8.0 to 5.0 yd.
- Live proof is static poses only: the headless client cannot hold movement keys.

## Godot: instance portal sheet

The portal sheet is `instanceportal.m2` (197007) batch 0: blend 7, shader `0x8022`, texture 7361559, colour 0 = (108, 133, 203)/255, transparency 0.7. Retail shows a blue translucent haze with the room behind tinted blue (`../../../data/diagnostics/portalgodot-20260928/retail-reference.png`); the Godot client drew it grey and straight-alpha.

- **Blend.** `material.rs` rendered blend 7 as `blend_mix` and 5/6 as `blend_add`, 3 as `blend_mix`. WebWowViewerCpp maps M2 blend 0–7 to EGxBlend Opaque, AlphaKey, Alpha, NoAlphaAdd, Add, Mod, Mod2x, BlendAdd (`m2Object.cpp:98-105`) with GL factors `GDeviceGL20.cpp:21-36`: NoAlphaAdd (ONE, ONE), Add (SRC_ALPHA, ONE), Mod (DST_COLOR, ZERO), Mod2x (DST_COLOR, SRC_COLOR), BlendAdd (ONE, ONE_MINUS_SRC_ALPHA). Godot now uses `blend_add` with alpha 1 for 3, `blend_add` for 4, `blend_mul` for 5, `blend_mul` of twice the authored colour for 6 (grey 0.5 stays neutral), `blend_premul_alpha` for 7.
- **Colour.** No M2 colour track reached the shader. WWV multiplies every combiner by `meshColor` (`commonM2Material.slang:61-66`: `matDiffuse = meshColor * tex.rgb`), the batch colour's animated RGB (`animationManager.cpp:1064-1094`); opacity is colour alpha × texture weight (`m2shader_text.slang:72-83`). `godot/core` `m2::batch_mesh_color` feeds `mesh_color`, multiplied in authored space; `WowMaterialAnimation` samples animated colour RGB, transparency and colour alpha with the effect-UV clock rule.
- **Proof.** `godot/tests/m2_portal_pixels.gd` loads the real batch material with a known texel over a known background: RED `[120,162,115]`, GREEN `[84,149,142]` against WWV's shading and factors `[85,149,142]`. `m2_blend_pixels.gd` covers modes 0, 2–7 and a two-key colour track (RED on 3, 5, 6, 7 and colour; GREEN 10/10).
- **Framebuffer.** WWV's UNORM framebuffer (`GFrameBufferVLK.cpp:21`) blends in gamma space; Godot blends in its linear framebuffer, as Bevy did. The expected value above uses linear blending; a gamma blend of the same inputs is `[106,186,156]`.
- **Drawn in world** since branch `wmodoodads` (`ab510ad7`): Godot spawns WMO MODD doodads (see [[wmo-format]] for the set rule), so MODD 1112 of `sw_magicdistrict` stands in the Jail01 doorway, 2.5 yd from area trigger 101's box centre. `wmodoodads-20260928/after-view0.png` is the live client from behind the player on the stairs (compare Retail `portalgodot-20260928/retail-reference.png`); `before-game.png`/`after-game.png` are the client's own camera without and with doodads. Earlier `portalgodot-20260928` images placed 197007 by hand.
- **Particles.** The Godot client has no particle system; the portal's six emitters (the white sparkles) are missing.

## Godot: player under the entrance floor

In the Godot client the Stockade entrance showed the stairwell as a plank floor the player could not get past. No WMO, ADT doodad, gameobject or terrain surface lies above the stairwell treads in the data; the planks are authored `mm_strmwnd_jail_trim_01` surfaces (group 58/59, material 70), and a native render of 321999 over tile 30_48 draws the stairs as authored. The cause was the ground: on continents the Godot `TerrainGround` passed only a WDT global WMO to `shared::ground::ground_at`, so ADT-placed WMO floors were not candidates. From the server's position on the room floor (WoW -8785.93, 820.67, 97.65) the local prediction fell to the flat, hole-free terrain at 86.21 and walked under the room floor and the stairs, while the server kept the player on the WMO floors. `TerrainObjects` now keeps each spawned WMO's placed collision and the player's ground includes it.

## Godot: reaching and entering the Stockade

A live walk (`godot/tests/stockade_walk.gd`, real arrow/W keys, own dev-server account) from the room-floor spawn down the stairwell into area trigger 101 found four client faults in a row; the server fired the trigger and transferred every time.

1. **Floors before the WMO spawned.** The previous fix took WMO floors from `TerrainObjects`, which spawns the ~8,200 placements of the 3x3 tiles at 8 ms a frame (minutes). Entering on the room floor (97.65) or at the Stockade exit arrival on the doorway (88.0), the prediction fell to the terrain (86.21) first; the floor was then more than `STEP_UP_HEIGHT` above the feet, so the player stayed under the building while the server kept it on the floor. The terrain worker now reads each tile's MODF WMO floors (`NativeTerrainTile.wmo_floors`), as the server's `GroundMap` does (`8fdc22d0`, `6b89cb28`).
2. **Lighting map ID.** `update_world_lighting` resolved the map ID from a hard-coded continent table; `stormwindjail` failed "No authored map ID" every frame, and the process chain stopped the account right after the transfer. The ID now comes from `NewWorld.map_id` or `Map.csv` `Directory` (`347f86c2`).
3. **ADT tiles on a WMO-only map.** Loading readiness requested `stormwindjail_31_31.adt` for the player's tile and recorded a failure (`a98b6029`).
4. **No global WMO, no camera.** Native global-WMO spawn did not exist, so the Stockade loading screen never finished; the world camera also waited for ADT tiles and kept its Stormwind pose after a transfer. `wmo::global::GlobalWmoScene` spawns the WDT WMO and reports `Spawned` to loading readiness; the camera runs on a global-WMO map and resets on `NewWorld`.

A server-side dead character does not move (`process_player_inputs` drops a corpse's input), and the Godot client has no release UI: a character killed in the Stockade stays at its spawn, with the client walking alone. `stockade_walk.gd` fails fast on health 0.

Still open: particles are absent in Godot.

## Godot: camera outside the walls

The Godot camera rays hit only physics bodies, and only terrain chunks had any: WMOs were meshes, and their collision data was used only for the player's ground. Inside the Stockade and on the entrance stairs the camera stayed at its full orbit behind the walls, showing the WMO from outside against the grey void. Fix: `wmo::collision::WmoCollisionBodies` builds physics shapes (layer 2) from the same shared collision faces as the floors, for the global WMO and each parsed tile's MODF WMOs, and the camera ray queries them; the smoothed position is ray-checked from the eye as in Bevy `c55ae4c5`.

- Rust, real faces (`godot/rust/src/wmo/collision_tests.rs`): walking down the stairwell (yaw -50°, pitch 10°, 10 yd) the camera is behind a wall on 0 of 91 frames and ends under 9 yd; without WMO faces, on 91 of 91 at 10.0 yd. In `stormwindjail` (108631) no yaw of a 15 yd orbit ends behind a wall; without faces, yaw 0 does.
- Live (`godot/tests/world_wmo_camera_collision.gd`, Fbcamera, 8 turn-key steps): Stockade entrance, all 8 steps at 15.0 yd before; 4.6–13.9 yd after, none behind a wall. Stormwind stairs, 15.0 yd before (the view under the floor); 4.8–8.2 yd on 7 steps and 15.0 on the open one after.
- Player walls use the same bodies: `TerrainGround::validate_move` first clamps the move with the original wall ray (0.6 yd up, 0.05 yd margin, no slide). Live (`godot/tests/world_wmo_wall_walk.gd`), 1.5 s of held W across the stairwell from the top step walked 10.50 yd through the wall and fell to the terrain at 86.2, on the client and, since the server adopts client x/z, on the server; after, both stop at 6.22 yd, the Bevy figure. `stockade_walk.gd` still walks down the stairwell into the Stockade.
- Cost: one Stormwind tile is 62 WMOs, 515 groups, ~350k triangles. Built in one frame that took 460–570 ms (debug build); with a 2 ms budget, nearest group first, the worst frame is 14 ms and all shapes exist ~140 frames after the tile parses, the stairwell's first. A camera ray costs 4.1 µs with WMO bodies against 2.9 µs without; frame rate with and without the bodies differs less than the run-to-run noise (28–60 FPS on this loaded host).

## See Also

- [[stormwind-hilly-plaza]]: the portal traversal these fixes refine
- [[collision-system]]: camera collision policy
- [[wmo-format]]: MODD/MODI
