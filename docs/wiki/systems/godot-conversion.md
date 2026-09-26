# Godot conversion

The Godot replacement remains an incomplete client-host experiment, not a completed migration. Recent native work adds M2 preview loading, a skeletal mesh conversion path, BLP albedo loading, deterministic sequence attachment, and fixed preview framing; it has no current integrated runtime GREEN because the portable UI-toolkit `WidgetDef` macro anchors mismatch blocks the build. Full feature, exact UI behavior, and visual appearance remain acceptance requirements.

## Bootstrap and workspace boundary

- Godot 4.7.2 project configuration starts `scenes/client.tscn`; `game_engine.gdextension` declares the Rust debug/release Linux libraries.
- `GameClient` is the exposed native `Node3D`. Historical `extension_smoke.gd` registered, instantiated, and attached it to a Godot scene tree. That is not current integrated proof; the former editor/import question also remains unresolved.
- `godot/` now contains `core`, `network`, `rust`, and `ui-model`. The workspace patches the shared protocol, UI macros, and local Taffy implementation.
- `godot/network` is only a dependency-level headless transport starting point: Bevy state, Lightyear, Replicon, and `shared`. `shared` still brings Bevy camera/mesh dependencies into the Godot workspace, so neither a Bevy-free client nor networking migration is proven.

## Native asset and preview boundary

`game-engine-core` remains Bevy-free authored-byte parsing for M2/skin/skeleton, BLP RGBA8, ADT companions, WDT, and WMO. It excludes CASC/file resolution, Godot conversion, rendering, and playback.

The GDExtension converts an M2 plus its `00.skin` and optional `.skel` into one indexed `ArrayMesh` per batch, with converted positions/normals/UVs and four bone indices/weights. It creates a `Skeleton3D`, `Skin`, and bound `MeshInstance3D` nodes. A model with sequences receives `WowAnimationPlayer`, which samples local pivot-relative poses, supports explicit loop/non-loop selection, and crossfades with the authored blend time subject to a 150 ms movement minimum.

BLP mip-0 RGBA8 becomes a Godot `Image`. Only ordinary type-0 texture FDIDs found under `data/textures` become an albedo texture. The material mapping is limited to unshaded, double-sided, and selected alpha/add/multiply modes. Replaceable character textures, compositing, multi-texture/layer behavior, shader effects, colour/lighting equivalence, and rendered material fidelity remain unresolved; unavailable texture FDIDs are reported rather than substituted.

`GameClient.load_model_scene` imports a model before replacing its previous preview, derives mesh bounds, and attaches a fixed 60-degree camera plus shadowed directional light. This is a model-preview helper, not game camera/light/sky/world streaming behavior.

## Capability and proof matrix

| Capability | Exact current implementation | Proof level / limit |
| --- | --- | --- |
| Extension bootstrap | `GameClient` native `Node3D`; historical scene-tree smoke. | Historical runtime GREEN only. Current build blocked by UI-toolkit `WidgetDef` macro-anchor mismatch; no integrated runtime GREEN. |
| Parser core | M2/BLP/ADT/WDT/WMO bytes to data only. | Seven parser fixtures passed before later parser edits; current revision unverified. |
| Native M2 geometry | Skinned indexed `ArrayMesh` batches, skeleton rest/binds, Godot node tree. | `m2_assets.gd` specifies HD 216-bone, indexed and weighted geometry assertions; not run at current revision. No visual proof. |
| BLP/material subset | RGBA8 image; one type-0 FDID albedo route; partial blend/flag mapping. | Script specifies torch-albedo and BLP-pixel assertions; not run. Material fidelity and unsupported texture modes remain open. |
| Animation attachment | `WowAnimationPlayer` attaches when sequences exist and writes skeleton poses. | Asset script only checks attachment. Playback, crossfade, deformation, and visual parity unproven at integrated revision. |
| Preview scene | Bounds-driven camera/light and successful-import-only replacement. | `model_scene.gd` specifies framing and failed-import preservation; not run. Not a gameplay scene. |
| UI model/projection | Sibling model extraction; local Godot layout projection is unfinished. | Two earlier sibling model tests green; current macro mismatch blocks integration. No projection or exact visual/interaction evidence. |
| Transport/gameplay | Headless Bevy-dependent transport manifest only. | No Godot lifecycle, connection, replication/state application, login-to-world, collision, equipment, or gameplay proof. |
| Tooling and parity | No Godot automation, screenshots, diagnostics, CLI/IPC, audio, debug scenes, or complete UI workflows. | Open. No milestone completion claim. |

## Sources

- [Godot conversion specification](../../specs/godot-conversion.md) — acceptance target and current capability/proof matrix.
- [Godot workspace](../../godot/Cargo.toml) — members and local dependency patches.
- [Network manifest](../../godot/network/Cargo.toml) — current headless transport dependencies.
- [Godot project](../../godot/project.godot) — project runtime configuration.
- [GDExtension declaration](../../godot/game_engine.gdextension) — extension ABI and library paths.
- [Rust extension](../../godot/rust/src/lib.rs) — `GameClient` registration and preview replacement.
- [Preview helpers](../../godot/rust/src/scene.rs) — bounds, camera, and light helper behavior.
- [Native assets](../../godot/rust/src/assets/mod.rs) — M2/BLP conversion and supported material subset.
- [Native animation](../../godot/rust/src/animation/mod.rs) — skeleton-pose playback and transition behavior.
- [Model-scene script](../../godot/tests/model_scene.gd) — unexecuted preview assertions.
- [M2-assets script](../../godot/tests/m2_assets.gd) — unexecuted native asset assertions.
- [Sibling UI core registry](../../../../ui-toolkit-godot-conversion/core/src/registry.rs) — extracted frame/model registry boundary.

## See Also

- [[asset-pipeline]] — reusable local-CASC asset boundary.
- [[ui-system]] — existing UI behavior to preserve.
- [[rendering-pipeline]] — existing client rendering behavior to replace.
