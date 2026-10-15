# Particle geometry, motion and shading

[Scope and pinned revisions](README.md). `W` = wowdecomp; `E` = game-engine. These entries establish absent semantics for specified inputs, not their frequency in shipped assets. The [particle spec's known gaps](../specs/m2-particles.md#known-gaps-current-cycle) already names most of these capabilities; source tracing below distinguishes current omissions from stale inventory. No server changes are implied.

## P1: Particle tails are not drawn

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:7389` passes authored head `0x20000`, tail `0x40000` and tail length independently to `SetParticleStyle`. `W/Engine/Source/Services/ParticleSystem2.cpp:296–314` stores the independent styles. `:1683–1733` generates tail geometry opposite velocity, samples the tail atlas cell and optionally clamps tail length by particle age.

**Current.** `E/godot/core/src/m2_particles.rs:648–655` returns immediately without `FLAG_HEAD`; all output is head quads. Parsed tail length exists at `E/godot/core/src/asset/m2_format/m2_particle.rs:78–79`, but this native simulation/render path never uses it. A velocity-oriented head is not equivalent to an independently authored tail.

**Required change.** Generate tails when the tail flag is present, independently of heads. Apply authored length, age-clamping policy and tail-cell animation. Preserve head-only output; head-and-tail must show both.

**Acceptance.** A non-refractive type-1 fixture, tail bit only, nonzero velocity, length 0.5 and a distinct tail atlas cell produces a visible tail instead of zero quads. Head-only, tail-only and combined fixtures must have independently observable geometry. Check the short-age clamp and near-zero projected velocity case against `W/:1692–1722`, then compare native pixels on an applicable real asset.

## P2: Follow-position is ignored

**Reference.** Authored `0x4000` maps to emitter-internal `0x80000` at `W/Engine/Source/Model2/M2Model.cpp:7377–7379`; do not confuse these two flag namespaces. Follow control points are passed at `:7486`. `W/Engine/Source/Services/ParticleSystem2.cpp:342–354` derives a speed-dependent linear factor; `:2480–2503` clamps it to [0,1] and multiplies emitter displacement. `:582–586` adds the resulting per-step burst offset to sufficiently old particles; `:1037–1063` distributes it over replay steps.

**Current.** Four follow values are parsed at `E/godot/core/src/asset/m2_format/m2_particle.rs:544–554`. `E/godot/core/src/m2_particles.rs:516–540,549–594` stores the current emitter frame and performs ordinary ballistic movement, but has no previous-frame follow displacement or speed-to-follow application. Neither these fields nor a follow-position branch is consumed in native `particles.rs`.

**Required change.** Apply authored speed-dependent emitter displacement to eligible surviving particles for the follow flag, with reference-equivalent age eligibility and replay distribution. Keep ordinary world-space particles stationary relative to their ballistic motion; do not turn the entire pool into model-space simulation or simply interpolate all spawn transforms.

**Acceptance.** Disable further emission after creating a particle aged 0.5 s. Over a 0.1 s update, move an otherwise force-free emitter 1 yd. Configure follow points `(speed 0,scale 0)` and `(speed 10,scale 1)`: the eligible particle receives 1 yd of carry. With the flag cleared it receives none. Check a particle too young for the reference age condition, speed clamping, and a multi-substep update. Exact modern asset applicability remains to be established.

## P3: Emitter velocity is not inherited

**Reference.** Authored `0x40` maps to internal `0x800` at `W/Engine/Source/Model2/M2Model.cpp:7342–7344`. `W/Engine/Source/Services/ParticleSystem2.cpp:2508–2528` updates parent velocity using its timer and `m_ivelScale`. New particle velocity includes that parent velocity at `:521–523`; the model supplies the authored scale at `W/Engine/Source/Model2/M2Model.cpp:7469`.

**Current.** `E/godot/core/src/m2_particles.rs:615–644` transforms generator velocity into world axes, but never adds a measured parent velocity or uses the inherited-velocity flag. `E/godot/core/src/asset/m2_format/m2_particle.rs:107–111` exposes `inherit_velocity_scale` while explicitly noting the persisted modern header field has not been established. Presence of that field is not runtime support.

**Required change.** Implement the flag's birth-velocity contribution with the correct displacement/time convention and scale for the target format. Resolve the modern scale's data provenance before claiming parity; do not blindly equate the reference's timer-normalized quantity with ordinary yd/s. Keep this distinct from P2: inherited velocity changes subsequent trajectory, while follow-position carries already-live particles.

**Acceptance.** For identical seeded generator output, compare stationary and moving-parent births with `0x40` on/off. The on/moving fixture must contain exactly the reference-derived parent contribution and retain its changed trajectory after parent motion stops. Test the reference timer boundary and no-live-particle reset. Asset decoding of the modern scale is an explicit dependency, not a reason to invent a constant.

## P4: Spline emitters are filtered out

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:7276–7284` creates `CSplineParticleEmitter` for type 3 and `:7489–7491` supplies its spline. `W/Engine/Source/Services/ParticleSystem2.cpp:2861–2873,3059–3130` contains spline setup and particle creation rather than treating type 3 as a sphere.

**Current.** `E/godot/core/src/m2_particles.rs:119–134` recognizes only plane and sphere. `E/godot/rust/src/particles.rs:315–326` filters out all other shapes before placement; `EmitterSim::update` also rejects them at `E/godot/core/src/m2_particles.rs:525–526`. The parser identifies type 3 as spline (`E/godot/core/src/asset/m2_format/m2_particle.rs:55–56`), but naming the type does not simulate it.

**Required change.** Decode and preserve the spline controls for the applicable asset layout, then generate positions/velocities and emission density along the authored curve. Do not approximate unsupported splines as plane/sphere emitters.

**Acceptance.** An emitter with a known bent three-control-point curve, positive rate and no refraction reaches native placement, emits along the curve and changes when the curve changes. Check width/height and rate-per-unit behavior against the reference. Establish real asset and spline-layout provenance before treating this as a shipped-asset fix.

## P5: Model particles and recursive emitters have no native path

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:7494–7500` calls `SetModel` for the geometry model and `CreateChildEmittersFromModel` for a recursive-emitter model. `W/Engine/Source/Services/ParticleSystem2.cpp:540–568` handles angular state and per-particle model creation; its ordinary render-buffer path excludes model particles at `:2451–2453`.

**Current.** `E/godot/core/src/asset/m2_format/m2_particle.rs:47–50` retains both filenames, but the native `E/godot/rust/src/particles.rs` has no consumer of either. `E/godot/core/src/m2_particles.rs:615–644,648–660` produces particle state and billboard quads only. An admitted plane/sphere emitter with a model reference has no corresponding per-particle model/recursive-model instantiation path.

**Required change.** Render authored particle geometry with its lifetime transform/orientation and drive referenced child emitters through the appropriate lifecycle. Keep these separate from normal model bone attachments. Asset resolution must respect the current product-isolated asset rules rather than assuming legacy MPQ filenames resolve in Retail CASC.

**Acceptance.** Use a uniquely identifiable geometry model and a distinct child-emitter model. Verify actual model instances and child emissions, not replacement colored quads; confirm natural retirement and owner teardown remove both. Asset availability, recursive-emitter timing and modern filename/FDID resolution are unresolved prerequisites; no current leak is claimed.

## S1: Lit particles are rendered unshaded

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:7431–7432` maps authored particle flag `0x1` to a lit material. `W/Engine/Source/Model2/CM2SceneRender.cpp:733–739` chooses the lit versus unlit particle effect and `:747–749` chooses normal-bearing `GxVBF_PNCT` versus `GxVBF_PCT` vertices from that material flag. `SetupLighting` is called in the render path.

**Current.** `E/godot/shaders/particle.gdshader:1–3,48–97` always renders unshaded texture/color plus fog. The variant binder preserves `unshaded` for every blend type (`E/godot/rust/src/particles.rs:56–64`); there is no authored-lit branch. Fog alone is not lighting.

**Required change.** Honor the authored lit/unlit distinction with target-appropriate particle normals and lighting. Preserve authored unlit effects and their existing blend/fog behavior. Do not globally brighten particles or enable generic PBR as a substitute for matching the effect's light response.

**Acceptance.** Place otherwise identical lit-flag and unlit-flag emitters in a fixed scene, disable fog and vary incident light while holding textures, age and camera constant. Lit particles must respond according to the reference lighting path; unlit particles must remain invariant. Compare captured pixels and ensure additive/modulating blends remain correct. Exact Retail lighting coefficients require a version-appropriate oracle.
