# Bevy and Godot Directional Shadow Comparison

Read-only source comparison of game-engine `c3e9be93`, patched Bevy `20ed24db`, upstream `bevy_light 0.19.0`, and Godot `4.7.2-stable` `ed1daf0bf001b61586d9930840f2f1394092c079`. It identifies implemented paths, not a runtime performance winner.

## Directional-shadow path

| Area | Bevy 0.19 path | Godot 4.7.2 path | Limit / follow-up |
|---|---|---|---|
| CPU caster selection | `check_dir_light_mesh_visibility` scans eligible mesh entities for every directional light/view pair, then checks every cascade frustum. It uses an OBB-vs-orthographic-frustum test with near-plane culling disabled. | `_scene_cull` scans scene instances, then loops directional lights × cascades. It applies camera visible layers, the light caster mask, its cascade frustum, and `cull_directional_light`. | Both are instance/entity scan × cascades. Godot's layer check includes camera `visible_layers`; do not describe it as ignoring camera layers. |
| Directional caster volume | The inspected upstream path uses the cascade orthographic frustum only. | `RenderingLightCuller` adds back-facing camera planes and silhouette-edge planes extruded along the light direction; `cull_directional_light` rejects AABBs outside them before the cascade-frustum test. | This is a real algorithmic difference, but no local measurement establishes its win for this scene. |
| Queueing and batching | `queue_shadows` incrementally removes/requeues `DirtySpecializations`; bins by mesh asset plus compatible pipeline/draw/material/slab keys, allowing batch/multi-draw where supported. | Forward Clustered groups compatible adjacent surfaces into repeated instanced draws, excluding multimesh and mesh-instance cases; the draw path consumes that repeat count. | Both batch shadow work. Local `Mesh3d(meshes.add(mesh))` per M2 batch can give otherwise-compatible instances distinct mesh asset identities, a potential blocker—not proof that Bevy cannot batch. |
| Shadow geometry | No automatic shadow-specific mesh substitution was demonstrated. The depth-only shadow draw can omit the material bind-group key (`light.rs:2603-2610`). | A material using the shared shadow material can select `mesh_get_shadow_mesh`; otherwise it uses the ordinary surface. The shadow render list also selects mesh LOD when enabled. | A Godot shadow mesh does not necessarily contain fewer triangles. Bevy's inspected path does not prove a corresponding automatic substitution. |
| Redraw and depth | Existing Bevy shadow depth attachments are marked unused each frame so their first draw clears them. | Directional cascade submissions are rebuilt in scene cull and passed to Forward Clustered shadow rendering, which clears depth for the opening/clearing pass. | Both redraw directional shadows. Do not infer directional caching behavior from Godot's separate positional-shadow atlas logic. |
| GPU preprocessing | `gather_shadow_cascades_for_view` includes directional cascade views. Per view, preprocessing selects direct work for `NoIndirectDrawing`, early GPU occlusion when configured, otherwise GPU frustum-culling preprocessing. | Not compared here. | The local runtime mode and end-to-end occlusion workflow were not measured. The nearby Bevy TODO is not absence evidence. |

## Game-engine configuration

The InWorld sun uses a 4096² directional shadow map and `default_cascade_shadow_config()`: four cascades from 0.1 to 500 units, first far bound 15, overlap 0.2. The historical September 24 profile's approximately 44% GPU shadow share is not current comparative evidence and does not establish a Godot/Bevy delta.

## Scope boundary

No runtime captures, triangle counts, draw counts, GPU mode inspection, or benchmark comparison was performed. Candidate optimization priority remains unproven until the same scene/configuration is profiled on both sides.

## Sources

- `src/rendering/lighting/shadow_config.rs:3-16` and `src/game/state/game_state.rs:262-274` — game-engine shadow-map and cascade settings at `c3e9be93`.
- `/home/osso/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy_light-0.19.0/src/lib.rs:327-461` — upstream Bevy CPU directional-light visibility scan and OBB cascade-frustum test.
- `/syncthing/Sync/Projects/world-of-osso/bevy-patches/bevy_pbr/src/render/light.rs:1918-1921,1974-1980,2505-2633,2660-2729` — patched Bevy cascade setup, depth reset, queueing, and batch keys at `20ed24db`.
- `/syncthing/Sync/Projects/world-of-osso/bevy-patches/bevy_pbr/src/render/gpu_preprocess.rs:630-665,793-805` — gathered cascade views and conditional direct/GPU-occlusion/GPU-frustum preprocessing.
- `src/rendering/model/m2_spawn.rs:619-650` — per-batch `meshes.add` identity boundary.
- `/home/osso/Repos/godot-shadow-comparison/4.7.2-stable/renderer_scene_cull.cpp:2922-3258,3472-3492` — Godot layer-aware scan, directional cascade selection, and submissions at `ed1daf0bf001b61586d9930840f2f1394092c079`.
- `/home/osso/Repos/godot-shadow-comparison/4.7.2-stable/servers/rendering/rendering_light_culler.cpp:231-416` — directional silhouette-plane construction and AABB rejection.
- `/home/osso/Repos/godot-shadow-comparison/4.7.2-stable/servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp:598-617,877-895,1105-1118,2796-2907,4222-4255` — instancing, draw consumption, LOD, shadow clear/draw, and shadow mesh substitution.
- [[movement-performance]] — historical local profile and its evidence boundary.

## See Also

- [[movement-performance]] — local historical shadow-cost measurement, not an engine comparison.
