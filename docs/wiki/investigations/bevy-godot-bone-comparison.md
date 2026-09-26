# Bevy, Godot, and solarityclient Bone Comparison

Source-only comparison of game-engine `c3e9be93` (documentation commits through `a9d9c950`), patched Bevy `20ed24db`, upstream Bevy 0.19.0, Godot 4.7.2 `ed1daf0bf001b61586d9930840f2f1394092c079`, and solarityclient `f5f5f4a81e5c11241f4c80c117e5dfe9b587dcee`. It records representation and traced update boundaries, not runtime cost or a winner.

## Representation and CPU evaluation

| Area | Bevy / game-engine | Godot | solarityclient | Evidence boundary |
|---|---|---|---|---|
| Bone representation | One joint is one ECS entity with transform components, `AnimationTargetId`, and `AnimatedBy`; target evaluation is a parallel ECS query. Transform propagation tracks changed trees and can skip unchanged subtrees. | `Skeleton3D` owns a contiguous `LocalVector<Bone>`; each `Bone` contains parent/children, local pose fields, cached/global transforms, and nested-set subtree indices. | `M2AnimationSet` owns `Vec<M2Bone>`; pose composition uses local/global `Mat4` vectors and parent-index recursion. | Different representations, not a measured allocation/cache comparison. Bevy is not a heap object per bone. |
| Change suppression | Patched transform propagation checks dirty hierarchy state and uses `set_if_neq` for derived globals. | Pose setters assign and dirty even without equality checks; `_make_dirty` coalesces pending deferred skeleton updates, then dirty global poses are updated. | One reusable `bone_pose_scratch` is recomposed for a placement; it is workspace, not a pose shared among actors. | Godot's dirty gate does **not** guarantee identical-pose suppression while an `AnimationMixer` keeps writing. |
| Local NPC LOD | Replicated NPCs within 30 yd sample every frame; 30–60 yd sample on staggered alternate frames; farther or last-frame-offscreen NPCs freeze joint sampling while clocks continue. | No automatic camera-visibility gate was found in the native mixer/skeleton path inspected here. | Earlier admission gates can skip placement work, but recomposition occurs before the shown final camera-frustum rejection. | This is game-engine policy, not stock Bevy behavior. |

## Palette upload and deformation

| Area | Bevy / game-engine | Godot | Evidence boundary |
|---|---|---|---|
| Palette identity | Patched Bevy keys a shared palette by the ordered joint entities **and** inverse-bindpose asset. Mesh users share it; independent actors do not merely because they use the same model. Changed joints update CPU staging entries. | `register_skin` reuses a binding for the same `Skin` within one `Skeleton3D`. Skeleton updates are dirty-gated; the renderer uploads the whole dirty skeleton buffer, not individual bone spans. | Neither source trace establishes relative CPU/GPU cost. |
| Upload | `prepare_skins` writes the complete nonempty Bevy staging buffer each frame, even when only changed joints wrote CPU entries. | Dirty skeleton upload is deferred until the renderer's dirty-resource update. Repeated unchanged setter writes can still make it dirty. | No byte counts or frame captures. |
| GPU deformation | Bevy mesh and prepass vertex shaders call `skin_model`; shadow specialization follows the prepass path. | Dirty mesh instances are queued only when visible for main/shadow/SDFGI work, then updated before shadows; its compute deformation output is reused by mesh surfaces/passes. | Source establishes the shader paths, not actual invocation counts or a fixed multiply of skinning cost. |

Solarityclient also skins in its M2 and shadow vertex shaders; its CPU bone arrays are not a compute-skinned vertex cache.

## Scope decision

No flat-array conversion is recommended or implemented. Both Bevy and Godot share palettes only under their own skeleton/binding identities; their different CPU layouts are not enough to choose an engine or architecture. Runtime costs remain unknown until equivalent scenes, visibility, skins, and pass settings are measured.

## Sources

- `/home/osso/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy_animation-0.19.0/src/lib.rs:160-208,1030-1097` — upstream target/player components and parallel target evaluation.
- `/syncthing/Sync/Projects/world-of-osso/bevy-patches/bevy_transform/src/systems.rs:38-86,105-113,659-678` — changed-tree propagation and equality suppression at `20ed24db`.
- `/syncthing/Sync/Projects/world-of-osso/bevy-patches/bevy_pbr/src/render/skin.rs:197-266,294-415,417-485` — complete staging upload, changed-joint writes, palette key, sharing and lifetime at `20ed24db`.
- `src/rendering/model/animation/bevy_player.rs:17-80` and `src/rendering/model/animation/lod.rs:1-93` — game-engine joint bindings, clock-continuing NPC LOD at `c3e9be93`.
- `/home/osso/Repos/godot-animation-comparison/4.7.2-stable/scene/3d/skeleton_3d.h:99-176` and `scene/3d/skeleton_3d.cpp:845-900,940-961,1027-1051,1062-1135` — `LocalVector<Bone>`, parent/nested-set representation, setter dirtying, and evaluation at Godot pin `ed1daf0bf001b61586d9930840f2f1394092c079`.
- `/home/osso/Repos/godot-animation-comparison/4.7.2-stable/servers/rendering/renderer_rd/storage_rd/utilities.cpp:252-258` and `servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp:1118-1240,2490-2506` — dirty-skeleton upload plus dirty/version-gated compute deformation into per-surface buffers.
- `/home/osso/Repos/godot-shadow-comparison/4.7.2-stable/renderer_scene_cull.cpp:3278-3289,3464-3469` — mesh-instance visibility admission and update before shadow rendering.
- `/home/osso/Repos/solarityclient/crates/asset/src/model/animation/mod.rs:198-220`, `crates/rendering/src/model/m2_animation/pose.rs:35-55,243-335,479-510`, and `crates/runtime/src/application/terrain_frame/m2.rs:2900-2916,3145-3166` — flat bone/pose representation and recomposition before final frustum rejection at `f5f5f4a81e5c11241f4c80c117e5dfe9b587dcee`.
- `src/rendering/model/retail_m2_material.rs:62-69`, `/syncthing/Sync/Projects/world-of-osso/bevy-patches/bevy_pbr/src/render/mesh.wgsl:37-55`, `prepass.wgsl:68-91`, and `render/light.rs:2455-2484` — Bevy material/shader pass boundary.

- `/home/osso/Repos/solarityclient/crates/rendering/src/shader/m2_spirv/source/m2.vert.glsl:70-88` and `crates/rendering/src/shader/shadow_spirv/source/shadow.vert.glsl:32-50` — per-pass vertex skinning.

## See Also

- [[solarityclient-performance-comparison]] — broader frame-cost comparison and corrected placement claim.
- [[movement-performance]] — local measurement method and results.
- [[bevy-godot-shadow-comparison]] — directional-shadow source comparison.
- [[character-rendering]] — project M2 and skinning context.
