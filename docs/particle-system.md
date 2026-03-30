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

**Entity model:** One Bevy entity per live particle, each with `Mesh3d` (unit quad), `MeshMaterial3d<StandardMaterial>`, `Transform`, and `Particle` component.

**Emitter entity:** `ParticleEmitterComp` with emitter data, bone link, emission accumulator. Parented to M2 model entity.

**Systems** (run in `GameState::InWorld`):
- `emit_particles` — accumulator-based emission, caps at 8/frame, resolves bone position
- `update_particles` — Euler integration (vel.y -= gravity*dt, pos += vel*dt), 3-point interpolation for color/opacity/scale, mutates `StandardMaterial` per particle per frame
- `billboard_particles` — `look_at(camera)` for each particle

**Material:** `StandardMaterial` unlit, double-sided, blend mode from emitter blend_type (Additive/Blend/Mask).

**Randomness:** `hash_float(seed, salt)` — deterministic hash from spawn position, not a real PRNG.

**Atlas:** Static random tile selected at spawn, not animated over lifetime.

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
