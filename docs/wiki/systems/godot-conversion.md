# Godot conversion

The Godot replacement remains an incomplete client-host experiment, not a completed migration. Godot is authorized to own rendering, UI, scenes, and gameplay while Bevy remains a transport-only headless networking worker. Native work now includes M2 preview loading, a limited ADT geometry adapter, and an account host route through that worker. None establishes user-facing login, terrain rendering, or full conversion parity.

## Bootstrap and workspace boundary

- Godot 4.7.2 project configuration starts `scenes/client.tscn`; `game_engine.gdextension` declares the Rust debug/release Linux libraries.
- `GameClient` is the exposed native `Node3D`. Historical `extension_smoke.gd` registered, instantiated, and attached it to a Godot scene tree. That is not current integrated proof; the former editor/import question also remains unresolved.
- `godot/` now contains `core`, `network`, `rust`, and `ui-model`. The workspace patches the shared protocol, UI macros, and local Taffy implementation.
- `godot/network` contains an actual headless Bevy-state/Lightyear/Replicon transport bridge, not only workspace dependencies. This is the authorized Bevy boundary; Godot owns rendering/UI/scenes/gameplay. `shared` still brings Bevy camera/mesh dependencies into the Godot workspace. Network verification ran 4/4 checks GREEN at `73339584`; later network work only re-exported `WireMessage` for typed host decoding.

## Portable UI and native login boundary

Sibling `ui-toolkit-core` commit `70db707` removes all Bevy dependencies. Its portable texture representation is `DynamicTextureId` plus RGBA8 registry data and atlas `PixelRect` arrays. Agent46 reports its two targeted model tests GREEN; main has not independently verified that result. This does not prove a Godot projection or UI parity.

Main startup wires `ui::RegistryUi` and `create_login_ui`. Separately, `GameClient.connect_account` resolves the realm, starts `NetworkBridge`, and sends typed login/register requests through `AuthChannel`; `account_state` exposes session screen, feedback, roster count, units, and reply receipt. Incoming account messages pass through `Session`; polling only updates login status and emits `screen_requested`. It does not construct or route to character-select, creation, loading, or world scenes. `account_failure.gd` is RED (exit 1) against an old native binary that lacks `connect_account`; real-server GREEN remains pending. This is not a claim that a user-facing login UI is connected.

## Native asset and preview boundary

`game-engine-core` remains Bevy-free authored-byte parsing for M2/skin/skeleton, BLP RGBA8, ADT companions, WDT, and WMO. It excludes CASC/file resolution, Godot conversion, rendering, and playback.

The GDExtension converts an M2 plus its `00.skin` and optional `.skel` into one indexed `ArrayMesh` per batch, with converted positions/normals/UVs and four bone indices/weights. It creates a `Skeleton3D`, `Skin`, and bound `MeshInstance3D` nodes. A model with sequences receives `WowAnimationPlayer`, which samples local pivot-relative poses, supports explicit loop/non-loop selection, and crossfades with the authored blend time subject to a 150 ms movement minimum.

BLP mip-0 RGBA8 becomes a Godot `Image`. Only ordinary type-0 texture FDIDs found under `data/textures` become an albedo texture. The material mapping is limited to unshaded, double-sided, and selected alpha/add/multiply modes. Replaceable character textures, compositing, multi-texture/layer behavior, shader effects, colour/lighting equivalence, and rendered material fidelity remain unresolved; unavailable texture FDIDs are reported rather than substituted.

`GameClient.load_model_scene` imports a model before replacing its previous preview, derives mesh bounds, and attaches a fixed 60-degree camera plus shadowed directional light. This is a model-preview helper, not game camera/light/sky/world streaming behavior.

`WowTerrainLoader.load_adt_geometry` reads one root ADT and emits one unmaterialed `MeshInstance3D` per nonempty chunk. It carries authored positions, normals, vertex colors, UVs, holes, and reversed triangle winding only. It does not load texture companions/materials, objects, water, collision, or world streaming; it depends on the pending core geometry API from ADT52.

## Capability and proof matrix

The [detailed Godot parity matrix](../../specs/godot-parity-matrix.md) inventories every existing feature contract by capability. It is tracking only: source specifications remain authoritative, and no parser/core/transport result closes a user-visible runtime row.

| Capability | Exact current implementation | Proof level / limit |
| --- | --- | --- |
| Extension bootstrap | `GameClient` native `Node3D`; historical scene-tree smoke. | Historical runtime GREEN only; no current integrated native-runtime proof. |
| Parser core | M2/BLP/ADT/WDT/WMO bytes to data only. | Verifier48: 7/7 parser fixtures GREEN, timing-qualified source evidence because concurrent geometry edits followed. |
| Native ADT geometry | `WowTerrainLoader.load_adt_geometry` builds raw chunk meshes with positions, normals, vertex colors, UVs, hole topology, and reversed winding. | `terrain_geometry.gd` is RED because the old native binary lacks `WowTerrainLoader`; it also awaits ADT52's core geometry API. No terrain materials, objects, water, collision, or streaming proof. |
| Native M2 geometry | Skinned indexed `ArrayMesh` batches, skeleton rest/binds, Godot node tree. | `m2_assets.gd` specifies HD 216-bone, indexed and weighted geometry assertions; not run at current revision. No visual proof. |
| BLP/material subset | RGBA8 image; one type-0 FDID albedo route; partial blend/flag mapping. | Script specifies torch-albedo and BLP-pixel assertions; not run. Material fidelity and unsupported texture modes remain open. |
| Animation attachment | `WowAnimationPlayer` attaches when sequences exist and writes skeleton poses. | Asset script only checks attachment. Playback, crossfade, deformation, and visual parity unproven at integrated revision. |
| Preview scene | Bounds-driven camera/light and successful-import-only replacement. | `model_scene.gd` specifies framing and failed-import preservation; not run. Not a gameplay scene. |
| UI model/projection | Bevy-free sibling frame/layout/widget/atlas/screen/registry model; main startup wires `ui::RegistryUi` and `create_login_ui`. | Agent46 reports two sibling model tests GREEN for `70db707`; main has not independently verified. No projection or exact visual/interaction evidence. |
| Account host | `GameClient.connect_account`/`account_state` route typed auth through `NetworkBridge` and `Session`; `poll_account` emits `screen_requested` and updates login status. | `account_failure.gd` RED exit 1 against an old binary lacking `connect_account`; real-server rejection GREEN pending. No actual character-select/world scene routing or user-facing login-UI connection. |
| Transport/gameplay | Actual headless Bevy-state/Lightyear/Replicon transport bridge. It remains transport-only; Godot owns rendering/UI/scenes/gameplay. | Network verifier: 4/4 GREEN at `73339584`; later change only added `WireMessage` re-export. Account route is source evidence, not real-server proof. No login-to-world state application, collision, equipment, gameplay, or parity proof. |
| Build/readability | Core warnings and network readability findings are tracked work. | Verifier49 assigned core warnings to ADT52/M254. Network readability-audit findings remain unfixed. |
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
- [Account failure script](../../godot/tests/account_failure.gd) — old-binary RED and pending real-server rejection assertion.
- [Terrain geometry script](../../godot/tests/terrain_geometry.gd) — pending native ADT geometry assertions.
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
