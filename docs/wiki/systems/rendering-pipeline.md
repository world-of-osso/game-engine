# Rendering Pipeline

The engine renders WoW assets using Bevy 0.19: M2 character and doodad models, ADT terrain, skybox M2 models, particle effects, and an in-world UI layer. Rendering is split across `src/rendering/` subsystems, each owning one concern.

## M2 Models

M2 models are parsed by `src/asset/m2_format/` (pure, no Bevy) and assembled into Bevy meshes in `src/asset/m2.rs`. Each skin batch becomes a separate `Mesh3d` + `MeshMaterial3d<StandardMaterial>`. Material properties come from the M2 material table (flags + blend mode per batch).

**Blend modes** map as:
- 0 → Opaque, 1 → Mask(0.878), 2/3/7 → Blend, 4-6 → Additive, unknown → Additive (safe default)

See [[character-rendering]] for character-specific mesh assembly.

## M2 Effect Material Empty boundary

Commit `0a1a1bfb` omitted the full `M2EffectMaterialPlugin` for exact `Empty`, but also removed `Assets<M2EffectMaterial>`, which remained required by active consumers. The rebuilt PID `2339740` therefore panicked during scene setup before IPC readiness; after identity verification, its stale socket was removed. A later rebuilt PID `2365242` reached `sync_equipment` and panicked for the same missing asset resource; its stale socket was likewise removed only after identity verification. These are failure evidence, not runtime proof.

Commit `49304144` (`Skip inactive scene setup validation`) attempted optional-resource and run-condition compatibility for those failures, but was incomplete. Commit `e7f98704` (`Keep Empty M2 assets without render plugin`) forward-reverted that approach and establishes the final architecture: exact `Empty` initializes lightweight `Assets<M2EffectMaterial>` only, while omitting Bevy `EntitiesNeedingSpecialization<M2EffectMaterial>` and the full material/render schedules. `Character` and later cumulative stages, plus unconfigured and debug runs, retain the full plugin.

The original PID `2297374` profile attributed recurring CPU to Bevy material-specialization parameter validation, not actual M2 draws. Corrective RED evidence: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-red.log`. GREEN: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-green.log`. Formatting: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-cargo-fmt.log`. Rust readability: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-readability/`. Earlier panic artifacts remain failure evidence: `/tmp/claude/game-engine-perf/empty-scene-setup-validation-red.log` and `/tmp/claude/game-engine-perf/empty-scene-setup-validation-red-3.log`.

Final PID `2390217` matched `e7f98704`, stayed connected with one link/player, retained the FPS text, and had zero terrain, `Camera3d`, or displayed NPCs. Its post-fix profile contained no Hanabi, `M2EffectMaterial`, or M2 `EntitiesNeedingSpecialization` symbol. Twelve passive windows after a 30-second warm-up measured **10.24% mean**, **10.20% median**, and **10.60% maximum** CPU; only **2/12** met `<=10.0%`. The M2 boundary is verified, but the overall Empty CPU and Character gates remain open.

## Terrain

ADT terrain uses a custom WGSL shader (`assets/shaders/terrain.wgsl`). Split files are loaded in three parts: root `.adt` (heights/normals), `_tex0.adt` (texture layers), `_obj0.adt` (doodads/WMOs). See [[terrain]] for details.

## Gizmo Registration

Commit `1503cd1c` disabled Bevy `GizmoPlugin` and `GizmoRenderPlugin` only when the configured cumulative stage was the explicit fixed `Empty` diagnostic stage. The August 11, 2026 `17c2bdc6` change also disabled `LightPlugin`; that historical boundary is superseded. Native RED evidence for `be9a1ff7` shows PBR still requires the `LightPlugin` providers `PointLightShadowMap` and scattering-medium assets even when Empty contains no light entities. Current Empty therefore keeps `LightPlugin` registered and disables only `GizmoPlugin` and `GizmoRenderPlugin`. Native GREEN is pending at `data/diagnostics/movement-perf-20260905/update-schedule-isolation/strict-empty-fixed/`; this is not yet a passing runtime claim. Unconfigured/default runs, `Character` and later stages, debug modes, and screenshot/default paths retain LightPlugin and gizmos. No project gizmo consumers exist; the gizmo boundary removes their registration/render work without changing project-owned behavior.

PID `2655273` provided the pre-A/B attribution: a **59.950-second** identity-bound `perf` interval collected **2,153 samples** with **0 lost samples**. `GizmoBuffer<LightGizmoConfigGroup>::queue` was the strongest current project-retained symbol at **1.51% of sampled CPU**. This is profiler share, not Linux-core utilization. RED/GREEN evidence is under `/tmp/claude/game-engine-perf/empty-gizmo-registration-{red,green}.log`; readability evidence is under `/tmp/claude/game-engine-perf/empty-gizmo-registration-readability/`.

Rebuilt candidate PID `2846177` (SHA `84f6ecc9590979a2fba498b8206549d33ebff7fdc27878a16331acccc19811da`) completed the A/B proof. After a 30-second warm-up, **10/12** passive windows passed, with **9.47% mean** and **10.60% maximum** CPU versus baseline PID `2655273` at **8/12**, **9.97% mean**, and **10.90% maximum**. The passive mean improved by **0.50 percentage points**. A 60-second profile collected **1,936 samples** with **0 lost samples** and no `Gizmo` symbol, confirming the targeted path disappeared. Commit `1503cd1c` is retained; the strict `<=10.0%` every-window gate remains open.

Readiness held one client/server with healthy IPC/admin ping, approximately **10.09 FPS**, zero displayed NPCs/cameras/terrain, no panic/device-loss/OOM, and no Character advancement. One startup `Received despawn for an entity that does not exist` warning was recorded without blocking readiness.

## Particles

GPU particles run via `bevy_hanabi` 0.19. Each live particle is a separate Bevy entity with `Mesh3d` (unit quad) + `StandardMaterial`. The emitter (`ParticleEmitterComp`) accumulates emission and resolves bone position per frame. Color, opacity, and scale use 3-point FakeAnimBlock interpolation. Texture tiles are static (chosen at spawn, not animated).

Commit `beead231` registers `ParticlePlugin` only when the configured cumulative stage includes `Particles`. Exact `Empty`, `Character`, `Skybox`, `Terrain`, `Npcs`, and `Lighting` therefore do not register Hanabi or its render graph; `Particles`, `Ui`, and an unconfigured normal run retain it. This removes plugin/render-graph work rather than merely skipping emitter systems.

The pre-fix strict-Empty profile for PID `2176863` (`character-camera-gate-2176863.perf-*.txt`, captured before `beead231`) sampled `bevy_hanabi::render::VfxSimulateNode::run` at **2.18% self CPU**, proving stage-gated emitter systems alone left Hanabi render work active. The same profile also sampled sprite 2D bind-group command application at **1.97%**.

The post-fix PID `2297374` profile contained no Hanabi symbol. Same-build passive 10-second process samples measured **12.50%**, **12.70%**, and **9.90%** of one core. The stage-registration boundary is verified, but measurement variance prevents a causal CPU-savings claim or a stable `<=10%` result.

**Current limitations:** one entity per particle is the main performance bottleneck; no drag/wind physics; no tail/ribbon particles; bone position can be stale for fast-moving animated bones.

## Skybox

Skybox models are rendered via the `SkyboxM2Material` path (depth writes and shadow/prepass disabled). Scene selection drives a `Light.csv → LightParams → LightSkybox → SkyboxFileDataID` lookup chain. See [[skybox]] for details.

Procedural clouds use three startup-generated 512×1024 RGBA textures with six-octave simplex noise. Commit `b2b07e5b` removed the synchronous five-second runtime regeneration path: the textures remain fixed after startup while `assets/shaders/sky.wgsl` animates their UVs from time/cloud parameters. This preserves cloud settings and removes the measured simplex CPU hotspot. Earlier FPS comparisons were invalidated by a separate Wayland/Vulkan presentation stall; see [[procedural-cloud-regeneration]].

The current separate InWorld performance investigation measured an enabled control at **12.332 FPS / 81.157 ms** from six unprofiled CLI samples with **71 remote entities** (`game-engine` `00e7b3b0`, `ui-toolkit` `5ead575`). An all-text-disabled run measured **37.415 FPS / 26.785 ms** with **76 remote entities** (`game-engine` `6806717c`, `ui-toolkit` `43a2784`). The revisions and exact workloads differ, so the result strongly implicates UI-text-associated rendering with moderate confidence, not proof. An equality-guard experiment (`ui-toolkit` `33fa74d`, reverted by `36d4692`) measured **11.373 FPS / 90.230 ms** and is rejected. The initial same-binary M2 UV pair measured **31.767 FPS / 54.177 ms** enabled versus **36.843 FPS / 54.482 ms** disabled, but its first sample in each condition immediately followed an expensive `dump-scene` request and its recorded readiness workloads differed (**135** versus **133** remote entities). That pair is **preliminary/inconclusive pending a clean repeat** with a prospective performance warm-up; it supports no M2 performance conclusion or fix. Temporary selector code was removed in `58e2f9c2`, then restored in `a7784e70` only for the repeat. Engine load reached approximately **224–277% CPU**, about **236% Compute Task Pool**, and **74% adapter-wide GPU busy on shared card1/renderD128**. Existing profiler attribution is dominated by wgpu buffer transitions/unmaps; text extraction self-cost was low and downstream causality remains unresolved. These profiler samples explain where to investigate, not how fast the engine runs. See [[procedural-cloud-regeneration]].
## Character Rendering

Character models live in `src/rendering/character/`. Geoset visibility is driven by character customization choices and equipment. Texture compositing happens in `src/asset/char_texture.rs`. See [[character-rendering]].

## Materials and Blend Modes

The complete WMVx blend mode reference:
| Mode | Behavior |
|------|----------|
| 0 | Opaque |
| 1 | Alpha test ≥ 0.7 |
| 2 | Alpha blend |
| 3 | Additive (SRC_COLOR, ONE) |
| 4 | Additive alpha (SRC_ALPHA, ONE) |
| 5 | Modulate |
| 6 | ModulateX2 |
| 7 | Blend add (ONE, ONE_MINUS_SRC_ALPHA) |

## Known Bugs

- **Bloom + PointLight = black screen** (Bevy 0.18): the real trigger is enabling bloom in a scene that also contains a `PointLight`. The older text/skinned-mesh explanation was a false correlation from the original reproduction. See [pointlight-skinned-mesh-bug-2026-04-04](../pointlight-skinned-mesh-bug-2026-04-04.md).
- **Torch halo**: `blend_mode > 7` values previously fell back to Opaque (wrong); now fall back to Additive. See [torch-halo-investigation-2026-03-30](../torch-halo-investigation-2026-03-30.md).
- **Particle bone staleness**: particle emitters don't follow fast-moving animated bones well.
- **Procedural cloud regeneration (fixed)**: before `b2b07e5b`, one 512×1024 six-octave cloud texture regenerated synchronously every five seconds, consuming about 60% of sampled CPU in simplex functions. Runtime regeneration is now removed and the hotspot disappeared from later profiler samples; FPS attribution from the earlier comparison is invalid because a separate Wayland/Vulkan presentation stall was present.
- **Wayland/Vulkan presentation stall (fixed)**: VSync-enabled FIFO presentation blocked `Queue::present` for approximately 0.96–0.97 seconds on the affected surface. Commit `89f58874` selects Mailbox for VSync-enabled mode; corrected fully visible unfocused evidence reached 28.35–29.32 FPS with approximately 38.7 ms CLI request latency.
- **SSAO/MSAA compatibility (fixed)**: commit `cff4ad46` removes SSAO from the real `WowCamera` when default MSAA4x is active. TAA restores `Msaa::Off`, `TemporalAntiAliasing`, and SSAO. The RED test reproduced SSAO with `Msaa::Sample4`; the exact GREEN test passes, removing Bevy's per-frame incompatibility errors. No runtime FPS improvement is claimed before restart measurement.
- **Permanent Empty-to-Npcs camera render-bundle gate**: commit `3b144afc` removes WowCamera TAA, SSAO, depth/normal/motion prepasses, `TemporalJitter`, and `MipBias` through cumulative stages `Empty`, `Character`, `Skybox`, `Terrain`, and `Npcs`. `Lighting` onward and unconfigured/default stages retain graphics-option-driven behavior, including TAA restoration with `Msaa::Off` or configured MSAA depth/normal prepasses. Bloom, render scale, CAS, DoF, camera identity, tonemapping, shadow filtering, spatial audio, UI/network/IPC, and FPS overlay remain unchanged. Alessio accepted the diagnostic cause; machine samples remain supporting only. Temporary selector commit `e9d3d470` removed the opt-in path. The historical PID `3367453` diagnostic client exited at `2026-08-09T06:05:43Z` with `WindowCloseRequested` followed by `AppExit Success`; it was not permanent-build verification.
- **Strict Empty world-camera boundary**: commit `96e1308a` makes `spawn_world_environment` create the world `WowCamera`/`Camera3d` only when the cumulative stage includes `Character`. `Empty` has no world camera; `Character` and later stages still do. The standalone performance panel/UI camera remains, and the `3b144afc` post-process stage gate is unchanged. Tests in `tests/unit/main_tests.rs` prove zero world cameras for `Empty` and one camera across `Character` re-entry. This audit made no live relaunch or performance claim.
- **Remote interpolation stage boundary**: commit `538e8329` keeps connection, replication receive, and replicated target synchronization active before `Npcs`, but gates `interpolate_remote_entities` to cumulative `Npcs`. `Empty`, `Character`, `Skybox`, and `Terrain` therefore no longer mutate remote visual `Transform`; RED/GREEN behavioral evidence is recorded in `/tmp/claude/game-engine-perf/strict-empty-interpolation-red.log` and `strict-empty-interpolation-green.log`. The live replacement used full commit `538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, and socket `/tmp/game-engine-2960624.sock`; its pre-gate comparison used full commit `96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, and socket `/tmp/game-engine-2592665.sock`. Both had zero world `WowCamera`/`Camera3d`; remote counts were `133` versus `134`. Passive aggregate CPU changed `325.49% → 288.12%`, compute-pool CPU `229.17% → 197.62%`, and client gfx occupancy `6.93% → 5.60%`; these are cautious attribution evidence only because runqueue delay and live conditions drifted, so FPS/frame direction is not acceptance evidence. The subsequent strict-Empty eu-stack capture has no `interpolate_remote_entities` stack, but about `2.9` cores remain; the root-cause loop continues.
- **Current UI/render-resource investigation (open)**: all-text disablement strongly implicates UI-text-associated rendering with moderate confidence, but the enabled and disabled runs used different revisions and 71 versus 76 remote entities. The calibrated profile points primarily to downstream wgpu buffer/resource work rather than high self-cost in text extraction. Shadow-only isolation is implemented and behaviorally GREEN; live measurement is pending. No code fix or FPS improvement claim exists yet.
- **Strict Empty diagnostic pacing**: commit `4fb2e5c9` selects a fixed 100 ms frame interval only for exact `InWorld` + exact `Empty`, using the existing limiter. Character/later stages and other states retain persisted/global frame limiting and PresentMode; FPS overlay, networking, and IPC remain registered. This paces frames but does not remove render/application work. Pre-pacing PID `2093844` measured 490.62 FPS at 369.05% one-core CPU; the first paced PID measured 11.20%. After later Empty scheduling/graph fixes, PID `2283621` measured 9.80%, meeting the numerical threshold. Ten FPS remains temporary rather than the accepted final design, so Character stays blocked.
- **Strict Empty camera/input boundary**: commit `c446d81c` gates `sync_camera_options`, `camera_input`, `cursor_grab`, `player_movement`, and `camera_follow` at exact `Character`. Empty therefore skips camera, input, cursor, movement, collision/pathing collection, raycast setup, and follow dispatch; Character and later stages retain them. Rebuilt PID `2176863` stayed connected at 10.01 FPS with zero world camera/terrain/displayed NPCs and measured 11.10% of one core. The prior paced result was 11.20%, but remote workload changed, so no measurable improvement is accepted; the CPU gate remains open.
- **Strict Empty FPS graph boundary**: `8cac2b03` disabled `frame_time_graph_config` only at startup, but later writers (`apply_loaded_client_options`, `sync_hud_visibility_toggles`, `apply_snapshot_to_world`) restored it; `/tmp/claude/game-engine-perf/empty-fps-graph-2246158.webp` shows the solid red graph. Follow-up `cc5780a8` makes all overlay visibility writers stage-aware: exact Empty keeps FPS text enabled and graph disabled, while Character/later stages restore the graph. Rebuilt PID `2283621` visually removed the graph, stayed connected at 9.95 FPS with zero world scene content, and measured 9.80% of one core. This meets the numerical CPU threshold under temporary pacing but is not accepted as the final Empty design or Character gate.

## Sources

- [game-engine app setup](../../src/app_setup.rs) — strict-Empty gizmo plugin filtering and screenshot/default retention
- [game-engine main](../../src/main.rs) — stage-aware gizmo policy wiring
- `game-engine` commit `1503cd1c` — disable Bevy gizmo plugins only for explicit Empty
- `/tmp/claude/game-engine-perf/empty-gizmo-registration-red.log`, `empty-gizmo-registration-green.log`, and `empty-gizmo-registration-readability/` — behavioral and readability evidence
- `/tmp/claude/game-engine-perf/final-polling-loops-perf-2655273.data` and companion header/self/children/script artifacts — baseline identity-bound profiler evidence
- `/tmp/claude/game-engine-perf/empty-gizmo-cpu-2846177.json` and rebuilt candidate 60-second perf artifacts — accepted A/B evidence; final every-window CPU gate remains open

- [particle-system.md](../particle-system.md) — emitter architecture, known limitations
- [torch-halo-investigation-2026-03-30.md](../torch-halo-investigation-2026-03-30.md) — blend mode fallback fix, WMVx reference
- [pointlight-skinned-mesh-bug-2026-04-04.md](../pointlight-skinned-mesh-bug-2026-04-04.md) — bloom/point-light Bevy rendering bug
- [procedural-cloud-regeneration](../investigations/procedural-cloud-regeneration.md) — synchronous cloud regeneration investigation and runtime-removal evidence
- [camera post-process](../../../src/rendering/camera/camera_post_process.rs) — permanent scene-stage render-bundle gate and graphics-option restoration
- [camera post-process tests](../../../tests/unit/camera_post_process_tests.rs) — Empty-stage removal, Lighting restoration, MSAA restoration, and unconfigured behavior
- [world environment](../../../src/game/state/game_state.rs) — strict Empty world-camera boundary
- [world environment tests](../../../tests/unit/main_tests.rs) — Empty has no world camera; Character retains one across re-entry
- [app setup](../../../src/app_setup.rs) — stage-dependent ParticlePlugin registration
- [particle system](../../../src/rendering/particles/mod.rs) — Hanabi plugin and emitter-system stage boundary
- AGENTS.md — `src/rendering/` structure

## See Also

- [[character-rendering]] — character-specific pipeline, geosets, texture compositing
- [[terrain]] — ADT terrain rendering, split files, doodad placement
- [[skybox]] — skybox M2 model rendering, light lookup chain
- [[procedural-cloud-regeneration]] — cloud texture generation and current UI/render performance evidence
- [[animation]] — M2 bone animation, crossfade system
