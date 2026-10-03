# Elastic tree contact

Native flying mounts interact with manually annotated tree models. One real tree is the initial prototype; annotations are reused across placements. Runtime details: [elastic trees](../wiki/systems/elastic-trees.md).

## What it must do

- [ ] Load model-local trunk and major-limb annotations once per model; respect placement translation, rotation and scale.
- [ ] Keep the trunk fixed and solid. Glancing flight contact preserves tangential travel instead of stopping the whole movement.
- [ ] Let thin limbs yield more than thick limbs, with bounded mount deflection and speed loss.
- [ ] Bend the contacted limb visibly around its attachment; recover with damping, preserving continuity during repeated contact.
- [ ] Leave foliage nonblocking and keep separate placements' bend states independent.
- [ ] Detect fast airborne contact along the full movement segment, not only at its endpoint.
- [ ] Remove per-placement tree collision and animation when its doodad is unloaded.

## How it works

- [Elastic tree runtime and authoring](../wiki/systems/elastic-trees.md).

## Implementation inventory

Source inventory at `0290ca95`, `7cf63c9b`, `32e9726b` (not runtime acceptance):

- `godot/core/src/elastic_tree.rs`: strict annotation parsing, two-region weights, analytic capsule sweep, tangential response and bounded damped branch state.
- `godot/rust/src/terrain/{elastic_tree,tree_contact,tree_render}.rs`: FDID cache, placement-owned bodies/states/weighted meshes, physics broadphase and placement-owned material shader bends; `assets/mod.rs` and `terrain/objects.rs` attach annotations and replace annotated doodad camera triangles.
- `godot/rust/src/gameplay.rs`: flying movement consumer with fixed radius 1.25 / vertical offset 1.0 sphere; no separate mount-model gate.
- `godot/shaders/{m2.gdshader,tree_bend.gdshaderinc}`: opt-in two-branch position/normal deformation.
- `godot/trees/201394.json`: one real Barrens tree, fixed trunk plus thin/thick limbs. Manual pivots and coarse vertex regions remain provisional until screenshot inspection; no automatic recognition.

## Tests asserting this spec

Assertions inspected; execution and acceptance remain main-owned and pending:

- `godot/core/tests/elastic_tree.rs`: seven tests for validation, stiffness/leverage, recontact/recovery, rigid trunk and region weights, fast/cap/oblique sweep/miss, overlap escape, and tangential response without added energy.
- `gameplay::tests::tree_contact_deflects_flight_and_reports_corrected_position_without_losing_steering`: pure movement with injected analytic contact; corrected packet position, continued flight and reverse steering. Not Godot broadphase or live network proof.
- `godot/tests/elastic_tree_contact.gd`: actual M2 loading, trunk/glancing/high-speed/foliage, transformed placement, thin/thick response, independent state, continuous recontact, recovery and unloaded collision. Optional `ELASTIC_TREE_SHOTS` enables lit captures and rendered-pixel deformation/recovery assertions.
- `godot/tests/elastic_tree_flight.gd`: real private-server mount, physical W/S input, limb contact, server-replicated corrected position and recovery; execution pending.

## Known gaps (current cycle)

- [ ] Native build/runtime, authored mounted-flight/network integration and visual bending acceptance pending; contract bullets remain unchecked.
- [ ] Inspect actual tree pivots/regions in rest, impact and recovered screenshots.
- [ ] Prototype bounded to two branches, uniform placement scale and one fixed mount envelope; crowded broadphase saturation is unproved.

## Out of scope

- Full tree catalog annotation; start with one representative model.
- Full physical tree simulation, branch breakage and wind.
- New flying-mount controls; reuse the existing native flight path.
