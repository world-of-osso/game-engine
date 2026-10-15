# Coverage, exclusions and proof ledger

[Backlog index and revision pins](README.md). Source inspection is not native-client validation or an exhaustive compatibility audit.

## Capability matrix

| Area | Inspected evidence | Established gap | Still missing / blocked | Proof level |
|---|---|---|---|---|
| Generic M2 track evaluation | W `M2Model.cpp` specializations; E `m2_anim.rs`, native `animation/mod.rs`, ribbon sampler | A1–A3 | Authored-asset census, native bone/edge samples; Hermite/Bezier scope not established | Paired source + explicit input/output examples |
| Particle geometry and birth/motion | W `M2Model.cpp`, `Services/ParticleSystem2.cpp`; E parser, simulation and native placement | P1–P5 | Real tail/follow/inheritance/spline/model-particle fixtures and native output; modern scale/spline/model-resolution provenance | Paired source, explicit missing consumer/branch |
| Particle lighting | W material mapping/render effect choice; E native variant and particle shader | S1 | Target-build light equation/normals and native pixels | Paired source, contrasting lit/unlit paths |
| General M2 material combiners | W `M2Shared.cpp:464–573,640–684,711–760`; E `m2_material.rs:10–99`, `m2.gdshader:167–239` | None established; V1–V3 retained for validation | Same-asset/batch/pass comparison and shader output | Partial source comparison; not parity |
| Bone hierarchy, skinning, attachments, variation and billboards | W `M2Model.cpp`; E attachment/bone code, `m2_variation.rs`, `m2_billboard.rs` | None established | End-to-end transform and lifetime fixtures; no general correctness claim | Partial source comparison |
| World movement/collision | W `WoW/Source/Object/Movement.cpp`; E native ground/player control; S shared movement | None established | Contact/slide/step-up, jump trajectories and water boundaries need matched inputs | Partial source comparison |
| WMO portal/frustum visibility | W `WoW/Source/WorldClient/MapObj.cpp`; E `wmo/portals.rs`, collision and format support | None established | Same camera/group/portal visibility fixture; no pass-equivalence proof | Reference mechanisms and existing current mechanisms inspected |
| Terrain compositing and liquids | Existing [terrain blend investigation](../wiki/investigations/terrain-blend-steps.md), [water investigation](../wiki/investigations/northshire-pale-water.md); reference world-render paths identified | None established | Direct paired shader/math and asset comparison incomplete | Coverage incomplete; no rework claim |
| Authoritative server gameplay | S shared movement/navmesh; W is client-only | None established | Server-side combat, persistence, pathing and authority are not recoverable from this source alone | No server defect established |

## Checked claims that must not become rework tasks

### Existing ribbon UVs do use the newest slot in the reference

An initial lead assumed atlas coordinates belonged to each edge at birth. `W/Engine/Source/Services/RibbonEmitter.cpp:349–354` rewrites surviving edge UVs using the current slot every update. `E/godot/core/src/m2_ribbons.rs:231–242` likewise derives them from the current trail slot. Storing each edge's birth slot would introduce a difference, not fix the proposed one.

### Ribbon interpolation uses current width for both frame endpoints

`W/Engine/Source/Services/RibbonEmitter.cpp:69–79` constructs previous/current endpoints with the same current `m_above`/`m_below`. `E/godot/core/src/m2_ribbons.rs:258–277` does the equivalent using current values plus the direction-difference bend. The assertion that reference interpolation necessarily retains previous-frame width is unsupported.

### Long-frame particle replay intentionally caps whole steps

`W/Engine/Source/Services/ParticleSystem2.cpp:1037–1063` computes remainder before capping whole steps to one lifespan. `E/godot/core/src/m2_particles.rs:528–540` does the same. For lifespan 1 s and delta 2.05 s, both cap the nominal whole-step replay to 1 s and retain the approximately 0.05 s remainder. This is not a demonstrated engine-only loss of elapsed time. Follow displacement distribution is a separate missing behavior, P2.

### A 45-degree slope is not rejected by a cos(50-degree) threshold

`W/WoW/Source/Object/Movement.cpp:2037–2039` uses normal Z `<= 0.64278764`, approximately cos(50 degrees). `S/crates/shared/src/movement.rs:3–4,77–84` uses a 50-degree maximum slope. A planar 45-degree slope has normal Z approximately 0.7071, above the reference rejection threshold: these snippets do not prove a 45-versus-50-degree policy discrepancy. Facet collision and height-delta validation remain different mechanisms requiring actual contact tests.

### A root-owned top-level ribbon mesh is not proof of a leak

`E/godot/rust/src/ribbons.rs:206–222` adds ribbon meshes as children before making transforms top-level; `:27–32` hides empty geometry. That representation alone does not prove surviving nodes after owner removal. Natural effect-tail teardown still warrants a lifecycle fixture, but no current ownership leak is established here.

## Game-server boundary

The reference README identifies a client decompilation. Client collision queries and packet consumption can reveal client expectations, not authoritative server policy. Different navmesh representations are not themselves defects. Do not change `game-server` slope rules, water policy, movement validation, protocols or persistence from these partial comparisons. A server finding would require an exact observable contract, traced producer/consumer paths and a discriminating test; none was established.

## Proof ledger

| Check | Revision/scope | Result | Invalidation rule |
|---|---|---|---|
| Baseline `git rev-parse HEAD` and `git status --short` for E/W/S | Revisions in index | Reference/server clean; engine no tracked changes, pre-existing untracked paths preserved | Relevant source or checkout modification requires rechecking affected citations |
| Paired read-only source inspection | File/line ranges in each entry | Nine conditional gaps; four separate candidates; rejected leads recorded above | A changed cited function invalidates its finding until re-audited |
| Asset-backed/native/original-binary checks | None | **Not run**; no rendered or gameplay reproduction claimed | Required before claiming a real affected asset or target-build parity |
| Builds/tests/deployment | None | **Not run**; documentation-only work | Not a code-fix or deployment completion claim |

Documentation verification checks source citations, internal links, coherent classifications and docs-only Git scope. It does not upgrade source evidence into runtime proof.
