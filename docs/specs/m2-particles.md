# M2 particle emitters (Godot client)

M2 `ParticleSystem2` emitters on placed doodads (ADT MDDF and WMO MODD), creature display models and spell visual kit models. The shared parsing and simulation live in `godot/core` (`m2_particles`, the M2 parser in `godot/core/src/asset/m2_format/m2_particle.rs`). Godot drawing lives in `godot/rust/src/particles.rs`, `godot/rust/src/particle_composite.rs` and the compositor effect `godot/rendering/particle_composite_effect.gd`. Behaviour follows WebWowViewerCpp `managers/particles/particleEmitter.cpp` and its generators; pool sizing follows solarityclient `particle_system2`. See [godot-conversion](../wiki/systems/godot-conversion.md#native-m2-particles).

## What it must do

### Parsing
- [x] Parse every 0x1EC-stride emitter with its authored flags, bone, position, blend, emitter type, atlas, motion tracks (first value), lifetime ramps (colour, opacity, scale, head/tail cells) and multitexture layers. `instanceportal.m2` (197007) yields its six emitters with their authored values.
- [x] With an EXP2 chunk, take zSource, colorMult, alphaMult and the alpha-cutoff ramp from the emitter's EXP2 record; the portal's MD20 zSource 255 reads 0.

### Simulation
- [x] World-space plane/sphere0x2000 births intersect the existing native map collision space with a20-yd downward segment and offset the hit by the larger sampled scale component; misses and unflagged/model-space births remain unchanged. Exact target surface-mask equivalence remains open.
- [x] Spawn per WWV plane (1), sphere (2) and cubic Bézier spline (3) generators; spline controls must form complete 3n+1 finite points. Types beyond these are not simulated.
- [x] Accumulate emission at (rate + variation) × particle density, capped by the pool; `particleDensity` scales the rate except for flag 0x02000000. Each placement captures the current density at registration. An Options density change updates only the default: existing placements retain their captured rate and future registrations use the new default, without recreating pools.
- [x] Advance ballistically with drag, gravity and wind (while age < windTime). Model-space emitters (0x10) simulate in the emitter frame; others in world axes, with WoW gravity converted.
- [x] Replay long gaps in 0.1 s steps, bounded by one authored lifespan for ordinary emitters; a P5 tree adds its longest descendant lifespan path to that history budget.
- [x] Explicitly resolved recursive definitions receive births from each surviving parent's moved position, eligible parent-particle velocity and previous position for interpolation. Retire parents at their lifespan before child births; advance existing children once per positive substep, without root-origin births. Descendants drain independently; zero-duration updates do not age or emit.
- [x] Retire a particle after its seed-varied lifespan; sample the lifetime ramps at age / maximum lifespan; apply seed-based size variation, random atlas cell (0x10000), twinkle, spin and inherited scale (0x20).
- [x] Size each emitter's pool to ceil((rate + variation) × (lifespan + variation) × 1.15), capped at 500.
- [x] Build independent head (0x20000) and tail (0x40000) quads. Heads are camera-facing, emitter-plane (0x1000) or velocity-streak (0x4). Tails extend opposite velocity for tailLength seconds, use tail atlas keys, and clamp duration to age for 0x400. Near-view-parallel tails use the finite tiny-quad policy from WebWowViewerCpp buildVertex2.

### Rendering
- [x] Explicitly resolved geometry resources draw actual model mesh batches with per-particle lifetime position, birth orientation/spin, scale, RGB and opacity, separately from recursive emitter definitions. Geometry references never substitute billboard quads. This consumer is proved synthetically; Retail reference resolution remains a named gap.
- [x] Authored0x2 sorts live particles far-to-near within each emitter/placement in the current camera before expanding head/tail pairs; unflagged emitters retain live order. No cross-emitter/placement sort is claimed.
- [x] Map particle blend types 0-7 to the M2 GL factors (Opaque, AlphaKey, Alpha, NoAlphaAdd, Add, Mod, Mod2x, BlendAdd); alpha test -1 / 0.502 / 1/255.
- [x] Blend in authored (gamma) space, as WebWowViewerCpp writes particles unconverted into an R8G8B8A8_UNORM target (`GFrameBufferVLK.cpp:21`): after Godot's transparent pass and MSAA resolve the scene colour is sRGB-encoded in place (extended curve, HDR round-trips), the pools draw far to near with the GL factors on authored values, depth tested against the resolved scene depth, and the colour is decoded back to linear.
- [ ] Blend 0/1 particles write depth. **Named gap PARTICLES_NO_DEPTH_WRITE:** the composite samples the resolved depth and attaches none, so opaque/alpha-key particles occlude scene geometry correctly but not each other.
- [ ] Particles sort among other transparent geometry. **Named gap PARTICLES_COMPOSITE_AFTER_ALL_TRANSPARENT_GEOMETRY:** pools draw after all transparent geometry.
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
- `godot/rust/src/particles.rs` — pools, explicit model-reference resources, emitter frames from bones, `WowParticleProbe`.
- `godot/rust/src/particle_models.rs` — geometry mesh/material resources, instance lifetime RGBA and placement-owned geometry nodes.
- `godot/rust/src/particle_models_probe.rs` — synthetic model/child acceptance through native placement and uploads.
- `godot/rust/src/particle_composite.rs` — per-frame publication of visible pools and the scene fog block to one compositor effect per camera.
- `godot/rendering/particle_composite_effect.gd`, `godot/shaders/particle_composite.glsl.in` (combiners, blend/fade output, `retail_fog.gdshaderinc` inserted), `godot/shaders/particle_gamma.glsl` (encode/decode).
- `godot/rust/src/terrain/objects.rs` — per-doodad emitters and `update_particles`.
- `godot/rust/src/lib.rs` — graphics settings and the per-frame update.

## Tests asserting this spec

- `godot/core/tests/m2_particles.rs` — parsing of 197007, pool capacity, ramps, appearance, twinkle, lifespan, integration, steady state, long updates, world-space gravity, quad axes, blend depth/alpha test; real1127086 refraction selection and AW colour/UV preservation.
- `godot/rust/src/particles.rs::tests` — real1127086 admits exactly0..4,7..9 and a refraction-only model creates no forward particles. Matched both-skin process/pixel proof: [rainbow investigation](../wiki/investigations/avenging-wrath-material-time.md#actual-source-capture-and-root-cause).
- `godot/tests/particle_blend_pixels.gd` — GPU pixels for blend 0-7, colour tint, fade, and half legacy fog for Alpha (toward the fog colour) and Add (toward black), as raw equations on authored 8-bit values. RED/GREEN `data/diagnostics/worldvis-2026-10-01/particle-fog-{red,green}.log`; gamma RED (12 of 16 fail on linear blending) and GREEN (16/16 within 2/255) `data/diagnostics/torch-particles-2026-10-20/gamma-{red,green}.log`. Native 1920x1080 captures of 145304, 199823 (MODD32) and Frostbolt 1598570 in both skins, with composite cost: [godot-torch-rendering](../wiki/investigations/godot-torch-rendering.md#gamma-compositing-2026-10-20).
- `godot/tests/world_particle_fog_flow.gd` — real client, Northshire: every doodad pool carries the terrain's fog uniforms (`pfog.log`, 207 pools).
- `godot/tests/world_unit_particles_flow.gd` — real client, Stormwind (-8708, 845): 9 creatures (Ethereals 123799/123791 and others) place 114 emitters in 46 pools, 529 particles drawn facing them (`unitp-sw.log`, `unitp-sw/*.png`).
- `godot/tests/wmo_doodads_flow.gd` — portal pools empty while Jail01 is culled, all five forward pools drawing at the trigger, no pool for refraction emitter 5.
- `godot/tests/world_portal_particles_flow.gd` via `native_input_fixture portal-particles-{disabled,enabled}` — persisted startup setting in an authenticated Azeroth 30_48 GameClient, with actual placed `sw_magicdistrict` MODD 1112 portal meshes in both modes. At `d23012b4` + `e5671528`, disabled has no particle pools/emitter state; enabled has all six pools with visible-instance counts 42, 21, 21, 21, 21, and 2 (143 total; scene totals 711 pools/emitters). Depot fixture/build and both runtime logs exit 0 at `data/diagnostics/portal-particles-{depot-build,disabled,enabled}-e5671528.log`. This headless state proof excludes pixels, audibility, live toggling, and density.
- `godot/tests/world_portal_density_flow.gd` via `native_input_fixture portal-density` — controlled disposable copy of portal 197007 with only the six `NO_GLOBAL_SCALE` bits cleared; the original asset hash remains unchanged. It samples known pool visible quads before/after real `GameClient` Options slider 100→10 and after owned same-map `NewWorld`. RED at `cf84a712` (`data/diagnostics/portal-density-red-cf84a712.log`, exit 101): existing placement 484→484.22, fresh placement 462.67 instead of below 30% of baseline. At `79d792b0`, Depot `kdhjvgmnt3` builds the fixture (exit 0; `data/diagnostics/portal-density-depot-green-build-retry1.log`) and runtime GREEN exits 0 (`data/diagnostics/portal-density-green-79d792b0.log`): existing 485.67→488.0 while the fresh placement is 46.67. This proves controlled density-sensitive state behavior, not retail pixels, audible output, or full parity.

## Known gaps (current cycle)

- [ ] Animated emitter tracks: `set_animation` keys emission rate, speed and `enabledIn` per sequence, but skips global-sequence tracks and tracks with at most one key per sequence; the other tracks use their first key.
- [ ] Clean-room unresolved dependencies: **P3-timer-reset** (birth-velocity timer/reset), **P4-density** (spline rate per unit), **S1-polarity-oracle** (target lit/unlit polarity and coefficients), **P5-product-resolution** (real model/child asset mappings), **P5-recursive-clock** (recursive model animation-parameter time). Details below; none are claimed as implemented. Randomized atlas mask (0x8000), TXAC UV variants, refraction pass and alpha-cutoff discard also remain open. **P7-surface-mask**: exact target collision-mask equivalence is unqualified; native projection uses existing terrain/WMO/doodad/active-game-object collision layers.
- [ ] Emitters of attached item models (weapons, held torches, enchants) and player models; display state spell kits (the HD kobold candle flame is not a model emitter).

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

Layout established from allowed WWV `M2ParticleOld::splinePoints` and real local MD21 6316199: emitter 0 has M2Array count4/MD20-offset264608 at 0x1C0/0x1C4, float3 payload [(0,0,0),(0,0.002777778,0),(0,0.005555556,0),(0,0,0)]. Parser preserves controls; the native admission predicate now admits type3 only with complete cubic controls. WWV C3Spline supplies four-point Bézier evaluation, twenty-interval segment-length weighting, and CSplineGenerator supplies clamped width/length parameter bounds, endpoint-on-width-change, tangent-axis launch and zSource rules. Synthetic bent controls are substituted only in a disposable in-memory real asset. RED: no particles/admission; GREEN 2/2 at23823f1c0. **Named gap P4-density:** allowed WWV uses ordinary rate; the supplied spec does not state the rate-per-unit convention. No density multiplier invented; exact target per-length semantics remain open; controlled native bent-curve placement/pixels pass in the continuation below.

### Clean-room P3 provenance and runtime gap (2026-10-15)

The modern scale is established: allowed WWV `M2ParticleOld::BurstMultiplier` is labelled ivelScale, and `Update` multiplies it into the inherited vector. The matching local stride places it at 0x170. Real2994778 emitter3 has flag0x40 and raw3.0; parser now retains3.0 in `inherit_velocity_scale`, rather than an invented default1.0. RED real-asset decode1≠3; GREEN1/1 atd7e7f2ae7. **Named gap P3-timer-reset:** WWV uses a >30ms timer and 30ms/timer displacement scale, but computes its vector only when the previous pool is empty and clears it when live. The supplied acceptance instead calls for a no-live reset; it does not supply the numerical timer convention or reset predicate. No ordinary yd/s or guessed inverse predicate is substituted. Runtime birth contribution, timer-boundary, continuing-trajectory and no-live-reset acceptance remain unimplemented pending allowed target evidence.

### Clean-room S1 blocked polarity oracle (2026-10-15)

**Named gap S1-polarity-oracle:** allowed WWV assigns material bit0 to NOT authored bit0 for ordinary particles, clears it for multitexture/refraction, and initializes material flags separately. This corroborates the supplied inversion ambiguity, not target lighting. Local torch145304 emitter0 has bit0 clear; portal197007 has both bit0 states, but asset flags alone do not establish whether their intended particle effect is lit. Pinned wow.export c2fd7bd M2Loader skips particle payloads and supplies no oracle. No incident-light observation of the target client is available in this checkout. Therefore shader selection, normals, lighting coefficients and fixed-light pixel acceptance remain blocked; existing unshaded blend/fog behavior stays unchanged. No generic PBR or invented flag polarity is substituted.

### Clean-room P5 blocked asset/lifecycle provenance (2026-10-15)

**Named gap P5-product-resolution:** bounded local-cache census (83,123 parseable unqualified M2 files; not a current-build publication audit) found no nonempty geometry/recursion filenames after stripping NUL terminators. Nonzero M2Array counts frequently describe a single NUL, not a referenced asset. Allowed WWV retains the two legacy filenames but has no implemented per-particle geometry/child-emitter consumer establishing Retail product-qualified FDID resolution. Pinned wow.export M2Loader skips particle emitters. Legacy listfile-name guessing is not a valid substitute for [product-isolated model assets](product-isolated-model-assets.md).

The October 17 approved P5 packet supersedes the old **P5-recursive-timing** blocker: per positive replay substep, age/retire parents at lifespan before births, move survivors, request child births at each survivor's resulting position (previous position retained for explicitly interpolated births), then advance child state once. Descendants retain independent live state and drain while their owner receives updates. Root-origin child emission is not part of that advance. Native resolved geometry is distinct from child-emitter definitions; unresolved references fail placement explicitly, never draw replacement quads. Controlled synthetic CPU/native fixtures pass (continuation below); no current Retail asset acceptance is claimed.

**Named gaps retained:** **P5-recursive-clock** (recursive model's own animated generator-parameter clock), no recursion depth limit, no cycle detection or cyclic-asset termination claim. Up to four emitter definitions from a recursive model is not a generation limit. The explicit resolved-resource boundary supplies interpolation policy; no modern authored-bit mapping is guessed. Ordinary root inherited-velocity timer/reset remains P3-timer-reset, separate from a recursive birth's known parent-particle velocity.

### P5 approved continuation (2026-10-17)

Approved behavioral input: `/home/osso/.worktrees/logs/m2-p5-s1-mat-source-assistance-2026-10-17.md`, SHA256 `b16ad486bd0e706d2369679f91ef24569959e7b5709ca4d29503701482edb204` (P5 section only). W/C labels are provenance, not accessible implementation sources. Geometry references suppress billboard output; geometry instance state carries independent lifetime position, birth orientation/spin, scale, RGB and opacity. Synthetic geometry uses the existing two scale channels (X/Z share X); no untouched Retail geometry parity is asserted. Birth-orientation decomposition is limited to geometry/recursive consumers: ordinary billboard births can use legitimate zero-scaled hidden bone frames without introducing an unused NaN quaternion. This P5 regression control is separate from the existing Godot zero-Basis diagnostics seen in the master-only FPS run.

Selected DB2 directory resolves to Retail `12.1.5.70448`. A bounded extracted-cache census finds no nonempty geometry or recursive filenames in 83,133 legacy-cache M2s and three product-qualified wow caches (28,040 / 28,211 / 28,213 M2s); receipt `/home/osso/.worktrees/logs/m2-p5-reference-census.json`. This is not a complete current-publication census. **P5-product-resolution** remains open; no legacy-name guessing or unrelated models substituted. Native proof must therefore be explicitly synthetic.

#### P5 acceptance proof

REDs observed before the corresponding implementation: geometry references emitted replacement quads (`m2-p5-red.log`); native geometry/recursion consumer absent (`m2-p5-native-red-history-receipt.log`). The hidden-bone ordinary-billboard regression additionally reproduced an unused NaN orientation before its correction (`m2-p5-billboard-frame-red.log`). First native setup then hit the current strict asset-loader boundary: the old synthetic template had no primary SFID. The fixture now supplies only explicit SFID910006 and its numeric skin file; no neighboring-name lookup was added.

| Capability | Proof |
|---|---|
| One/two moved parents; one tagged-child age increment; no root-origin births; inherited velocity/interpolation | CPU controls and native one/two-parent buffer assertions pass. Native outputs:1/2 actual six-vertex3D geometry instances and6/11 child quads (including the pre-existing tagged child). |
| Parent exactly at lifespan; finite parent/child/grandchild draining; zero update;0.1s replay | Ten internal P5 controls pass; two external boundary controls pass. Three-generation draining is CPU proof. Native two-generation drained geometry/child uploads both report0 visible instances; owner teardown destroys both nodes. |
| Independent geometry lifetime transform/RGBA and actual tinted raster | CPU numeric scale/orientation/RGBA controls plus native per-instance deltas and red-tinted raster pixels pass. Both1920×1080 PNGs were inspected; no replacement quad model was used. |
| Current product assets / recursive parameter animation / cyclic policy | Not proved or implemented: **P5-product-resolution**, **P5-recursive-clock**, no depth limit and no cycle detection remain. |

Scoped regression:33 ordinary particle tests plus12 P5 controls pass (45 total; the available-capacity descendant fixture has a later focused pass). Native library at `d7b4d366c` builds without compiler warnings; subsequent changes are fixtures only. Logs `/home/osso/.worktrees/logs/m2-p5-{particle-regression,core-current,drain-current,native-current-build,native-green}.log`; original PNGs, hashes and synthetic-only receipt in `/home/osso/.worktrees/logs/m2-p5-native-proof/`. No full suite or untouched-Retail pixel parity is claimed.

Post-master integration `30ca371e2` includes the material-transition changes. Native rebuild (173.8s), helper-driven `cargo check`, formatter and both synthetic native cases pass; no compiler warnings. CPU particle sources/tests are unchanged from the45-control proof above, so that proof was retained rather than rerun. Revision-stamped logs: `/home/osso/.worktrees/logs/m2-p5-integration-{build,check,fmt}.log`; fresh PNGs, hashes and native output: `m2-p5-integration-proof/` beside them. Independent read-only artifact verification passed within these synthetic limits. Hidden-bone geometry, recursive implode/emitter-plane/inherited-scale combinations and pool-exhaustion fairness are not covered by this acceptance; no additional parity claim is made.

### Integrated proof

After merging origin/master, `edc9c82be` passes all eight `particle_cleanroom_` controls (no full suite), native helper build exits0 without compiler warnings, and Godot-workspace `cargo fmt --all --check` exits0. Logs: `/home/osso/.worktrees/logs/m2particles-{integrated-green,native-green-build,native-stable,fmt-check}.log`. Later changes are fixture/docs only. Helper has no cargo-check mode; native build supplies compile proof rather than claiming an unrun check. S1/P5 and the named partial P3/P4 boundaries remain open.

### P6 — authored depth ordering (2026-10-15)

Supplied clean-room P6 establishes authored0x2, transformed particle depth, maximum-depth-first order, and unchanged live-list order without the bit. `EmitterSim::quads` now sorts only flagged live particles using their world center projected onto the current view back axis (ascending dot product = far first); model-space centers are transformed before comparison. Head/tail pairs stay together. Native `ParticlePool` consumes that sequence without a second spatial sort. Actual Blood Presence165894 emitter0 has flag0x40422, including0x2; this is one observed asset, not a full census.

RED: both new CPU controls fail because near-born-first remains first. Tests cover forward/reversed camera, flag-off insertion order, and paired geometry. `particle_cleanroom_sort.gd` drives the shared simulator and native pool through two stationary red/blue half-alpha heads over black, with sort/camera toggles and additive controls, capturing1920×1080 PNGs. GREEN2/2 at5b76a724d. Native overlap8/8 PASS: alpha sort switches center RGB from(0.7373,0,0.5373) to(0.5373,0,0.7373), reverse camera exchanges it; all additive cases remain(0.7373,0,0.7373). Eight1920×1080 PNGs saved in AgentShared/2026-10-15/m2-particles and alpha/additive previews inspected. Logs `m2particles-p6-{red,green,native-build,native}.log` under /home/osso/.worktrees/logs/.

### P7 — world collision birth projection (2026-10-15)

Authored0x2000 on world-space plane/sphere births invokes an actual downward20-yd segment intersection before first ballistic integration. The sampled lifetime scale’s larger component offsets the hit along world-up (`wow_to_world * WoW+Z`). An actual miss preserves the candidate position; unflagged/model-space births and surviving particles are not queried. No scalar terrain-floor substitute exists. `EmitterSim::update` now requires an explicit segment query, and every native placement supplies the existing Godot map collision space using terrain/WMO/doodad/active-game-object layers without render-visibility filtering. **P7-surface-mask** remains a target equivalence gap, not a claim that generic map collision exactly reproduces Retail masks.

Highest-fidelity RED: `particle_cleanroom_ground.gd` builds real flat/raised/too-deep collision surfaces and an opening. Before projection, flat/raised births incorrectly stay at5 instead of0.5/3.5; opening, beyond20, unflagged and model-space controls remain5 (4/6 pass). CPU controls additionally cover plane/sphere world-up and projection-before-motion, no-hit and no re-projection of survivors. GREEN at7cfed41dd: all12 focused clean-room CPU controls pass; native collision6/6 pass with flatY0.5, raisedY3.5 and opening/beyond20/unflagged/model-spaceY5. The1920×1080 `p7-ground-births.png` capture was inspected; it is controlled geometric visibility, not real-map/target mask parity. Logs `/home/osso/.worktrees/logs/m2particles-p7-{red,green,native-build,native}.log`.

### P4 native continuation (2026-10-15)

`particle_cleanroom_spline.gd` loads the real6316199 type3/four-control header through a native probe, substitutes explicit upward/downward bent controls and a white test texture, then uses normal `ModelParticles::from_model` → `ParticlePools::place` → `PlacedParticles::update_and_draw`. Native pool centers must satisfy the supplied cubic, and every center must have an actual bright raster pixel; changing only the middle controls reverses the curve. This is a controlled native placement proof, not an untouched helmet visual claim. Existing source-layout/curve REDs precede this acceptance fixture. Native GREEN at85932e156: each up/down curve admits99 particles,86 beyond absoluteY0.5; all99 centers lie on the expected cubic and have a bright raster pixel. Both1920×1080 PNGs (`p4-bent-up.png`, `p4-bent-down.png`) were inspected. The controlled white-texture curve visibly reverses when the middle controls change.

P4-density re-investigation: allowed WWV `CParticleGenerator::GetEmissionRate` returns authored rate plus signed variation; `ParticleEmitter::EmitNewParticles` accumulates seconds×that rate with no spline-length multiplier. `CSplineGenerator` has no rate override. These facts establish the allowed viewer’s ordinary-rate behavior, not the supplied reference’s unspecified rate-per-unit rule. No supported target numeric mapping/polarity has been supplied, so P4-density remains named rather than inventing a multiplier or freezing speculative behavior into a test.

### Continuation integration proof

P6 implementation5b76a724d, P7 implementation7cfed41dd and P4 native fixture43dc121fb are preserved as separate commits; origin/master is merged forward at54731f9c2. Native build85932e156 exits0. At test-onlydcf850fe8, all13 focused CPU controls pass (including translated/rotated model-space sorting); native fixtures on the same implementation pass sorting8/8, real collision6/6 and both bent curves99/99 visible centers. `cargo fmt --all --check` passes. Logs `/home/osso/.worktrees/logs/m2particles-{continuation-green,continuation-fmt,final-sort,final-ground,final-spline,p4-native-build}.log`. No full suite or unrun cargo-check claim. P4-density remains blocked on an allowed target rule; P7-mask and older P3/S1/P5 provenance boundaries remain explicit.
