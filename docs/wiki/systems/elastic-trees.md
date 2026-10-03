# Elastic trees (native prototype)

Evidence reconciled at `34b6ada6` on 2026-10-03: manual per-model capsules and vertex regions drive fixed-trunk contact, two elastic limbs and per-placement rendering. One real Barrens tree, FDID **201394**, has main-accepted coarse prototype annotations and passing native lit-contact and private mounted-flight runs. Independent integrated verification passed; exact proof scopes appear below. Contract: [elastic trees](../../specs/elastic-trees.md).

## Annotation authoring

1. Use a numeric model path such as `data/models/201394.m2`; author `godot/trees/201394.json` (`res://trees/201394.json`). `model_fdid` must match the filename.
2. Author coordinates in model-local **engine axes**, converted from WoW `(x,y,z)` to `(x,z,-y)`; Y is up. Placement translation, rotation and uniform scale are applied later. Nonuniform scale is not supported by the radius conversion.
3. Set fixed `trunk.start`, `trunk.end` and `trunk.radius`. Each branch has unique `name`, `pivot`, `tip`, `radius`, `stiffness`, `damping`, `max_angle` (radians), and strictly ordered `region_min`/`region_max` vertex-selection bounds. Parser and renderer permit **at most two branches**, not an arbitrary limb catalog.
4. Inspect the real model's rest/impact/recovered screenshots before accepting pivots or regions. Main inspected the lit rest/impact images and accepted these **coarse manual annotations for the prototype**, not anatomically perfect segmentation or automatic recognition.

Current trunk runs `[0,0,0]` → `[0,18,0]`, radius 3. `left-thin` uses pivot `[-3,19,-1]`, tip `[-24.24,26.39,-6.72]`, radius 0.65, stiffness 18, damping 5, maximum angle 0.55. `right-thick` uses pivot `[3,17,0]`, tip `[28.76,13.04,-1.07]`, radius 1.5, stiffness 90, damping 12, maximum angle 0.25. Exact region boxes remain in the JSON, not duplicated here.

Only trunk/limb capsules block; there is no foliage collider or foliage schema field. Foliage vertices inside a selected region can bend with that limb. Trunk-capsule vertices remain rigid; branch weights use smoothstep axial leverage inside their AABBs, with overlapping weights normalized to a total no greater than one.

## Loading and render ownership

`load_annotation` recognizes numeric M2 stems and caches each FDID's parsed `Arc<TreeAnnotation>`, absence, or error in a thread-local map. Missing JSON means unannotated; malformed/mismatched JSON fails model construction. Cache has no live authoring reload: restart the client after edits.

`build_model` attaches an `ElasticTree` child. Each placement owns branch spring states, capsule bodies and weighted copies of direct `Batch*` ArrayMeshes; cached source meshes and shared materials are not mutated. Red/green vertex-color channels store the two weights. Each batch's independently allocated `ShaderMaterial` carries pivots and axis-angle rotations; `m2.gdshader` rotates positions and normals. The original instance-uniform shader approach allocated tree slots for **all M2 instances**, exhausting Godot's global buffer. `1de713b8` replaced it with ordinary uniforms on the existing distinct per-batch materials; no extra material ownership layer is needed. Prepared bend-expanded AABBs are restored after animation changes. Skeleton bindings and material overrides remain unchanged.

Annotated ADT doodads skip their ordinary authored-triangle camera collider. Their trunk/limb bodies instead carry tree layer `1 << 4` and `DOODAD_LAYER`, so camera queries can see these capsules too. Freeing the doodad subtree removes its controller, render meshes and physics bodies; the model annotation cache persists.

## Contact and recovery lifecycle

The local `PlayerMovement::fly` path calls tree contact before existing ground/WMO validation. Gating is the movement's `flying` flag (from the existing can-fly/takeoff rules), not a separate mounted-model test. Prototype envelope is a **fixed sphere radius 1.25**, centered **1.0 above player position**, not fitted to the current mount.

Godot `intersect_shape` queries a box enclosing the full swept sphere segment on the tree layer, deduplicates tree parents, then analytic model-local sphere-versus-capsule sweeps choose earliest trunk or currently bent limb contact. This is not endpoint overlap or a scan of every placement. Uniform scale adjusts mover radius; world normals are normalized after transformation. Broadphase result saturation is not established by current fixtures.

Contact leaves a 0.01 margin and removes only inward normal motion. Trunk resistance is 1; limb resistance is `stiffness / (stiffness + 100)`. Tangential/outward travel remains. Branch impulse uses incoming velocity times prototype momentum 60, axial leverage and inverse stiffness; contact changes angular velocity without resetting pose. A branch is excluded after contact within that sweep, allowing yielding travel. At most four contacts are resolved; exhausted budget returns the last checked position.

Each physics tick advances the closed-form damped spring, clamps rotation to `max_angle`, uploads bends and moves branch capsules about their fixed pivots. Separate placements never share spring state. Recovery continues without further contact; repeated impacts preserve the current rotation.

## Evidence boundary

Saved evidence, not reruns by this docs audit:

- **Native lit contact:** production `c659e8d3`, Depot build `zlhxfjpkzx` exit 0; fixture `d7cbee25` exit 0. `data/diagnostics/elastic-trees-2026-10-03/native-rendered-contact-d7cbee25.log` records fixed trunk, glancing/high-speed contact, foliage miss, transformed placement, thin/thick yielding, independent states, continuous recontact, recovery and subtree collision removal. Actual M2 loaded twice. Thin/thick angles are 0.253056/0.072250 radians; impact changes 17,363 pixels, recovery changes exactly 0. Main inspected lit images and accepted coarse regions; verifier independently decoded the pixel comparison.
- **Mounted input/network:** fixture `ed0da43b` with production `c659e8d3` passes in `data/diagnostics/elastic-trees-2026-10-03/mounted-flight.log`. Fresh owned level-70 Paladin `Elasticfly`, Golden Gryphon, private UDP 5186 server and dedicated redb; no user-world changes. W crosses the thin limb: peak 0.5386369 radians, travel 28.4136, lateral deflection 0.0893676; replicated server position exactly equals client `(-8948.916,148.5206,-36.22149)`. S reverses and the branch recovers. Main inspected three `live-screenshots/` captures; foliage occludes the rider during contact, so they do not establish pixel-perfect mount/hero contact visuals. Numeric proof is not a flight-rate benchmark. Log retains unrelated spell-attachment/UI-asset errors, not a clean whole-client run.
- **Integrated commands:** production `34b6ada6` includes merged master `074aa434`; documentation-only successor `e4aadc2a` passed independent actual Depot check `ckwx9h82hf` and core tests `8h9wrcrp1p` (7/7). All 12 changed Rust files pass focused formatting; 51 owned functions have maximum body length 29 and cognitive complexity 7. Native extension build `v9wcfrsm1r` exits 0. Gameplay 27/27 at `1de713b8` is retained after unchanged-algorithm source comparison. The integrated report records equivalent validation changes and why particle-free tree runtime evidence remains applicable. Existing `NativeWmoGroup.fdid` warning and baseline global-format failures remain outside this feature's proof. Saved reports/logs are under `data/diagnostics/elastic-trees-2026-10-03/proofs/`.

Earlier live attempts used incorrect WoW-versus-engine spawn axes and killed the original private character. A fresh owned character restored the terrain/mount prerequisite; no flight-code fix was required.

## Prototype boundaries

Two annotated limbs on one model; uniform placement scale only; one fixed contact sphere (radius 1.25, height offset 1.0). Springs are client-local: the server receives corrected mount position but does not independently validate tree geometry. Crowded broadphase result-cap behavior and actual streamed-tile retirement remain unproved (subtree unload passes). Cache-once-per-FDID is source-established, not separately behavior-tested. These bounds are not new approval requirements.

## Sources

- [Prototype contract](../../specs/elastic-trees.md) — required behavior and bounded acceptance.
- `godot/trees/201394.json` — manual model annotation.
- `godot/core/src/elastic_tree.rs`, `godot/core/tests/elastic_tree.rs` — validation, weights, sweep and spring math.
- `godot/rust/src/terrain/{elastic_tree,tree_contact,tree_render,objects}.rs`, `godot/rust/src/assets/mod.rs` — cache, placement ownership, physics/render integration.
- `godot/rust/src/gameplay.rs`, `godot/shaders/{m2.gdshader,tree_bend.gdshaderinc}`, `godot/tests/{elastic_tree_contact,elastic_tree_flight}.gd` — flight consumer, deformation and native fixtures.
- Saved logs and `data/diagnostics/elastic-trees-2026-10-03/proofs/verify-integrated.md` — revision-scoped runtime and independent verification evidence; main supplied private-server setup and visual inspection.

## See Also

- [[mounts]] — existing flight controls and network reporting.
- [[collision-system]] — other collision layers and camera queries.
- [[terrain]] — ADT doodad placement and unloading.
