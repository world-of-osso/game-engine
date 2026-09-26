# Stockade Entrance

At the Stockade entrance in the Stormwind canals (WoW `-8774, 838`, map 0) the instance portal was not drawn, and walking down the stairs the camera ended up outside the stairwell, under the steps, with the stairwell gone and open Stormwind terrain in view. Evidence and captures: `../../../data/diagnostics/stockadeentrance-20260926/`.

## Instance portal

- **Source.** `sw_magicdistrict` (321999) MODD 1112, `instanceportal.m2` (197007), in group 59 `Jail01` beside area trigger 101. No gameobject spawn and no ADT MDDF places it.
- **Root cause.** `sw_magicdistrict` has MODI but no MODN. The engine looked MODD `name_offset` up as a MODN byte offset, found nothing, and spawned no doodad of any MODI-only WMO, which includes every Stormwind district. With MODI present `name_offset` indexes MODI (WebWowViewerCpp `wmoObject.cpp`: `doodadFileDataIds[doodadDef->name_offset]`). Fixed in `8c0d0fcd`.
- **Still open.** The portal's six particle emitters are not spawned: WMO doodads spawn no emitters, and ADT doodads spawn them only for waterfall backdrops. Only the portal's mesh is drawn.

## Camera through the stairwell walls

The stairwell is group 58 `BigJailRoom01` (interior). Portal 42 joins it to `Jail01` at the doorway. A real-data test walks the player down the stairs through `CullingPlugin` + `camera_follow` over every `sw_magicdistrict` group mesh (`terrain_objects_wmo_tests/camera_collision.rs`).

1. **Collision followed render culling (the escape).** Camera smoothing dipped the camera about 0.08 yd under a step. Portal culling found no stairwell floor below it, classified it as outside, and hid group 58. The camera ray only hit visible meshes, so it swung out through the hidden walls to full distance and stayed outside, with 58 culled for good. That matches the user's screenshot. Fix `df515f77`: the camera collides with the batches of portal-culled, non-antiportal WMO groups.
2. **Smoothing path unchecked.** The collision ray validated only the target pose, and the lerp cut under stair noses. That hid 58 on 16 of 91 walk frames. Fix `c55ae4c5`: the smoothed position is ray-checked from the eye.
3. **Portal corner test.** A portal counted as visible only when a corner was in the frustum. A camera 0.15 yd past the doorway plane closed portal 42. Fix `91b14529`: the polygon is clipped by the frustum (WWV `MathHelper::planeCull`), and a portal within 2.25 yd of the camera stays open (WWV `dotepsilon`).

With all three fixes, a 36-pose yaw × pitch matrix of the walk culls the stairwell on no frame.

## Still open

- Player wall collision (`clamp_movement_against_wmo_meshes`) uses the default `VisibleInView` ray, so portal-culled and off-screen walls do not block the player.
- The camera terrain clamp still applies inside WMO interiors. Terrain rises to 92.5 at the portal (tunnel floor about 87); this was not exercised live.
- Live proof is static poses only: the headless client cannot hold movement keys.

## See Also

- [[stormwind-hilly-plaza]]: the portal traversal these fixes refine
- [[collision-system]]: camera collision policy
- [[wmo-format]]: MODD/MODI
