# Outdoor visibility through WMO portals

Indoor cameras admit outdoor scene content through visible exterior portal windows. WMO portal logic lives in `godot/rust/src/wmo/portals.rs`; outdoor scenery uses the same admission in the world object and terrain paths.

## What it must do

- [ ] A fully enclosed interior admits no outdoor color geometry. Objects overlapping interior groups remain eligible.
- [ ] One or more visible exterior portal paths admit outdoor geometry intersecting their clipped windows. Preserve doorway transitions, overlapping WMOs, and split groups.
- [ ] Camera rejection does not remove shadows that can reach visible geometry. Preserve double-sided casting, distance fades, animations and particles.
- [ ] Apply outdoor admission to ADT doodads, terrain, other WMOs, liquids and sky-adjacent scene content without removing visible geometry.
- [ ] Apply it to remote units by their model and mount boxes; the local player is always admitted. Inactive attachments keep casting nothing.
- [ ] Confirm enclosed, doorway and exterior views with rendered images and source-specific draw counts.

## How it works

- [WMO format and portal records](../wiki/formats/wmo-format.md)
- [Stormwind frame-time investigation](../wiki/investigations/godot-stormwind-fps.md)

## Implementation inventory

- `godot/rust/src/wmo/portals.rs` — camera group and portal traversal.
- `godot/rust/src/terrain/objects.rs` — WMO and doodad admission.
- `godot/rust/src/assets/material.rs` — scenery fade and independent camera/shadow state.
- `godot/rust/src/terrain/material.rs` — terrain chunks.
- `godot/rust/src/world.rs` (`cull_outdoor`) and `godot/rust/src/animation/attachment.rs` (`refresh_unit_shadows`) — units.

## Tests asserting this spec

- `godot/rust/src/wmo/portals.rs` — portal-window geometry tests.
- `godot/tests/m2_material_pixels.gd` — receiver shadow pixel with camera rejection.
- `godot/tests/outdoor_scenery_shadow_modes.gd` — restoration and fade cases.
- `godot/tests/unit_outdoor_admission.gd` — unit rejection, windows, local player, new visuals and attachments.

## Known gaps (current cycle)

- [ ] Admit other WMOs, liquids and sky-adjacent content through exterior windows.
- [ ] Verify terrain shadow preservation and views through an exterior doorway in the rendered world.

## Out of scope

- Shadow-caster LOD and sub-texel culling: separate measured changes after admission.
