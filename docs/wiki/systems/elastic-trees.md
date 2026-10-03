# Elastic trees (native prototype)

Source audit of `0290ca95`, `7cf63c9b`, and `32e9726b`: manual per-model capsules and vertex regions drive fixed-trunk contact, two elastic limbs and per-placement rendering. One real Barrens tree, FDID **201394**, is annotated. Native execution, screenshots and integration acceptance remain main-owned and pending; fixture assertions are not proof of a passing run. Contract: [elastic trees](../../specs/elastic-trees.md).

## Annotation authoring

1. Use a numeric model path such as `data/models/201394.m2`; author `godot/trees/201394.json` (`res://trees/201394.json`). `model_fdid` must match the filename.
2. Author coordinates in model-local **engine axes**, converted from WoW `(x,y,z)` to `(x,z,-y)`; Y is up. Placement translation, rotation and uniform scale are applied later. Nonuniform scale is not supported by the radius conversion.
3. Set fixed `trunk.start`, `trunk.end` and `trunk.radius`. Each branch has unique `name`, `pivot`, `tip`, `radius`, `stiffness`, `damping`, `max_angle` (radians), and strictly ordered `region_min`/`region_max` vertex-selection bounds. Parser and renderer permit **at most two branches**, not an arbitrary limb catalog.
4. Inspect the real model's rest/impact/recovered screenshots before accepting pivots or regions. Current selections are **provisional coarse manual annotations**, not automatic recognition or validated anatomical segmentation.

Current trunk runs `[0,0,0]` → `[0,18,0]`, radius 3. `left-thin` uses pivot `[-3,19,-1]`, tip `[-24.24,26.39,-6.72]`, radius 0.65, stiffness 18, damping 5, maximum angle 0.55. `right-thick` uses pivot `[3,17,0]`, tip `[28.76,13.04,-1.07]`, radius 1.5, stiffness 90, damping 12, maximum angle 0.25. Exact region boxes remain in the JSON, not duplicated here.

Only trunk/limb capsules block; there is no foliage collider or foliage schema field. Foliage vertices inside a selected region can bend with that limb. Trunk-capsule vertices remain rigid; branch weights use smoothstep axial leverage inside their AABBs, with overlapping weights normalized to a total no greater than one.

## Loading and render ownership

`load_annotation` recognizes numeric M2 stems and caches each FDID's parsed `Arc<TreeAnnotation>`, absence, or error in a thread-local map. Missing JSON means unannotated; malformed/mismatched JSON fails model construction. Cache has no live authoring reload: restart the client after edits.

`build_model` attaches an `ElasticTree` child. Each placement owns branch spring states, capsule bodies and weighted copies of direct `Batch*` ArrayMeshes; cached source meshes and shared materials are not mutated. Red/green vertex-color channels store the two weights. Instance shader parameters carry pivots and axis-angle rotations; `m2.gdshader` rotates positions and normals. Prepared bend-expanded AABBs are restored after animation changes. Skeleton bindings and material overrides remain unchanged.

Annotated ADT doodads skip their ordinary authored-triangle camera collider. Their trunk/limb bodies instead carry tree layer `1 << 4` and `DOODAD_LAYER`, so camera queries can see these capsules too. Freeing the doodad subtree removes its controller, render meshes and physics bodies; the model annotation cache persists.

## Contact and recovery lifecycle

The local `PlayerMovement::fly` path calls tree contact before existing ground/WMO validation. Gating is the movement's `flying` flag (from the existing can-fly/takeoff rules), not a separate mounted-model test. Prototype envelope is a **fixed sphere radius 1.25**, centered **1.0 above player position**, not fitted to the current mount.

Godot `intersect_shape` queries a box enclosing the full swept sphere segment on the tree layer, deduplicates tree parents, then analytic model-local sphere-versus-capsule sweeps choose earliest trunk or currently bent limb contact. This is not endpoint overlap or a scan of every placement. Uniform scale adjusts mover radius; world normals are normalized after transformation. Broadphase result saturation is not established by current fixtures.

Contact leaves a 0.01 margin and removes only inward normal motion. Trunk resistance is 1; limb resistance is `stiffness / (stiffness + 100)`. Tangential/outward travel remains. Branch impulse uses incoming velocity times prototype momentum 60, axial leverage and inverse stiffness; contact changes angular velocity without resetting pose. A branch is excluded after contact within that sweep, allowing yielding travel. At most four contacts are resolved; exhausted budget returns the last checked position.

Each physics tick advances the closed-form damped spring, clamps rotation to `max_angle`, uploads bends and moves branch capsules about their fixed pivots. Separate placements never share spring state. Recovery continues without further contact; repeated impacts preserve the current rotation.

## Evidence boundary

Seven core behavioral tests assert strict parsing, thin/thick and tip leverage, continuous recontact/long-frame recovery, rigid trunk/region weights, fast/cap/oblique sweeps and misses, initial-overlap escape, and tangential energy-nonincreasing response. A pure gameplay test adds corrected flight reporting and reverse steering with an injected analytic contact closure, not Godot broadphase or a live network. These assertions were inspected, **not executed in this docs audit**.

`godot/tests/elastic_tree_contact.gd` loads the actual M2 twice and asserts fast/glancing trunk contact, a foliage miss, rotated/scaled placement, thin/thick yielding, independent states, recontact continuity, recovery and collision removal. Optional `ELASTIC_TREE_SHOTS` captures rest/impact/recovered images; numeric angle assertions alone do not prove visible mesh bending. Authored flight/network integration, visual region/pivot acceptance and native build/runtime proof remain pending.

## Sources

- [Prototype contract](../../specs/elastic-trees.md) — required behavior and pending acceptance.
- `godot/trees/201394.json` — manual model annotation.
- `godot/core/src/elastic_tree.rs`, `godot/core/tests/elastic_tree.rs` — validation, weights, sweep and spring math.
- `godot/rust/src/terrain/{elastic_tree,tree_contact,tree_render,objects}.rs`, `godot/rust/src/assets/mod.rs` — cache, placement ownership, physics/render integration.
- `godot/rust/src/gameplay.rs`, `godot/shaders/{m2.gdshader,tree_bend.gdshaderinc}`, `godot/tests/elastic_tree_contact.gd` — flight consumer, deformation and native fixture.

## See Also

- [[mounts]] — existing flight controls and network reporting.
- [[collision-system]] — other collision layers and camera queries.
- [[terrain]] — ADT doodad placement and unloading.
