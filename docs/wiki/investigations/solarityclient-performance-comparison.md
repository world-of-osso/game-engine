# solarityclient Performance Comparison

Read-only comparison (2026-09-24) of [solarityclient](https://github.com/isekaishy-jpg/solarityclient) (`~/Repos/solarityclient`, Rust, raw Vulkan via `ash`, WotLK 3.3.5a, commit of 2026-09-12) against our in-world hot paths. No code changed and nothing measured in our engine; every item below is a candidate, not a result.

## Their measured numbers

- World replay, GTX 1070, 1280x720, no NPCs/network: "about 437-525 FPS" (`docs/architecture/world-performance.md:1986`).
- Glue UI: "1280x720 windowed | 1,150-1,186" FPS (`frame-performance.md:18`).
- Populated live client: "full 10 ms" (~100 FPS), unattributed (`critical-performance-audit-2026-09-12.md:20`).
- Targets are 300 FPS floor, 1,200 FPS stretch; no "800 fps" appears in code or docs.

Methodology worth copying: 2,400-frame phases, alternating new/old/new/old runs, baseline binary pinned by SHA-256, per-frame camera and draw-count equality between runs (`world-performance.md:7-33`).

## Where their frame cost goes away

Their render path is not exotic: no instancing, no indirect draws, CPU particles, CPU bone poses per placement, one `cmd_draw_indexed` per batch. Speed comes from **few objects and no per-frame churn**:

- One flat bone-matrix `Vec` and one shared particle vertex/index `Vec` per frame, in one persistently mapped per-slot buffer.
- Retained scratch vectors cleared, never reallocated.
- Compact visibility arrays tested before touching large per-model records (17-19%).
- Scenery distance fade/cull by size class (30%).
- Coarse whole-tile terrain rejection before per-chunk tests (7-9%).
- Portal traversal with lazy portal projection (outdoor 129 → 32 µs); doodads admitted only through visited WMO groups.
- Terrain: one merged vertex/index buffer and one RGBA alpha/shadow atlas per ADT tile.
- Dirty-gated UI with unchanged-result suppression and byte-span buffer patching.
- Mesh prep and cache destruction on workers; fence-polled uploads.

## Our structural gaps (code-traced)

| Gap | Evidence | Their counterpart |
|---|---|---|
| Every M2 instance spawns N bone entities + B batch entities with per-instance mesh and material assets, including static ADT doodads that never animate | `rendering/model/m2_spawn.rs:98`, `:631-636`, `:678-714`; `m2_spawn_material.rs:43-148` | Assets shared by identity (`Arc<DecodedM2Model>`); bones are a flat array, not objects |
| Terrain: 256 entities, meshes and `TerrainMaterial`s per tile, separate alpha/shadow `Image` per chunk | `terrain/terrain_spawn.rs:317-350`, `terrain_material.rs:463-507` | One merged buffer + one atlas per tile |
| Distance culling mixes per-tile chunk indices in one global set | `camera/culling.rs:166-177`, `terrain_spawn.rs:339` (`chunk_index: i as u16`) | Per-tile resident lookup table |
| `wmo_portal_cull_system` scans the whole `WmoGroup` query three times per WMO per frame, before its early return | `camera/culling.rs:381-442` | Per-WMO group data, explicit traversal stack, lazy projection |
| One hanabi `EffectAsset` per emitter instance; model particles spawn full animated M2s per particle | `particles/emitters.rs:143-151`, `emitters_model_particles.rs:68-132` | One shared frame particle buffer, pooled slots |
| Animation clips/graphs built per instance; LOD only for NPCs | `animation/bevy_player.rs:64-66`, `animation/lod.rs:78` | Per-placement CPU pose, skipped when frustum-rejected |

Our entity count feeds the Bevy costs already identified in [[movement-performance]] (`extract_skins` top leaf, transform-propagation spin) and the 216% continuous-render floor in [[empty-window-baseline]]. Their serial single-thread frame loop is not transferable to Bevy.

## Candidate order

Constraint (user, 2026-09-24): follow Bevy best practice; do not port solarityclient's custom render architecture. Joint entities for animated models and one entity per batch are standard Bevy and stay. Only fixes that move us toward standard Bevy usage qualify.

1. Static doodads as plain meshes: no joint entities, no `SkinnedMesh` when a model has no animation player.
2. Share mesh and material handles across identical M2/WMO instances (as Bevy's glTF scene spawning does) so automatic batching can merge draws. Unknown: whether Bevy 0.19 batches skinned meshes across instances.
3. Share `AnimationClip`s/`AnimationGraph` per model; one `AnimationPlayer` per instance.
4. Share hanabi `EffectAsset`s per emitter definition with per-instance properties.
5. Fix the chunk-index culling key (tile, chunk).
6. Lower priority: merge terrain chunks per tile; precompute per-WMO group lists for portal culling.

Rejected: flat per-frame bone arrays, custom CPU particle buffers, serial frame loop.

Each needs an entity-count and CPU/FPS A/B using the method in [[movement-performance]].

## Sources

- `~/Repos/solarityclient/docs/architecture/` — `frame-performance.md`, `world-performance.md`, `critical-performance-audit-2026-09-12.md`, `live-frame-timings.md`, `cpu-executor.md`
- [particle-system.md](../../particle-system.md) — our particle bottleneck note (line 78)

## See Also

- [[open-source-wow-clients]] — catalog entry for solarityclient
- [[movement-performance]] — our CPU measurements and A/B method
- [[empty-window-baseline]] — Bevy continuous-render floor
- [[rendering-pipeline]] — current render architecture
