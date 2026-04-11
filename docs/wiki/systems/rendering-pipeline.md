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

## Sources

- [particle-system.md](../particle-system.md) — emitter architecture, known limitations
- [torch-halo-investigation-2026-03-30.md](../torch-halo-investigation-2026-03-30.md) — blend mode fallback fix, WMVx reference
- [pointlight-skinned-mesh-bug-2026-04-04.md](../pointlight-skinned-mesh-bug-2026-04-04.md) — bloom/point-light Bevy rendering bug
- AGENTS.md — `src/rendering/` structure

## See Also

- [[character-rendering]] — character-specific pipeline, geosets, texture compositing
- [[terrain]] — ADT terrain rendering, split files, doodad placement
- [[skybox]] — skybox M2 model rendering, light lookup chain
- [[animation]] — M2 bone animation, crossfade system
