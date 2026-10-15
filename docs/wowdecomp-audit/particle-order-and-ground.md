# Particle ordering and world contact

[Scope and revision pins](README.md). These extend the original geometry/motion audit with two independent runtime behaviors. `W` is wowdecomp; `E` is game-engine. Source-supported omissions for the specified inputs; no native pixels or affected-asset census.

## P6: Authored per-particle depth sorting is ignored

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:7434–7436` maps authored flag `0x2` to emitter-internal `0x20`. `W/Engine/Source/Services/ParticleSystem2.cpp:2298–2320` (`BuildParticleVertices`) computes transformed depth for each live particle, pushes the records into a priority queue, then builds vertices in popped order when that flag is set. Without it, live-list order is used. The max-depth queue ordering is implemented at `:2084–2125`.

**Current.** `E/godot/core/src/m2_particles.rs:649–659` iterates the live particle vector and appends quads without a sort flag branch. Native `E/godot/rust/src/particles.rs:490–498` forwards that order to `ParticlePool::push`; `:263–272` writes instance records in received order. Sorting the pool as one transparent object does not reorder overlapping particles within the pool.

**Required change.** Honor the authored sorting flag using particle depth in the current view, and preserve the unsorted path for emitters that do not request it. Keep the decision independent of particle birth order and pool object bounds. This finding proves missing per-emitter sorting, not that the reference globally sorts all particles across every emitter or placement.

**Acceptance.** Put two stationary, overlapping half-alpha head particles in one alpha-blended emitter at different view depths, with different colors and insertion order opposite the reference's sorted order. With `0x2`, generated draw order must match reference depth order; reverse the camera so their depth order exchanges and verify it exchanges again. Without `0x2`, preserve live-list order. Compare native overlap pixels against that draw order; add an additive control where ordering must not spuriously change the effect's color contribution. Real-asset use of the flag remains to be enumerated.

## P7: Ground-projected particle births do not query world geometry

**Reference.** Authored `0x2000` maps to emitter-internal `0x40000` at `W/Engine/Source/Model2/M2Model.cpp:7372–7374`. Reachable plane and sphere birth paths call `ProjectParticle` when that bit is set (`W/Engine/Source/Services/ParticleSystem2.cpp:2929–2931,3000–3002`). `ProjectParticle` at `:470–490` queries height and positions the particle at that height plus the larger sampled scale component; query failure leaves its position unchanged. World setup registers the callback at `W/WoW/Source/WorldClient/World.cpp:1736–1737`, through `W/Engine/Source/Model2/M2Scene.cpp:898–902`. The actual callback at `W/WoW/Source/WorldClient/World.cpp:4623–4635` intersects a 20-yd downward segment against the map.

**Current.** `E/godot/core/src/m2_particles.rs:615–644` spawns and transforms particles without a ground-projection branch or surface query. Its native caller at `E/godot/rust/src/particles.rs:438–498` supplies transform, time, axis conversion and density only; it never supplies or performs a birth-position world query. Existing player/world collision support is not wired into these particle births.

**Required change.** For the authored world-space projection flag, project candidate birth positions against the intended map collision surfaces and apply the source-defined scale offset. Preserve unflagged/model-space behavior and the reference's no-hit behavior. Do not clamp every effect to terrain height: the callback is an actual downward intersection, not a map-wide scalar floor.

**Acceptance.** Use a world-space plane emitter above a flat collision surface at world height 0, zero velocity/gravity, sampled scale `(0.25,0.5)` and the projection bit. Birth height must be 0.5 in the corresponding world-up axis rather than the original emitter height. Place the emitter over a raised collision platform and a no-hit opening within the reference segment bounds; assert platform selection and unchanged position on no hit. Clear the flag and verify ordinary free births. Native geometric/pixel acceptance and modern surface-mask equivalence remain unperformed.
