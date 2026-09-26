# Godot client replacement

Replace the Bevy client engine with Godot while retaining reusable Rust and preserving existing feature behavior and UI appearance. The conversion is an isolated experiment; a partial renderer or launchable scene does not satisfy completion.

## What it must do

### Client engine and assets

- [ ] Godot owns client rendering, scenes, input, UI, audio, lifecycle, and gameplay. Bevy remains only in the headless networking worker for transport; it is not a rendering/UI/scene/gameplay fallback.
- [ ] Retain existing local-CASC asset resolution and supported M2/BLP/ADT/WMO parsing semantics, materials, characters, animation, equipment and world streaming.
- [ ] Preserve existing camera, collision, light/sky/shadow, particle and sound behavior.

### Application and UI parity

- [ ] Preserve login, registration, character selection/creation/deletion, loading, world entry and reconnect behavior against the existing authoritative server.
- [ ] Preserve all existing gameplay requests, replication, state application and UI workflows specified by existing feature contracts.
- [ ] Preserve UI screen appearance, authored assets, layouts, text, focus, editing, scrolling, layering, input and named-frame automation.
- [ ] Preserve CLI, IPC, screenshot, scene/UI-tree diagnostics, JavaScript automation and debug scenes.

### Evidence

- [ ] Record a complete feature-parity matrix, with missing/blocked/unverified capabilities explicit. [Detailed matrix](godot-parity-matrix.md) inventories every existing feature contract; all rows remain open.
- [ ] Exercise real client/server workflows and inspect matching visual/interaction fixtures.
- [ ] Record matched frame-time, loading and memory evidence without inferring performance from implementation shape.
- [ ] Pass relevant behavioral/integration tests and final Rust formatting/checks at the integrated revision.

## How it works

- [Existing UI contracts](registry-bevy-ui.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [Networking boundary](../wiki/systems/networking.md)

## Parity inventory

[Godot feature parity matrix](godot-parity-matrix.md) tracks every existing feature specification by capability. Its source contracts remain authoritative; parser/core/transport work does not close runtime parity.

## Implementation inventory

- `godot/project.godot` — Godot project configuration and bootstrap scene.
- `godot/game_engine.gdextension` — Rust extension ABI/library declaration.
- `godot/rust/` — native `GameClient`, M2/BLP preview conversion, skeletal sequence attachment, fixed preview helpers, unfinished UI projection, an account host, and raw ADT geometry adapter. It does not yet provide game scenes or renderer parity.
- `godot/network/` — implemented headless Bevy-state/Lightyear/Replicon transport bridge. It remains the authorized transport-only worker; through `shared`, Bevy camera/mesh dependencies still enter the Godot workspace. Verifier network evidence is 4/4 GREEN at `73339584`; later work only re-exported `WireMessage` for typed host decoding.
- `godot/session/` — pure account/roster state decisions. The host owns transport, token I/O, screen projection, reset, and completion behavior.
- `godot/core/` — `game-engine-core`, a Bevy-free authored-byte parser boundary: M2/skin/skeleton data, BLP RGBA8, ADT companions, WDT, and WMO raw data. It excludes CASC/file resolution, Godot conversion, rendering, and playback. Verifier48's parser evidence is 7/7 GREEN but timing-qualified because concurrent geometry edits followed.
- `../ui-toolkit-godot-conversion/core/` — sibling `ui-toolkit-core`: Bevy-free reusable frame/layout/widget/atlas/screen/registry model. Commit `70db707` replaces Bevy texture/image types with `DynamicTextureId`, RGBA8 registry data, and atlas `PixelRect` arrays. It is not a Godot projection.

## Tests asserting this spec

- `godot/tests/extension_smoke.gd` — native client extension loads and joins a real Godot scene tree; historical runtime smoke only.
- `godot/tests/model_scene.gd` — calls `GameClient.load_model_scene` on the torch fixture; asserts nonempty mesh bounds, an active camera facing the bounds centre, and failed replacement preservation. Present but unexecuted at the current revision.
- `godot/tests/m2_assets.gd` — asserts HD 216-bone/skinned indexed batches, identity rest palette, native animation attachment, one torch albedo texture, BLP pixels, and selected error paths. Present but unexecuted at the current revision.
- `godot/core/tests/asset_parsing.rs` — seven fixture behaviors for M2, BLP, ADT, WDT, and WMO. All seven passed before final public-visibility and strict-skeleton-validation edits; current-commit status is unverified.
- `../ui-toolkit-godot-conversion/core/tests/model.rs` — two targeted model tests GREEN: registry mutation/layout publication and named-frame screen diff. Agent46 ran them for sibling commit `70db707`; main has not independently verified them. A root local macro patch fixes the former `WidgetDef` anchor blocker; no sibling fix is required. No native host or visual proof.
- `godot/tests/account_failure.gd` — RED, exit 1 against the old native binary because it lacks `GameClient.connect_account`. Its real-server rejection assertion remains pending.
- `godot/tests/terrain_geometry.gd` — RED because the old native binary lacks `WowTerrainLoader`; asserts raw 256-chunk geometry, vertex channels, holes, error paths, and Godot winding once ADT52 exposes the required core geometry API.

## Known gaps (current cycle)

| Capability | Current boundary | Proof status |
| --- | --- | --- |
| Extension bootstrap | `GameClient` is a native `Node3D`; historical smoke attached it to a Godot scene tree. | Historical runtime smoke only. No current integrated native-runtime proof; the old editor/import question remains unresolved. |
| Parser core | Byte-to-data APIs for M2/BLP/ADT/WDT/WMO only. | Verifier48: 7/7 fixture behaviors GREEN, timing-qualified source evidence because concurrent geometry edits followed. |
| Native ADT geometry | `WowTerrainLoader.load_adt_geometry` reads one root ADT and emits unmaterialed chunk meshes with positions, normals, colors, UVs, holes, and reversed winding. | `terrain_geometry.gd` RED against an old binary missing `WowTerrainLoader`; pending ADT52 core geometry API. No texture/material, object, water, collision, or streaming implementation/proof. |
| Native M2 geometry | `WowAssetLoader` reads `.m2`, `00.skin`, optional `.skel`; emits one indexed `ArrayMesh` per batch with converted positions/normals/UVs and four bone indices/weights, `Skeleton3D`, `Skin`, and `MeshInstance3D` bindings. | `m2_assets.gd` specifies an HD 216-bone, indexed/skinned-fixture check; unexecuted at current revision. No rendered-fidelity evidence. |
| Native BLP and material subset | BLP mip-0 RGBA8 becomes a Godot `Image`; type-0 FDID textures under `data/textures` become albedo textures. Material flags map only unshaded, double-sided, and selected alpha/add/multiply modes; absent files are reported as FDIDs. | `m2_assets.gd` specifies one torch albedo and BLP-pixel check; unexecuted. Replaceable textures, character compositing, multi-texture/layer rules, shader effects, colour/lighting, and material visual fidelity remain unresolved. |
| Skeleton animation attachment | Models with sequences receive native `WowAnimationPlayer`; mesh batches bind to its `Skeleton3D`. | Asset script checks attachment, not playback, transitions, or rendered deformation; unexecuted. Animation parity remains open. |
| Preview scene | `load_model_scene` swaps a successfully imported model, derives bounds, and attaches a fixed camera/directional light. | `model_scene.gd` specifies framing and failed-swap behavior; unexecuted. This is a model preview, not game camera, lighting, scene, or world streaming. |
| Reusable UI model | Sibling registry/layout/screen model plus local Godot projection startup wiring through `ui::RegistryUi` and `create_login_ui`. | Two targeted sibling model tests are GREEN. No Godot projection or parity proof. |
| Account host | `GameClient.connect_account`/`account_state` start `NetworkBridge`, send typed login/register requests through `AuthChannel`, apply `Session` effects, expose state, and emit `screen_requested` while polling. | `account_failure.gd` RED exit 1 against an old binary lacking `connect_account`; real-server GREEN pending. Polling does not create or route actual character-select, creation, loading, or world scenes. No claim that a user-facing login UI is connected. |
| Client networking and gameplay | `godot/network` contains an actual headless Bevy-state/Lightyear/Replicon transport bridge, not only workspace dependencies. It remains transport-only; Godot owns rendering/UI/scenes/gameplay. Its `shared` dependency still brings Bevy camera/mesh dependencies into the workspace. | Network verifier: 4/4 GREEN at `73339584`; later code only adds the `WireMessage` re-export. Account routing is source evidence, not real-server proof. Login-to-world state application, collision, equipment, and gameplay migration remain absent. |
| Build/readability | Core warnings and network readability findings remain tracked work. | Verifier49 assigned core warnings to ADT52/M254. Network readability-audit findings are not fixed. |
| UI parity | No complete Godot frame projection or screen/workflow/visual/input parity. | Open; exact appearance and behavior remain conversion acceptance requirements. |
| Tooling | No Godot automation, screenshots, diagnostics, CLI/IPC, audio, or debug-scene parity. | Open. |

- [ ] Preserve existing Lightyear protocol semantics while retaining the authorized headless Bevy transport worker; no server/protocol redesign is authorized.
- [ ] Exercise actual client/server workflows and visual fixtures before checking any final parity requirement.

## Out of scope

Unrequested server/protocol redesign, new gameplay features, production deployment, and changes to host safety mitigations. No existing client feature is excluded from the conversion target.
