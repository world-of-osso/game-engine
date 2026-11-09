# Outdoor visibility through WMO portals

Indoor cameras admit outdoor scene content through visible exterior portal windows. WMO portal logic lives in `godot/rust/src/wmo/portals.rs`; outdoor scenery uses the same admission in the world object and terrain paths.

## What it must do

- [ ] A fully enclosed interior admits no outdoor color geometry. Objects overlapping interior groups remain eligible.
- [x] Independently admit in-frustum WMO groups whose root MOGI flags contain any bit of `0x20010008`, except antiportals (`0x04000000`).
- [ ] One or more visible exterior portal paths admit outdoor geometry intersecting their clipped windows. Preserve doorway transitions, overlapping WMOs, and split groups.
- [ ] Camera rejection does not remove shadows that can reach visible geometry. Preserve double-sided casting, distance fades, animations and particles.
- [ ] Apply outdoor admission to ADT doodads, terrain, other WMOs, liquids and sky-adjacent scene content without removing visible geometry.
- [ ] Apply it to remote units by their model and mount boxes; the local player is always admitted. Inactive attachments keep casting nothing.
- [ ] Confirm enclosed, doorway and exterior views with rendered images and source-specific draw counts.
- [x] Opaque batches of EXTERIOR (MOGP `0x8`) groups of WMOs spanning at least 100 yd occlude camera geometry behind them, across WMOs (Godot occlusion culling). Smaller WMOs, interior groups, blended and alpha-keyed batches never occlude; shadow casting is unchanged.

## How it works

- [WMO format and portal records](../wiki/formats/wmo-format.md)
- [Stormwind frame-time investigation](../wiki/investigations/godot-stormwind-fps.md)

## Implementation inventory

- `godot/rust/src/wmo/portals.rs` — camera group and portal traversal.
- `godot/rust/src/wmo/occluder.rs` and `wmo/scene.rs` (`WmoBuild`) — one `ArrayOccluder3D` per WMO of at least 100 yd from its drawn opaque exterior batches; `godot/project.godot` `rendering/occlusion_culling/*` (128 rays per thread, Low BVH build quality).
- `godot/rust/src/terrain/objects.rs` — WMO and doodad admission.
- `godot/rust/src/assets/material.rs` — scenery fade and independent camera/shadow state.
- `godot/rust/src/terrain/material.rs` — terrain chunks.
- `godot/rust/src/world.rs` (`cull_outdoor`) and `godot/rust/src/animation/attachment.rs` (`refresh_unit_shadows`) — units.

## Tests asserting this spec

- `godot/rust/src/wmo/portals.rs` — portal-window geometry tests.
- `godot/rust/src/wmo/occluder.rs` — occluding batch selection, the size threshold and concatenated indices.
- `godot/tests/m2_material_pixels.gd` — receiver shadow pixel with camera rejection.
- `godot/tests/outdoor_scenery_shadow_modes.gd` — restoration and fade cases.
- `godot/tests/unit_outdoor_admission.gd` — unit rejection, windows, local player, new visuals and attachments.

## Known gaps (current cycle)

- [ ] Admit other WMOs, liquids and sky-adjacent content through exterior windows.
- [ ] Verify terrain shadow preservation and views through an exterior doorway in the rendered world.
- [ ] Resolve the two-group camera seed rules and the background behind the unportaled Halls of Infusion opening; see [investigation](../wiki/investigations/halls-interior-opening.md).
- [ ] Apply the verified MOGI `0x10000` portal-walker stop rule.
- [ ] Occlusion culling has no rendered fixture; it is verified by draw counts and pixel diffs in the [Stormwind investigation](../wiki/investigations/godot-stormwind-fps.md).

## Out of scope

- Shadow-caster LOD and sub-texel culling: separate measured changes after admission.
