# Elastic tree contact

Native flying mounts interact with manually annotated tree models. One real tree is the initial prototype; annotations are reused across placements. Runtime details: [elastic trees](../wiki/systems/elastic-trees.md).

## What it must do

- [ ] Load model-local trunk and major-limb annotations once per model. Cache-once behavior is source-established, not separately tested.
- [x] Reuse annotations across placements; respect translation, rotation and uniform scale.
- [x] Keep the trunk fixed and solid. Glancing flight contact preserves tangential travel instead of stopping the whole movement.
- [x] Let thin limbs yield more than thick limbs, with bounded mount deflection and speed loss.
- [x] Bend the contacted limb visibly around its attachment; recover with damping, preserving continuity during repeated contact.
- [x] Leave foliage nonblocking and keep separate placements' bend states independent.
- [x] Detect fast airborne contact along the full movement segment, not only at its endpoint.
- [x] Remove per-placement tree collision and animation when its doodad is unloaded.

## How it works

- [Elastic tree runtime and authoring](../wiki/systems/elastic-trees.md).

## Implementation inventory

Source inventory reconciled at `34b6ada6`; exact runtime revisions and evidence live in the [wiki evidence boundary](../wiki/systems/elastic-trees.md#evidence-boundary):

- `godot/core/src/elastic_tree.rs`: strict annotation parsing, two-region weights, analytic capsule sweep, tangential response and bounded damped branch state.
- `godot/rust/src/terrain/{elastic_tree,tree_contact,tree_render}.rs`: FDID cache, placement-owned bodies/states/weighted meshes, physics broadphase and placement-owned material shader bends; `assets/mod.rs` and `terrain/objects.rs` attach annotations and replace annotated doodad camera triangles.
- `godot/rust/src/gameplay.rs`: flying movement consumer with fixed radius 1.25 / vertical offset 1.0 sphere; no separate mount-model gate.
- `godot/shaders/{m2.gdshader,tree_bend.gdshaderinc}`: opt-in two-branch position/normal deformation.
- `godot/trees/201394.json`: one real Barrens tree, fixed trunk plus thin/thick limbs. Main accepted manual pivots and coarse vertex regions after lit-image inspection for this prototype, not anatomical perfection or automatic recognition.

## Tests asserting this spec

Passing saved runs and their revision limits: [evidence SSOT](../wiki/systems/elastic-trees.md#evidence-boundary). Refreshed checks for latest source remain pending.

- `godot/core/tests/elastic_tree.rs`: seven tests for validation, stiffness/leverage, recontact/recovery, rigid trunk and region weights, fast/cap/oblique sweep/miss, overlap escape, and tangential response without added energy.
- `gameplay::tests::tree_contact_deflects_flight_and_reports_corrected_position_without_losing_steering`: pure movement with injected analytic contact; corrected packet position, continued flight and reverse steering. Not Godot broadphase or live network proof.
- `godot/tests/elastic_tree_contact.gd`: actual M2 loading, trunk/glancing/high-speed/foliage, transformed placement, thin/thick response, independent state, continuous recontact, recovery and unloaded collision. Optional `ELASTIC_TREE_SHOTS` enables lit captures and rendered-pixel deformation/recovery assertions.
- `godot/tests/elastic_tree_flight.gd`: real private-server mount, physical W/S input, limb contact, server-replicated corrected position and recovery; private-server run passes. Live capture inspection remains pending.

## Prototype boundaries

- One manually annotated model with two limbs, uniform scale and a fixed mount envelope.
- Client-local springs; corrected position replicates, but server does not independently validate tree geometry.
- Crowded broadphase saturation and actual streamed-tile retirement unproved; native subtree unload passes.

[Evidence and remaining inspection/check limits](../wiki/systems/elastic-trees.md#evidence-boundary) are not additional prototype approval requirements.

## Out of scope

- Full tree catalog annotation; start with one representative model.
- Full physical tree simulation, branch breakage and wind.
- New flying-mount controls; reuse the existing native flight path.
