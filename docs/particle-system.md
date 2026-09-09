# Particle System

## Architecture

Two files: parser + renderer.

### Parser (`src/asset/m2_particle.rs`)

Parses M2 particle emitters from MD20 header at offset 0x128. Cata+ layout, 476-byte stride per emitter.

**Parsed fields:**
- Static: `flags`, `position`, `bone_index`, `texture_index`, `blend_type`, `emitter_type` (0=plane, 1=sphere, 2=spline), `tile_rows`, `tile_cols`
- M2Track (static first value only): `emission_speed`, `speed_variation`, `vertical_range`, `horizontal_range`, `gravity`, `lifespan`, `emission_rate`, `area_length`, `area_width`
- FakeAnimBlock: 3-point `colors` (RGB 0-255), `opacity` (0-1), `scales` (x,y pairs), `mid_point`
- Texture FDID resolved from TXID chunk

**Not parsed:** drag, wind, spin, tail_length, head/tail cell tracks, per-animation-sequence track data, most flags.

### Renderer (`src/particle.rs`)

**Stage registration:** Commit `beead231` registers the particle/Hanabi plugin only from cumulative `Particles` onward. `Empty` through `Lighting` do not register Hanabi or its render graph; `Particles`, `Ui`, and unconfigured normal runs retain the plugin. This is separate from the emitter systems' stage run condition and removes the render-side plugin work in strict Empty.

**Configured effects switch:** `particleEffectsEnabled` is persisted in `~/.config/world-of-osso/options_settings.ron`, defaulting to `true` for backward compatibility. At startup, `false` omits `ParticlePlugin`/Hanabi and deferred M2 emitter creation before texture loading. It also suppresses weather particle effects while retaining weather state, fog, and lighting. Animation and other non-particle world systems remain active. Existing config edits require restart. No CLI flag or graphics UI control was added. This is behavior coverage, not a CPU or native-visual result.

**Entity model:** One Bevy entity per live particle, each with `Mesh3d` (unit quad), `MeshMaterial3d<StandardMaterial>`, `Transform`, and `Particle` component.

**Emitter entity:** `ParticleEmitterComp` with emitter data, bone link, emission accumulator. Parented to M2 model entity.

**Systems** (run in `GameState::InWorld`):
- `emit_particles` — accumulator-based emission, caps at 8/frame, resolves bone position
- `update_particles` — Euler integration (vel.y -= gravity*dt, pos += vel*dt), 3-point interpolation for color/opacity/scale, mutates `StandardMaterial` per particle per frame
- `billboard_particles` — `look_at(camera)` for each particle

**Material:** `StandardMaterial` unlit, double-sided, blend mode from emitter blend_type (Additive/Blend/Mask).

**Randomness:** `hash_float(seed, salt)` — deterministic hash from spawn position, not a real PRNG.

**Atlas:** Static random tile selected at spawn, not animated over lifetime.

## Empty-stage performance evidence

The pre-registration strict-Empty profile for PID `2176863` sampled `bevy_hanabi::render::VfxSimulateNode::run` at **2.18% self CPU** despite particle emitter systems being stage-gated. Artifact: `/tmp/claude/game-engine-perf/character-camera-gate-2176863.perf-*.txt` (captured before `beead231`).

The post-fix PID `2297374` profile contained no Hanabi symbol. Three same-build passive 10-second process samples measured **12.50%**, **12.70%**, and **9.90%** of one core. The plugin boundary is proven, but the variable totals do not establish a causal CPU reduction or a stable `<=10%` Empty result.

## Known Limitations

1. **One entity per particle** — main performance bottleneck. Each particle = Entity + Mesh + Material + Transform + per-frame material mutation
2. **No emitter type dispatch** — plane/sphere/spline all spawn at point
3. **Area dimensions unused** — `area_length`/`area_width` parsed but ignored
4. **No drag or wind** physics
5. **No animated tracks** — only reads first static value from M2Tracks
6. **No tail/ribbon particles** — billboard head quads only
7. **No spin/rotation** on billboards
8. **Static texture atlas** — doesn't animate cell over lifetime
9. **No twinkle/LOD** culling
10. **Euler integration** — should be velocity Verlet
11. **Bone position stale** — particles don't follow animated bones well
12. **No world-space vs local-space** flag support
13. **No velocity inheritance** from bone movement

## Reference

`CParticleEmitter2.zip` in project root contains a decompiled/reimplemented C++ WoW client particle system. Use as behavioral reference (algorithms, flag meanings, physics model), not for direct porting. Key files:
- `CParticleEmitter2.hpp` — struct layout with all 0x2B8 bytes documented
- `CParticleEmitter2_Emission.cpp` — accumulator emission, burst mode, position interpolation
- `CParticleEmitter2_Physics.cpp` — velocity Verlet, wind, drag, track interpolation, simple optimization
- `CParticleEmitter2_Render.cpp` — billboard vertex building, tail quads, model particles
- `CRndSeed.hpp` — gnoise32 PRNG (61-entry lookup table)
- `ParticleTypes.hpp` — particle structs, flags, constants
- `CParticleBuffer.hpp` — pool with free-stack + active-list
