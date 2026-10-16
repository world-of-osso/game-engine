# M2 particle emitters (Godot client)

M2 `ParticleSystem2` emitters on placed doodads (ADT MDDF and WMO MODD), creature display models and spell visual kit models. The shared parsing and simulation live in `godot/core` (`m2_particles`, the M2 parser in `godot/core/src/asset/m2_format/m2_particle.rs`). Godot drawing lives in `godot/rust/src/particles.rs` and `godot/shaders/particle.gdshader`. Behaviour follows WebWowViewerCpp `managers/particles/particleEmitter.cpp` and its generators; pool sizing follows solarityclient `particle_system2`. See [godot-conversion](../wiki/systems/godot-conversion.md#native-m2-particles).

## What it must do

### Parsing
- [x] Parse every 0x1EC-stride emitter with its authored flags, bone, position, blend, emitter type, atlas, motion tracks (first value), lifetime ramps (colour, opacity, scale, head/tail cells) and multitexture layers. `instanceportal.m2` (197007) yields its six emitters with their authored values.
- [x] With an EXP2 chunk, take zSource, colorMult, alphaMult and the alpha-cutoff ramp from the emitter's EXP2 record; the portal's MD20 zSource 255 reads 0.

### Simulation
- [x] Spawn per WWV plane (1), sphere (2) and cubic Bézier spline (3) generators; spline controls must form complete 3n+1 finite points. Types beyond these are not simulated.
- [x] Accumulate emission at (rate + variation) × particle density, capped by the pool; `particleDensity` scales the rate except for flag 0x02000000. Each placement captures the current density at registration. An Options density change updates only the default: existing placements retain their captured rate and future registrations use the new default, without recreating pools.
- [x] Advance ballistically with drag, gravity and wind (while age < windTime). Model-space emitters (0x10) simulate in the emitter frame; others in world axes, with WoW gravity converted.
- [x] Replay long gaps in 0.1 s steps, at most one lifespan of them.
- [x] Retire a particle after its seed-varied lifespan; sample the lifetime ramps at age / maximum lifespan; apply seed-based size variation, random atlas cell (0x10000), twinkle, spin and inherited scale (0x20).
- [x] Size each emitter's pool to ceil((rate + variation) × (lifespan + variation) × 1.15), capped at 500.
- [x] Build independent head (0x20000) and tail (0x40000) quads. Heads are camera-facing, emitter-plane (0x1000) or velocity-streak (0x4). Tails extend opposite velocity for tailLength seconds, use tail atlas keys, and clamp duration to age for 0x400. Near-view-parallel tails use the finite tiny-quad policy from WebWowViewerCpp buildVertex2.

### Rendering
- [x] Map particle blend types 0-7 to the M2 GL factors (Opaque, AlphaKey, Alpha, NoAlphaAdd, Add, Mod, Mod2x, BlendAdd); only 0/1 write depth; alpha test -1 / 0.502 / 1/255.
- [x] Multiply colour into the texel in authored space; combine multitexture layers (Particle_Mod, 2Color_3Alpha, 3Color_3Alpha).
- [x] Select Refraction for non-multitexture flag0x100000 and omit those emitters from the forward pass while no refraction pass exists, matching WebWowViewerCpp; never draw displacement textures as ordinary colour. Multitexture classification takes priority; AW's colour/UV emitters remain drawable.
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

- `godot/core/src/asset/m2_format/m2_particle.rs` — emitter, EXP2 and cell-key parsing (shared with the Bevy client).
- `godot/core/src/asset/m2_particle_defaults.rs` — emitter defaults.
- `godot/core/src/m2.rs` — `Model::particle_emitters`.
- `godot/core/src/m2_particles.rs` — random stream, spawn, update, lifetime appearance, quads, pool sizing, blend depth/alpha-test rules.
- `godot/rust/src/particles.rs` — pools, materials, emitter frames from bones, `WowParticleProbe`.
- `godot/shaders/particle.gdshader` — particle fragment combiners and blend/fade output.
- `godot/rust/src/terrain/objects.rs` — per-doodad emitters and `update_particles`.
- `godot/rust/src/lib.rs` — graphics settings and the per-frame update.

## Tests asserting this spec

- `godot/core/tests/m2_particles.rs` — parsing of 197007, pool capacity, ramps, appearance, twinkle, lifespan, integration, steady state, long updates, world-space gravity, quad axes, blend depth/alpha test; real1127086 refraction selection and AW colour/UV preservation.
- `godot/rust/src/particles.rs::tests` — real1127086 admits exactly0..4,7..9 and a refraction-only model creates no forward particles. Matched both-skin process/pixel proof: [rainbow investigation](../wiki/investigations/avenging-wrath-material-time.md#actual-source-capture-and-root-cause).
- `godot/tests/particle_blend_pixels.gd` — GPU pixels for blend 0-7, colour tint, fade, and half legacy fog for Alpha (toward the fog colour) and Add (toward black). RED/GREEN `data/diagnostics/worldvis-2026-10-01/particle-fog-{red,green}.log`.
- `godot/tests/world_particle_fog_flow.gd` — real client, Northshire: every doodad pool carries the terrain's fog uniforms (`pfog.log`, 207 pools).
- `godot/tests/world_unit_particles_flow.gd` — real client, Stormwind (-8708, 845): 9 creatures (Ethereals 123799/123791 and others) place 114 emitters in 46 pools, 529 particles drawn facing them (`unitp-sw.log`, `unitp-sw/*.png`).
- `godot/tests/wmo_doodads_flow.gd` — portal pools empty while Jail01 is culled, all six drawing at the trigger.
- `godot/tests/world_portal_particles_flow.gd` via `native_input_fixture portal-particles-{disabled,enabled}` — persisted startup setting in an authenticated Azeroth 30_48 GameClient, with actual placed `sw_magicdistrict` MODD 1112 portal meshes in both modes. At `d23012b4` + `e5671528`, disabled has no particle pools/emitter state; enabled has all six pools with visible-instance counts 42, 21, 21, 21, 21, and 2 (143 total; scene totals 711 pools/emitters). Depot fixture/build and both runtime logs exit 0 at `data/diagnostics/portal-particles-{depot-build,disabled,enabled}-e5671528.log`. This headless state proof excludes pixels, audibility, live toggling, and density.
- `godot/tests/world_portal_density_flow.gd` via `native_input_fixture portal-density` — controlled disposable copy of portal 197007 with only the six `NO_GLOBAL_SCALE` bits cleared; the original asset hash remains unchanged. It samples known pool visible quads before/after real `GameClient` Options slider 100→10 and after owned same-map `NewWorld`. RED at `cf84a712` (`data/diagnostics/portal-density-red-cf84a712.log`, exit 101): existing placement 484→484.22, fresh placement 462.67 instead of below 30% of baseline. At `79d792b0`, Depot `kdhjvgmnt3` builds the fixture (exit 0; `data/diagnostics/portal-density-depot-green-build-retry1.log`) and runtime GREEN exits 0 (`data/diagnostics/portal-density-green-79d792b0.log`): existing 485.67→488.0 while the fresh placement is 46.67. This proves controlled density-sensitive state behavior, not retail pixels, audible output, or full parity.

## Known gaps (current cycle)

- [ ] Animated emitter tracks: `set_animation` keys emission rate, speed and `enabledIn` per sequence, but skips global-sequence tracks and tracks with at most one key per sequence; the other tracks use their first key.
- [ ] Clean-room unresolved dependencies: **P3-timer-reset** (birth-velocity timer/reset), **P4-density** (spline rate per unit), **S1-polarity-oracle** (target lit/unlit polarity and coefficients), **P5-product-resolution / P5-recursive-timing** (real model/child assets and clocks). Details below; none are claimed as implemented. Randomized atlas mask (0x8000), ground snap (0x2000), TXAC UV variants, refraction pass and alpha-cutoff discard also remain open.
- [ ] Emitters of attached item models (weapons, held torches, enchants) and player models; display state spell kits (the HD kobold candle flame is not a model emitter).
- [x] Authored0x2 sorts live particles far-to-near within one emitter/placement in the current camera, before expanding each head/tail pair; unflagged emitters retain live-list order. This does not claim global sorting across placements/emitters.

## Out of scope

- The Bevy client's Hanabi particle path (abandoned client).
- Ribbon emitters (separate feature).

## Clean-room evidence (2026-10-15)

### P1 — tails

Behaviour provenance: user-supplied particle clean-room spec and WebWowViewerCpp `particleEmitter.cpp::buildVertex2` (duration, 0x400 age clamp, projected squared threshold 0.0001 and tiny scale 0.05). No decompiled sources consulted. Native pools reserve both styles rather than dropping half of combined output. Targeted `particle_cleanroom_tail` fixtures assert independent geometry/cells and age/degeneracy. RED: two tests failed because tails produced zero quads; GREEN 2/2 at1ce73e656. Native real-asset pixels PASS at `edc9c82be` with fixture `33cb46507`: five real165894 tail-only quads change4,547 pixels; two frozen baseline frames are byte-identical. PNGs at `/syncthing/AgentShared/2026-10-15/m2-particles/p1-bloodpresence-tails-{on,stable,off}.png` were inspected (on/off and amplified difference). This proves native authored geometry visibility, not target-client visual parity. Tail width remains independent of the optional velocity-oriented head: view-parallel velocity reproduced a zero-width tail (RED); scale is now taken directly from appearance rather than inferred from head axes.

P1 native fixture: `godot/tests/particle_cleanroom_tails.gd` opens the real particledebug screen at1920×1080, cycles to actual Blood Presence165894 (tail-only flags0x40422), freezes simulation and compares native pool visible/hidden pixels. Original three-model screen RED cannot reach the asset; the debug catalog now includes it using its authored emitter height and existing three-yard camera distance. GREEN: five quads,4,547 changed pixels, frozen baseline byte-identical; 1920×1080 PNGs saved and inspected.

Native capture calibration: the first real165894 run drew5 tail quads but the inherited0.08 brightness threshold excluded every changed pixel (observed peak14/255). The fixture now freezes bone/material descendants as well as emission, requires two byte-identical baseline PNGs, and counts actual changed 8-bit pixels rather than imposing an unrelated brightness cutoff. This is a fixture correction, not particle brightening.


### Clean-room P2 (2026-10-15)

Follow-position 0x4000 applies clamped authored speed/scale interpolation to emitter displacement, only when the already-advanced particle age exceeds twice the step. Replay slices receive their proportional displacement, not the entire frame displacement repeatedly. Provenance: supplied clean-room spec and allowed WebWowViewerCpp Update/UpdateParticle. Duplicate speed points produce zero carry as in WWV. Modern asset applicability remains unproven; combined model-space/follow semantics require target observation. Tests isolate old/young, flag off, clamped speed and multi-step carry. RED 0/2; GREEN 2/2 at05de65324.

### Clean-room P4 (2026-10-15)

Layout established from allowed WWV `M2ParticleOld::splinePoints` and real local MD21 6316199: emitter 0 has M2Array count4/MD20-offset264608 at 0x1C0/0x1C4, float3 payload [(0,0,0),(0,0.002777778,0),(0,0.005555556,0),(0,0,0)]. Parser preserves controls; the native admission predicate now admits type3 only with complete cubic controls. WWV C3Spline supplies four-point Bézier evaluation, twenty-interval segment-length weighting, and CSplineGenerator supplies clamped width/length parameter bounds, endpoint-on-width-change, tangent-axis launch and zSource rules. Synthetic bent controls are substituted only in a disposable in-memory real asset. RED: no particles/admission; GREEN 2/2 at23823f1c0. **Named gap P4-density:** allowed WWV uses ordinary rate; the supplied spec does not state the rate-per-unit convention. No density multiplier invented; exact target per-length semantics and native bent-spline pixels remain open.

### Clean-room P3 provenance and runtime gap (2026-10-15)

The modern scale is established: allowed WWV `M2ParticleOld::BurstMultiplier` is labelled ivelScale, and `Update` multiplies it into the inherited vector. The matching local stride places it at 0x170. Real2994778 emitter3 has flag0x40 and raw3.0; parser now retains3.0 in `inherit_velocity_scale`, rather than an invented default1.0. RED real-asset decode1≠3; GREEN1/1 atd7e7f2ae7. **Named gap P3-timer-reset:** WWV uses a >30ms timer and 30ms/timer displacement scale, but computes its vector only when the previous pool is empty and clears it when live. The supplied acceptance instead calls for a no-live reset; it does not supply the numerical timer convention or reset predicate. No ordinary yd/s or guessed inverse predicate is substituted. Runtime birth contribution, timer-boundary, continuing-trajectory and no-live-reset acceptance remain unimplemented pending allowed target evidence.

### Clean-room S1 blocked polarity oracle (2026-10-15)

**Named gap S1-polarity-oracle:** allowed WWV assigns material bit0 to NOT authored bit0 for ordinary particles, clears it for multitexture/refraction, and initializes material flags separately. This corroborates the supplied inversion ambiguity, not target lighting. Local torch145304 emitter0 has bit0 clear; portal197007 has both bit0 states, but asset flags alone do not establish whether their intended particle effect is lit. Pinned wow.export c2fd7bd M2Loader skips particle payloads and supplies no oracle. No incident-light observation of the target client is available in this checkout. Therefore shader selection, normals, lighting coefficients and fixed-light pixel acceptance remain blocked; existing unshaded blend/fog behavior stays unchanged. No generic PBR or invented flag polarity is substituted.

### Clean-room P5 blocked asset/lifecycle provenance (2026-10-15)

**Named gap P5-product-resolution:** bounded local-cache census (83,123 parseable unqualified M2 files; not a current-build publication audit) found no nonempty geometry/recursion filenames after stripping NUL terminators. Nonzero M2Array counts frequently describe a single NUL, not a referenced asset. Allowed WWV retains the two legacy filenames but has no implemented per-particle geometry/child-emitter consumer establishing Retail product-qualified FDID resolution. Pinned wow.export M2Loader skips particle emitters. Legacy listfile-name guessing is not a valid substitute for [product-isolated model assets](product-isolated-model-assets.md).

**Named gap P5-recursive-timing:** supplied spec identifies lifetime ownership/orientation and retirement requirements, but not the child-emission clock, loop/recursion termination policy, or target asset resolution. No authentic uniquely identifiable geometry/child fixture can presently exercise those boundaries. Actual model-instance, child-emission, natural-retirement and owner-teardown acceptance therefore remain open; no colored-quads substitute, synthetic asset receipt, or attachment-path reuse was added.

### Integrated proof

After merging origin/master, `edc9c82be` passes all eight `particle_cleanroom_` controls (no full suite), native helper build exits0 without compiler warnings, and Godot-workspace `cargo fmt --all --check` exits0. Logs: `/home/osso/.worktrees/logs/m2particles-{integrated-green,native-green-build,native-stable,fmt-check}.log`. Later changes are fixture/docs only. Helper has no cargo-check mode; native build supplies compile proof rather than claiming an unrun check. S1/P5 and the named partial P3/P4 boundaries remain open.

### P6 — authored depth ordering (2026-10-15)

Supplied clean-room P6 establishes authored0x2, transformed particle depth, maximum-depth-first order, and unchanged live-list order without the bit. `EmitterSim::quads` now sorts only flagged live particles using their world center projected onto the current view back axis (ascending dot product = far first); model-space centers are transformed before comparison. Head/tail pairs stay together. Native `ParticlePool` consumes that sequence without a second spatial sort. Actual Blood Presence165894 emitter0 has flag0x40422, including0x2; this is one observed asset, not a full census.

RED: both new CPU controls fail because near-born-first remains first. Tests cover forward/reversed camera, flag-off insertion order, and paired geometry. `particle_cleanroom_sort.gd` drives the shared simulator and native pool through two stationary red/blue half-alpha heads over black, with sort/camera toggles and additive controls, capturing1920×1080 PNGs. GREEN/native overlap proof pending.
