# Godot client replacement

Replace the Bevy client engine with Godot while retaining reusable Rust and preserving existing feature behavior and UI appearance. The conversion is an isolated experiment; a partial renderer or launchable scene does not satisfy completion.

## What it must do

### Client engine and assets

- [ ] Godot owns client rendering, scenes, input, UI, audio and lifecycle; replaced Bevy engine paths are removed, not kept as runtime fallbacks.
- [ ] Retain existing local-CASC asset resolution and supported M2/BLP/ADT/WMO parsing semantics, materials, characters, animation, equipment and world streaming.
- [ ] Preserve existing camera, collision, light/sky/shadow, particle and sound behavior.

### Application and UI parity

- [ ] Preserve login, registration, character selection/creation/deletion, loading, world entry and reconnect behavior against the existing authoritative server.
- [ ] Preserve all existing gameplay requests, replication, state application and UI workflows specified by existing feature contracts.
- [ ] Preserve UI screen appearance, authored assets, layouts, text, focus, editing, scrolling, layering, input and named-frame automation.
- [ ] Preserve CLI, IPC, screenshot, scene/UI-tree diagnostics, JavaScript automation and debug scenes.

### Evidence

- [ ] Record a complete feature-parity matrix, with missing/blocked/unverified capabilities explicit.
- [ ] Exercise real client/server workflows and inspect matching visual/interaction fixtures.
- [ ] Record matched frame-time, loading and memory evidence without inferring performance from implementation shape.
- [ ] Pass relevant behavioral/integration tests and final Rust formatting/checks at the integrated revision.

## How it works

- [Existing UI contracts](registry-bevy-ui.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [Networking boundary](../wiki/systems/networking.md)

## Implementation inventory

- `godot/project.godot` — Godot project configuration and bootstrap scene.
- `godot/game_engine.gdextension` — Rust extension ABI/library declaration.
- `godot/rust/` — native `GameClient`, M2/BLP preview conversion, skeletal sequence attachment, fixed preview scene helpers, and an unfinished UI layout projection. It does not yet provide a game renderer or scene implementation.
- `godot/network/` — workspace-only headless transport starting point. It currently depends on Bevy state/Lightyear/Replicon and `shared`; through `shared`, Bevy camera/mesh dependencies still enter the Godot workspace. This is not proof of full Bevy removal.
- `godot/core/` — `game-engine-core`, a Bevy-free authored-byte parser boundary: M2/skin/skeleton data, BLP RGBA8, ADT companions, WDT, and WMO raw data. It excludes CASC/file resolution, Godot conversion, rendering, and playback.
- `../ui-toolkit-godot-conversion/core/` — sibling `ui-toolkit-core`: reusable frame/layout/widget/atlas/screen/registry model. It is not a Godot projection and currently still uses Bevy asset/image/math crates.

## Tests asserting this spec

- `godot/tests/extension_smoke.gd` — native client extension loads and joins a real Godot scene tree; historical runtime smoke only.
- `godot/tests/model_scene.gd` — calls `GameClient.load_model_scene` on the torch fixture; asserts nonempty mesh bounds, an active camera facing the bounds centre, and failed replacement preservation. Present but unexecuted at the current revision.
- `godot/tests/m2_assets.gd` — asserts HD 216-bone/skinned indexed batches, identity rest palette, native animation attachment, one torch albedo texture, BLP pixels, and selected error paths. Present but unexecuted at the current revision.
- `godot/core/tests/asset_parsing.rs` — seven fixture behaviors for M2, BLP, ADT, WDT, and WMO. All seven passed before final public-visibility and strict-skeleton-validation edits; current-commit status is unverified.
- `../ui-toolkit-godot-conversion/core/tests/model.rs` — two targeted model tests green: registry mutation/layout publication and named-frame screen diff. No native host or visual proof.

## Known gaps (current cycle)

| Capability | Current boundary | Proof status |
| --- | --- | --- |
| Capability | Current boundary | Proof status |
| --- | --- | --- |
| Extension bootstrap | `GameClient` is a native `Node3D`; historical smoke attached it to a Godot scene tree. | Historical runtime smoke only. Current integrated build is blocked before runtime by the sibling UI-toolkit `WidgetDef` macro-anchor mismatch; the old editor/import question also remains unresolved. |
| Parser core | Byte-to-data APIs for M2/BLP/ADT/WDT/WMO only. | Historical 7/7 fixture pass, unverified after final parser edits. |
| Native M2 geometry | `WowAssetLoader` reads `.m2`, `00.skin`, optional `.skel`; emits one indexed `ArrayMesh` per batch with converted positions/normals/UVs and four bone indices/weights, `Skeleton3D`, `Skin`, and `MeshInstance3D` bindings. | `m2_assets.gd` specifies an HD 216-bone, indexed/skinned-fixture check; unexecuted at current revision. No rendered-fidelity evidence. |
| Native BLP and material subset | BLP mip-0 RGBA8 becomes a Godot `Image`; type-0 FDID textures under `data/textures` become albedo textures. Material flags map only unshaded, double-sided, and selected alpha/add/multiply modes; absent files are reported as FDIDs. | `m2_assets.gd` specifies one torch albedo and BLP-pixel check; unexecuted. Replaceable textures, character compositing, multi-texture/layer rules, shader effects, colour/lighting, and material visual fidelity remain unresolved. |
| Skeleton animation attachment | Models with sequences receive native `WowAnimationPlayer`; mesh batches bind to its `Skeleton3D`. | Asset script checks attachment, not playback, transitions, or rendered deformation; unexecuted. Animation parity remains open. |
| Preview scene | `load_model_scene` swaps a successfully imported model, derives bounds, and attaches a fixed camera/directional light. | `model_scene.gd` specifies framing and failed-swap behavior; unexecuted. This is a model preview, not game camera, lighting, scene, or world streaming. |
| Reusable UI model | Sibling registry/layout/screen model only. The local Godot layout projection is unfinished. | Two earlier sibling model tests green, but current integration is compile-blocked by `WidgetDef` macro anchors. No Godot projection or parity proof. |
| Client networking and gameplay | `godot/network` only establishes workspace dependencies for headless Bevy-state/Lightyear/Replicon transport; no Godot lifecycle or state application exists. Its `shared` dependency still brings Bevy camera/mesh dependencies into the workspace. Login-to-world, collision, equipment, and gameplay migration are absent. | No runtime proof; this does not establish a Bevy-free client. |
| UI parity | No complete Godot frame projection or screen/workflow/visual/input parity. | Open; exact appearance and behavior remain conversion acceptance requirements. |
| Tooling | No Godot automation, screenshots, diagnostics, CLI/IPC, audio, or debug-scene parity. | Open. |

- [ ] Preserve existing Lightyear protocol semantics while replacing the client host; the existing headless Bevy worker does not authorize a server/protocol redesign.
- [ ] Build and prove Godot projection before claiming any UI parity.
- [ ] Exercise actual client/server workflows and visual fixtures before checking any final parity requirement.

## Out of scope

Unrequested server/protocol redesign, new gameplay features, production deployment, and changes to host safety mitigations. No existing client feature is excluded from the conversion target.
