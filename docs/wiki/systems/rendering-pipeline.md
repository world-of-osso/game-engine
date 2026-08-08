# Rendering Pipeline

The engine renders WoW assets using Bevy 0.18: M2 character and doodad models, ADT terrain, skybox M2 models, particle effects, and an in-world UI layer. Rendering is split across `src/rendering/` subsystems, each owning one concern.

## M2 Models

M2 models are parsed by `src/asset/m2_format/` (pure, no Bevy) and assembled into Bevy meshes in `src/asset/m2.rs`. Each skin batch becomes a separate `Mesh3d` + `MeshMaterial3d<StandardMaterial>`. Material properties come from the M2 material table (flags + blend mode per batch).

**Blend modes** map as:
- 0 → Opaque, 1 → Mask(0.878), 2/3/7 → Blend, 4-6 → Additive, unknown → Additive (safe default)

See [[character-rendering]] for character-specific mesh assembly.

## Terrain

ADT terrain uses a custom WGSL shader (`assets/shaders/terrain.wgsl`). Split files are loaded in three parts: root `.adt` (heights/normals), `_tex0.adt` (texture layers), `_obj0.adt` (doodads/WMOs). See [[terrain]] for details.

## Particles

GPU particles run via `bevy_hanabi`. Each live particle is a separate Bevy entity with `Mesh3d` (unit quad) + `StandardMaterial`. The emitter (`ParticleEmitterComp`) accumulates emission and resolves bone position per frame. Color, opacity, and scale use 3-point FakeAnimBlock interpolation. Texture tiles are static (chosen at spawn, not animated).

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
- **Current UI/render-resource investigation (open)**: all-text disablement strongly implicates UI-text-associated rendering with moderate confidence, but the enabled and disabled runs used different revisions and 71 versus 76 remote entities. The calibrated profile points primarily to downstream wgpu buffer/resource work rather than high self-cost in text extraction. Shadow-only isolation is implemented and behaviorally GREEN; live measurement is pending. No code fix or FPS improvement claim exists yet.

## Sources

- [particle-system.md](../particle-system.md) — emitter architecture, known limitations
- [torch-halo-investigation-2026-03-30.md](../torch-halo-investigation-2026-03-30.md) — blend mode fallback fix, WMVx reference
- [pointlight-skinned-mesh-bug-2026-04-04.md](../pointlight-skinned-mesh-bug-2026-04-04.md) — bloom/point-light Bevy rendering bug
- [procedural-cloud-regeneration](../investigations/procedural-cloud-regeneration.md) — synchronous cloud regeneration investigation and runtime-removal evidence
- AGENTS.md — `src/rendering/` structure

## See Also

- [[character-rendering]] — character-specific pipeline, geosets, texture compositing
- [[terrain]] — ADT terrain rendering, split files, doodad placement
- [[skybox]] — skybox M2 model rendering, light lookup chain
- [[procedural-cloud-regeneration]] — cloud texture generation and current UI/render performance evidence
- [[animation]] — M2 bone animation, crossfade system
