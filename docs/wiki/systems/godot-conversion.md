# Godot conversion

The Godot replacement remains an incomplete client-host experiment, not a completed migration. Godot is authorized to own rendering, UI, scenes, and gameplay while Bevy remains a transport-only headless networking worker. At `ac02b9a0`, the native extension builds and has bounded runtime proof for authored login control names, raw ADT geometry, and rejected-account status feedback; M2-asset proof is agent-reported. None establishes user-facing login, terrain rendering, or full conversion parity.

## Bootstrap and workspace boundary

- Godot 4.7.2 project configuration starts `scenes/client.tscn`; `game_engine.gdextension` declares the Rust debug/release Linux libraries.
- `GameClient` is the exposed native `Node3D`. Historical `extension_smoke.gd` registered, instantiated, and attached it to a Godot scene tree. That is not current integrated proof; the former editor/import question also remains unresolved.
- `godot/` now contains `core`, `network`, `rust`, and `ui-model`. The workspace patches the shared protocol, UI macros, and local Taffy implementation.
- `godot/network` contains an actual headless Bevy-state/Lightyear/Replicon transport bridge, not only workspace dependencies. This is the authorized Bevy boundary; Godot owns rendering/UI/scenes/gameplay. `shared` still brings Bevy camera/mesh dependencies into the Godot workspace. Network verification ran 4/4 checks GREEN at `73339584`; later network work only re-exported `WireMessage` for typed host decoding.

## Portable UI and native login boundary

Sibling `ui-toolkit-core` commit `70db707` removes all Bevy dependencies. Its portable texture representation is `DynamicTextureId` plus RGBA8 registry data and atlas `PixelRect` arrays. Agent46 reports its two targeted model tests GREEN; main has not independently verified that result. This does not prove a Godot projection or UI parity.

Main startup wires `ui::RegistryUi` and `create_login_ui`; `client_login.gd` exits 0 at `ac02b9a0` with the authored credential control names. Separately, `GameClient.connect_account` resolves the realm, starts `NetworkBridge`, and sends typed login/register requests through `AuthChannel`; `account_state` exposes session screen, feedback, roster count, units, and reply receipt. Incoming account messages pass through `Session`; polling only updates login status and emits `screen_requested`. It does not construct or route to character-select, creation, loading, or world scenes. `account_failure.gd` exits 0 at `ac02b9a0`: the local UDP server on port 5000 rejected incorrect `admin` credentials and native account status received that feedback. This is not a claim that a user-facing login UI is connected.

## Native asset and preview boundary

`game-engine-core` remains Bevy-free authored-byte parsing for M2/skin/skeleton, BLP RGBA8, ADT companions, WDT, and WMO. It excludes CASC/file resolution, Godot conversion, rendering, and playback.

The GDExtension converts an M2 plus its `00.skin` and optional `.skel` into one indexed `ArrayMesh` per batch, with converted positions/normals/UVs and four bone indices/weights. It creates a `Skeleton3D`, `Skin`, and bound `MeshInstance3D` nodes. A model with sequences receives `WowAnimationPlayer`, which samples local pivot-relative poses, supports explicit loop/non-loop selection, and crossfades with the authored blend time subject to a 150 ms movement minimum.

BLP mip-0 RGBA8 becomes a Godot `Image`. Only ordinary type-0 texture FDIDs found under `data/textures` become an albedo texture. The material mapping is limited to unshaded, double-sided, and selected alpha/add/multiply modes. Replaceable character textures, compositing, multi-texture/layer behavior, shader effects, colour/lighting equivalence, and rendered material fidelity remain unresolved; unavailable texture FDIDs are reported rather than substituted.

`GameClient.load_model_scene` imports a model before replacing its previous preview, derives mesh bounds, and attaches a fixed 60-degree camera plus shadowed directional light. `model_scene.gd` exits 1 at `ac02b9a0` because the login canvas covers the imported model. Commit `5ca2dc8d` hides that canvas after a successful import, but a rebuilt native binary has not yet proved the correction. This is a model-preview helper, not game camera/light/sky/world streaming behavior.

`WowTerrainLoader.load_adt_geometry` reads one root ADT and emits one unmaterialed `MeshInstance3D` per nonempty chunk. It carries authored positions, normals, vertex colors, UVs, holes, and reversed triangle winding only. `terrain_geometry.gd` exits 0 at `ac02b9a0` with 256 chunks and those raw-channel/topology assertions. It does not load texture companions/materials, objects, water, collision, or world streaming.

## Capability and proof matrix

The [detailed Godot parity matrix](../../specs/godot-parity-matrix.md) inventories every existing feature contract by capability. It is tracking only: source specifications remain authoritative, and no parser/core/transport result closes a user-visible runtime row.

| Capability | Exact current implementation | Proof level / limit |
| --- | --- | --- |
| Extension bootstrap | `GameClient` native `Node3D`; historical scene-tree smoke. | Historical runtime GREEN only; no current integrated native-runtime proof. |
| Parser core | M2/BLP/ADT/WDT/WMO bytes to data only. | Verifier48: 7/7 parser fixtures GREEN, timing-qualified source evidence because concurrent geometry edits followed. |
| Native ADT geometry | `WowTerrainLoader.load_adt_geometry` builds raw chunk meshes with positions, normals, vertex colors, UVs, hole topology, and reversed winding. | `terrain_geometry.gd` exits 0 at `ac02b9a0`: 256 chunks plus authored heights/UVs/colors, holes, and Godot winding. No terrain materials, objects, water, collision, or streaming proof. |
| Native M2 geometry | Skinned indexed `ArrayMesh` batches, skeleton rest/binds, Godot node tree. | Agent39 reports `m2_assets.gd` GREEN after `97d97af1`/`5297d394` on the same `.so`: 216 bones, 113 batches, 37,813 vertices, 147,966 indices, torch BLP, and no leaks. Reported evidence only; no visual proof. |
| BLP/material subset | RGBA8 image; one type-0 FDID albedo route; partial blend/flag mapping. | Agent39 reports torch BLP assertion GREEN on the same `.so`. Material fidelity and unsupported texture modes remain open. |
| Animation attachment | `WowAnimationPlayer` attaches when sequences exist and writes skeleton poses. | Agent39 reports asset assertions GREEN; playback, crossfade, deformation, and visual parity remain unproven. |
| Preview scene | Bounds-driven camera/light and successful-import-only replacement. | `model_scene.gd` exits 1 at `ac02b9a0`: login canvas covers the model. `5ca2dc8d` hides it after successful import; GREEN awaits rebuild. Not a gameplay scene. |
| UI model/projection | Bevy-free sibling frame/layout/widget/atlas/screen/registry model; main startup wires `ui::RegistryUi` and `create_login_ui`. | Agent46 reports two sibling model tests GREEN for `70db707`; agent42's projection test remains RED for wrong positions/input and requires actual keyboard exercise. No visual parity. Logged texture-file messages are cwd preflight only: source paths remain retained and native loader still loads them; they do not show artwork absence. |
| Account host | `GameClient.connect_account`/`account_state` route typed auth through `NetworkBridge` and `Session`; `poll_account` emits `screen_requested` and updates login status. | `account_failure.gd` exits 0 at `ac02b9a0`: local UDP 5000 rejected wrong `admin` credentials and native status feedback returned. No actual character-select/world scene routing or user-facing login-UI connection. |
| Transport/gameplay | Actual headless Bevy-state/Lightyear/Replicon transport bridge. It remains transport-only; Godot owns rendering/UI/scenes/gameplay. | Network verifier: 4/4 GREEN at `73339584`; the account-rejection path is runtime proof only for typed auth/status feedback. No login-to-world state application, collision, equipment, gameplay, or parity proof. |
| Build/readability | The native extension builds at `ac02b9a0`; agent58 reports ownership fixes. | Build exits 0 with 27 `game-engine-core` warnings and 2 native-animation dead-code warnings. Agent58's ownership work has no supplied proof; network readability-audit findings remain unfixed. |
| Character-select model | `CharacterSelectModel` has pure model tests. | `a828a028`/`fcd34042`: two pure tests GREEN. No native character-select scene, rendering, input, or workflow proof. |
| Tooling and parity | No Godot automation, screenshots, diagnostics, CLI/IPC, audio, debug scenes, or complete UI workflows. | Open. No milestone completion claim. |

## Sources

- [Godot conversion specification](../../specs/godot-conversion.md) — acceptance target and current capability/proof matrix.
- [Godot feature parity matrix](../../specs/godot-parity-matrix.md) — all preexisting feature contracts and current conversion status.
- [Godot workspace](../../godot/Cargo.toml) — members and local dependency patches.
- [Network manifest](../../godot/network/Cargo.toml) — current headless transport dependencies.
- [Godot project](../../godot/project.godot) — project runtime configuration.
- [GDExtension declaration](../../godot/game_engine.gdextension) — extension ABI and library paths.
- [Rust extension](../../godot/rust/src/lib.rs) — `GameClient` registration, account host, and preview replacement.
- [Account host](../../godot/rust/src/account.rs) — typed account requests, worker event handling, and `Session` effects.
- [Session decisions](../../godot/session/src/lib.rs) — headless login/roster state transitions.
- [Native terrain](../../godot/rust/src/terrain/mod.rs) — raw ADT chunk mesh conversion boundary.
- [Account failure script](../../godot/tests/account_failure.gd) — `ac02b9a0` real-server rejection/status-feedback GREEN.
- [Terrain geometry script](../../godot/tests/terrain_geometry.gd) — `ac02b9a0` 256-chunk raw-geometry GREEN.
- `/tmp/claude/godot-integrated-build-ac02b9a0.log` — build exit 0 and 27 core plus 2 animation warnings.
- `/tmp/claude/godot-client_login-ac02b9a0.log` — named-login-control GREEN; texture-file messages are cwd-preflight only.
- `/tmp/claude/godot-model_scene-ac02b9a0.log` — login-canvas occlusion RED before `5ca2dc8d`.
- [Preview helpers](../../godot/rust/src/scene.rs) — bounds, camera, and light helper behavior.
- [Native assets](../../godot/rust/src/assets/mod.rs) — M2/BLP conversion and supported material subset.
- [Native animation](../../godot/rust/src/animation/mod.rs) — skeleton-pose playback and transition behavior.
- [Model-scene script](../../godot/tests/model_scene.gd) — `ac02b9a0` canvas-occlusion RED; `5ca2dc8d` correction awaits rebuild.
- [M2-assets script](../../godot/tests/m2_assets.gd) — agent39-reported GREEN on the `ac02b9a0` native library after `97d97af1`/`5297d394`.
- [Sibling UI core registry](../../../../ui-toolkit-godot-conversion/core/src/registry.rs) — extracted frame/model registry boundary.

## See Also

- [[asset-pipeline]] — reusable local-CASC asset boundary.
- [[ui-system]] — existing UI behavior to preserve.
- [[rendering-pipeline]] — existing client rendering behavior to replace.
