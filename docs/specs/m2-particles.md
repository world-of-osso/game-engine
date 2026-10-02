# M2 particle emitters (Godot client)

M2 `ParticleSystem2` emitters on placed doodads (ADT MDDF and WMO MODD), creature display models and spell visual kit models. The shared parsing and simulation live in `godot/core` (`m2_particles`, the M2 parser in `src/asset/m2_format/m2_particle.rs`). Godot drawing lives in `godot/rust/src/particles.rs` and `godot/shaders/particle.gdshader`. Behaviour follows WebWowViewerCpp `managers/particles/particleEmitter.cpp` and its generators; pool sizing follows solarityclient `particle_system2`. See [godot-conversion](../wiki/systems/godot-conversion.md#native-m2-particles).

## What it must do

### Parsing
- [x] Parse every 0x1EC-stride emitter with its authored flags, bone, position, blend, emitter type, atlas, motion tracks (first value), lifetime ramps (colour, opacity, scale, head/tail cells) and multitexture layers. `instanceportal.m2` (197007) yields its six emitters with their authored values.
- [x] With an EXP2 chunk, take zSource, colorMult, alphaMult and the alpha-cutoff ramp from the emitter's EXP2 record; the portal's MD20 zSource 255 reads 0.

### Simulation
- [x] Spawn per WWV `CPlaneGenerator` (type 1) and `CSphereGenerator` (type 2), in the WWV random-stream order; other types are not simulated.
- [x] Accumulate emission at (rate + variation) × particle density, capped by the pool; `particleDensity` scales the rate except for flag 0x02000000. Each placement captures the current density at registration. An Options density change updates only the default: existing placements retain their captured rate and future registrations use the new default, without recreating pools.
- [x] Advance ballistically with drag, gravity and wind (while age < windTime). Model-space emitters (0x10) simulate in the emitter frame; others in world axes, with WoW gravity converted.
- [x] Replay long gaps in 0.1 s steps, at most one lifespan of them.
- [x] Retire a particle after its seed-varied lifespan; sample the lifetime ramps at age / maximum lifespan; apply seed-based size variation, random atlas cell (0x10000), twinkle, spin and inherited scale (0x20).
- [x] Size each emitter's pool to ceil((rate + variation) × (lifespan + variation) × 1.15), capped at 500.
- [x] Build head quads only (0x20000): camera-facing, emitter-plane (0x1000) or velocity-streak (0x4).

### Rendering
- [x] Map particle blend types 0-7 to the M2 GL factors (Opaque, AlphaKey, Alpha, NoAlphaAdd, Add, Mod, Mod2x, BlendAdd); only 0/1 write depth; alpha test -1 / 0.502 / 1/255.
- [x] Multiply colour into the texel in authored space; combine multitexture layers (Particle_Mod, 2Color_3Alpha, 3Color_3Alpha).
- [x] Draw one pooled `MultiMeshInstance3D` per (model FDID, emitter index), shared by every placement; pool capacity = sum of placement capacities capped at 4096.
- [x] Update and draw only the emitters of doodads that are drawn (scenery distance, WMO group portal cull) and whose box is in the view frustum; fade their particles with the doodad's scenery fade.
- [x] Creature display models draw their emitters (WebWowViewerCpp `animationManager.cpp` `calcParticleEmitters` runs for every M2 object), pooled per (model FDID, emitter) like doodads, updated while the unit is shown and its model box is in the view frustum, at the Options density captured when the visual attaches.
- [x] Item models units hold (creature virtual items and armor, player equipment) draw their emitters on their item nodes, idle while the item is hidden (unsheathed state); Warpweaver Hashom's staff (148608) has four.
- [x] Particles take the scene fog (`m2ParticleShader.frag.slang` `makeFog2` with the particle's blend mode: additive and modulating blends fog toward black/white/grey, `validateFogColor`).
- [x] Persisted startup `particleEffectsEnabled = false` spawns no particle pools or emitters. An authenticated actual-Azeroth portal fixture proves this state boundary; it does not prove pixels, audibility, live toggling, or density.
- [ ] The in-world portal matches retail framing (small white sparkles inside the blue sheet). Captured only, not compared by pixels.

## How it works

- [godot-conversion](../wiki/systems/godot-conversion.md#native-m2-particles)
- [Particle system](../particle-system.md)
- [stockade-entrance](../wiki/investigations/stockade-entrance.md)

## Implementation inventory

- `src/asset/m2_format/m2_particle.rs` — emitter, EXP2 and cell-key parsing (shared with the Bevy client).
- `src/asset/m2_particle_defaults.rs` — emitter defaults.
- `godot/core/src/m2.rs` — `Model::particle_emitters`.
- `godot/core/src/m2_particles.rs` — random stream, spawn, update, lifetime appearance, quads, pool sizing, blend depth/alpha-test rules.
- `godot/rust/src/particles.rs` — pools, materials, emitter frames from bones, `WowParticleProbe`.
- `godot/shaders/particle.gdshader` — particle fragment combiners and blend/fade output.
- `godot/rust/src/terrain/objects.rs` — per-doodad emitters and `update_particles`.
- `godot/rust/src/lib.rs` — graphics settings and the per-frame update.

## Tests asserting this spec

- `godot/core/tests/m2_particles.rs` — parsing of 197007, pool capacity, ramps, appearance, twinkle, lifespan, integration, steady state, long updates, world-space gravity, quad axes, blend depth/alpha test.
- `godot/tests/particle_blend_pixels.gd` — GPU pixels for blend 0-7, colour tint, fade, and half legacy fog for Alpha (toward the fog colour) and Add (toward black). RED/GREEN `data/diagnostics/worldvis-2026-10-01/particle-fog-{red,green}.log`.
- `godot/tests/world_particle_fog_flow.gd` — real client, Northshire: every doodad pool carries the terrain's fog uniforms (`pfog.log`, 207 pools).
- `godot/tests/world_unit_particles_flow.gd` — real client, Stormwind (-8708, 845): 9 creatures (Ethereals 123799/123791 and others) place 114 emitters in 46 pools, 529 particles drawn facing them (`unitp-sw.log`, `unitp-sw/*.png`).
- `godot/tests/wmo_doodads_flow.gd` — portal pools empty while Jail01 is culled, all six drawing at the trigger.
- `godot/tests/world_portal_particles_flow.gd` via `native_input_fixture portal-particles-{disabled,enabled}` — persisted startup setting in an authenticated Azeroth 30_48 GameClient, with actual placed `sw_magicdistrict` MODD 1112 portal meshes in both modes. At `d23012b4` + `e5671528`, disabled has no particle pools/emitter state; enabled has all six pools with visible-instance counts 42, 21, 21, 21, 21, and 2 (143 total; scene totals 711 pools/emitters). Depot fixture/build and both runtime logs exit 0 at `data/diagnostics/portal-particles-{depot-build,disabled,enabled}-e5671528.log`. This headless state proof excludes pixels, audibility, live toggling, and density.
- `godot/tests/world_portal_density_flow.gd` via `native_input_fixture portal-density` — controlled disposable copy of portal 197007 with only the six `NO_GLOBAL_SCALE` bits cleared; the original asset hash remains unchanged. It samples known pool visible quads before/after real `GameClient` Options slider 100→10 and after owned same-map `NewWorld`. RED at `cf84a712` (`data/diagnostics/portal-density-red-cf84a712.log`, exit 101): existing placement 484→484.22, fresh placement 462.67 instead of below 30% of baseline. At `79d792b0`, Depot `kdhjvgmnt3` builds the fixture (exit 0; `data/diagnostics/portal-density-depot-green-build-retry1.log`) and runtime GREEN exits 0 (`data/diagnostics/portal-density-green-79d792b0.log`): existing 485.67→488.0 while the fresh placement is 46.67. This proves controlled density-sensitive state behavior, not retail pixels, audible output, or full parity.

## Known gaps (current cycle)

- [ ] Animated emitter tracks: `set_animation` keys emission rate, speed and `enabledIn` per sequence, but skips global-sequence tracks and tracks with at most one key per sequence; the other tracks use their first key.
- [ ] Tail quads (0x40000), spline emitters (type 3), model particles, follow-position (0x4000), burst/inherit velocity (0x40), randomized atlas mask (0x8000), ground snap (0x2000), TXAC shader variants, alpha-cutoff discard and lit particles (no 0x1 flag).
- [ ] Emitters of attached item models (weapons, held torches, enchants) and player models; display state spell kits (the HD kobold candle flame is not a model emitter).
- [ ] Particles within a pool are not depth-sorted; a shared pool sorts as one transparent object.

## Out of scope

- The Bevy client's Hanabi particle path (abandoned client).
- Ribbon emitters (separate feature).
