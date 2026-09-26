# Godot conversion

The Godot replacement remains an incomplete client-host experiment, not a completed migration. Godot is authorized to own rendering, UI, scenes, and gameplay while Bevy remains a transport-only headless networking worker. `94c4e6ae` reuses the original authored `LoadingModel`, `1be11d6c` exposes it through `RegistryUi.show_loading`, and `e8845a5e` centralizes original shell construction, panel styles, and Godot model registration; no `GameClient` Loading route or world-readiness logic exists. At `d1641bc9`/`9249929e`, successful `admin`/`admin` authentication against explicitly local UDP `127.0.0.1:5000` creates native `CharacterSelectUI`, maps protocol roster data through the original `CharacterSelectModel`, and hides `LoginUI`; raw ADT and M2 proof retain their prior limits. None establishes character-select actions, a character/world scene, terrain rendering, matched visuals, or full conversion parity.

## Bootstrap and workspace boundary

- Godot 4.7.2 project configuration starts `scenes/client.tscn`; `game_engine.gdextension` declares the Rust debug/release Linux libraries.
- `GameClient` is the exposed native `Node3D`. Historical `extension_smoke.gd` registered, instantiated, and attached it to a Godot scene tree. That is not current integrated proof; the former editor/import question also remains unresolved.
- `godot/` now contains `core`, `network`, `rust`, and `ui-model`. The workspace patches the shared protocol, UI macros, and local Taffy implementation.
- `godot/network` contains an actual headless Bevy-state/Lightyear/Replicon transport bridge, not only workspace dependencies. This is the authorized Bevy boundary; Godot owns rendering/UI/scenes/gameplay. `shared` still brings Bevy camera/mesh dependencies into the Godot workspace. Network verification ran 4/4 checks GREEN at `73339584`; later network work only re-exported `WireMessage` for typed host decoding.

## Portable UI and native login boundary

Sibling `ui-toolkit-core` commit `70db707` removes all Bevy dependencies. Its portable texture representation is `DynamicTextureId` plus RGBA8 registry data and atlas `PixelRect` arrays. Agent46 reports its two targeted model tests GREEN; main has not independently verified that result. This does not prove a Godot projection or UI parity.

`94c4e6ae` reuses the original authored `LoadingModel`; `1be11d6c` exposes it through `RegistryUi.show_loading`; and `e8845a5e` centralizes original loading-bar shell construction, panel styles, and Godot model registration. Its targeted loading state/style test is GREEN for status, zone, tip, 25%/80% progress, and shell style. Native `loading_ui.gd` remains RED after `63de89d7`: PNG decoding now reaches `LoadingBarBackground` decoration rejection. Native three-slice projection is pending agent74; no native fixture GREEN or rendered visual proof follows. `LoadingModel` assigns world readiness to the host, and `GameClient` has neither that logic nor a Loading scene route.

Main startup wires `ui::RegistryUi` and `create_login_ui`. At `50cd3c63`/`33ed3a57`, native `sync_input`/`pop_action` route connect, reconnect, exit, selected-host `set_server`, and pending status. `GameClient.connect_account` resolves the realm, starts `NetworkBridge`, and sends typed login/register requests through `AuthChannel`; `account_state` exposes `Session` effects. Polling emits `screen_requested`; it does not construct or route character-select, creation, loading, or world scenes.

The historical `login_flow.gd` log identifies `3de5946b`, exits 0, and has empty stderr: the existing local UDP server on port 5000 rejects wrong `admin` credentials, with credentials supplied by the native login button and rejection projected into status. At `d1641bc9`/`9249929e`, `character_select_flow.gd` exits 0 with empty stderr: successful `admin`/`admin` auth against explicitly local UDP `127.0.0.1:5000` creates `CharacterSelectUI` using the original `CharacterSelectModel`, maps the protocol roster, and hides `LoginUI`. Empty authored UI and Back action are GREEN; the initial test assumption that EnterWorld is disabled was removed because the original authored button is enabled. `744f3e1b3` mapping tests are pure. Current `1296be4b` native build and auth→Back flow are GREEN with empty stderr: Back restores LoginUI. `b7070542` adds Frame left-press routing, but its real viewport card fixture is RED: clicking `Elara` selected `Theron`; agent68 build/fixture GREEN remains pending. `a89ab4d4` is an input readability refactor with no runtime proof yet. No positive EnterWorld/delete/create, character/world scene, background/appearance, or matched visual proof exists; campsite/menu remain unsupported. This is bounded auth-to-roster UI proof, not complete workflow or scene parity.

## Native asset and preview boundary

`game-engine-core` remains Bevy-free authored-byte parsing for M2/skin/skeleton, BLP RGBA8, ADT companions, WDT, and WMO. Reported `fb54637f` evidence is 233/233 `--lib` tests GREEN with no warnings, including concurrent ADT changes. `1a5c9045` and `aadb3597` relocate engine-dependent ADT/WMO/M2 tests into Bevy adapters; those legacy adapter tests were not compiled or executed. The core excludes CASC/file resolution, Godot conversion, rendering, and playback.

The GDExtension converts an M2 plus its `00.skin` and optional `.skel` into one indexed `ArrayMesh` per batch, with converted positions/normals/UVs and four bone indices/weights. It creates a `Skeleton3D`, `Skin`, and bound `MeshInstance3D` nodes. A model with sequences receives `WowAnimationPlayer`, which samples local pivot-relative poses, supports explicit loop/non-loop selection, and crossfades with the authored blend time subject to a 150 ms movement minimum.

BLP mip-0 RGBA8 becomes a Godot `Image`. Only ordinary type-0 texture FDIDs found under `data/textures` become an albedo texture. The material mapping is limited to unshaded, double-sided, and selected alpha/add/multiply modes. Replaceable character textures, compositing, multi-texture/layer behavior, shader effects, colour/lighting equivalence, and rendered material fidelity remain unresolved; unavailable texture FDIDs are reported rather than substituted.

`GameClient.load_model_scene` imports a model before replacing its previous preview, derives mesh bounds, and attaches a fixed 60-degree camera plus shadowed directional light. `model_scene.gd` exits 1 at `ac02b9a0` because the login canvas covers the imported model. Commit `5ca2dc8d` hides that canvas after a successful import, but a rebuilt native binary has not yet proved the correction. This is a model-preview helper, not game camera/light/sky/world streaming behavior.

`WowTerrainLoader.load_adt_geometry` reads one root ADT and emits one unmaterialed `MeshInstance3D` per nonempty chunk. It carries authored positions, normals, vertex colors, UVs, holes, and reversed triangle winding only. Historical `terrain_geometry.gd` evidence exits 0 at `ac02b9a0` with 256 chunks and those raw-channel/topology assertions. `3de5946b` retains authored ADT metadata and exposes tile/LOD parsers; `18666116` corrects fixture coordinate and companion coverage. Reported `fb54637f` pure-core evidence is 233/233 `--lib` GREEN/no warnings; legacy Bevy adapter tests were not executed. None of this loads texture companions/materials, objects, water, collision, or world streaming.

## Capability and proof matrix

The [detailed Godot parity matrix](../../specs/godot-parity-matrix.md) inventories every existing feature contract by capability. It is tracking only: source specifications remain authoritative, and no parser/core/transport result closes a user-visible runtime row.

| Capability | Exact current implementation | Proof level / limit |
| --- | --- | --- |
| Extension bootstrap | `GameClient` native `Node3D`; historical scene-tree smoke. | Historical runtime GREEN only; no current integrated native-runtime proof. |
| Parser core | M2/BLP/ADT/WDT/WMO bytes to data only. `3de5946b` retains ADT authored metadata and exposes tile/LOD parsers; `18666116` corrects fixture coordinate and companion coverage. `1a5c9045`/`aadb3597` move engine-dependent ADT/WMO/M2 fixtures to Bevy adapters. | Reported `fb54637f`: 233/233 pure-core `--lib` tests GREEN with no warnings, including concurrent ADT changes. Bevy adapter tests were not compiled/executed. Parser proof only, not conversion/runtime proof. |
| Native ADT geometry | `WowTerrainLoader.load_adt_geometry` builds raw chunk meshes with positions, normals, vertex colors, UVs, hole topology, and reversed winding. | Historical `terrain_geometry.gd` exits 0 at `ac02b9a0`: 256 chunks plus authored heights/UVs/colors, holes, and Godot winding. `fb54637f` pure-core proof does not execute Bevy adapters. No terrain materials, objects, water, collision, or streaming proof. |
| Native M2 geometry | Skinned indexed `ArrayMesh` batches, skeleton rest/binds, Godot node tree. | Agent39 reports `m2_assets.gd` GREEN after `97d97af1`/`5297d394` on the same `.so`: 216 bones, 113 batches, 37,813 vertices, 147,966 indices, torch BLP, and no leaks. Reported evidence only; no visual proof. |
| BLP/material subset | RGBA8 image; one type-0 FDID albedo route; partial blend/flag mapping. | Agent39 reports torch BLP assertion GREEN on the same `.so`. Material fidelity and unsupported texture modes remain open. |
| Animation attachment | `WowAnimationPlayer` attaches when sequences exist and writes skeleton poses. | Agent39 reports asset assertions GREEN; playback, crossfade, deformation, and visual parity remain unproven. |
| Preview scene | Bounds-driven camera/light and successful-import-only replacement. | `model_scene.gd` exits 1 at `ac02b9a0`: login canvas covers the model. `5ca2dc8d` hides it after successful import; GREEN awaits rebuild. Not a gameplay scene. |
| UI model/projection | Bevy-free sibling frame/layout/widget/atlas/screen/registry model; native projection wires `sync_input`/`pop_action`. `1251724a` projects disabled buttons and suppresses their callbacks. | `ui_projection.gd` exits 0 at `3de5946b`, empty stderr; verifier67 reran it and `login_flow.gd` PASS on an artifact timing-qualified to `d1641bc9` or `12a23693` (identical host files; tint-only difference). True viewport Unicode/Ctrl-A/backspace/focus editing; gold/resize/removal updates; disabled/callback suppression; asset/font/insets. Behavioral fixture only. RealForward+ screenshot `04aa3610` predates final colour/focus changes, so exact visual baseline is unverified. |
| Account host | `50cd3c63`/`33ed3a57` wire connect/reconnect/exit, selected-host `set_server`, pending status, and typed auth through `NetworkBridge`/`Session`. `d1641bc9`/`9249929e` create `CharacterSelectUI` with the original `CharacterSelectModel`, protocol roster mapping, and LoginUI removal after successful auth. `1296be4b` adds host `SelectChar` dispatch and routes Back; `b7070542` adds Frame left-press routing. | `character_select_flow.gd` at `d1641bc9` exits 0 with empty stderr: local `admin`/`admin` auth at UDP `127.0.0.1:5000` reaches native roster UI. Current `1296be4b` native-build and auth→Back-flow logs are GREEN with empty stderr; Back restores LoginUI. The real viewport card fixture is RED after `b7070542`: clicking `Elara` selected `Theron`; agent68 build/fixture GREEN remains pending. No positive EnterWorld/delete/create, character/world routing, or scene proof; campsite/menu remain unsupported. |
| Loading UI | `94c4e6ae` reuses original authored `LoadingModel`; `1be11d6c` exposes it through `RegistryUi.show_loading`; `e8845a5e` centralizes original shell construction, panel styles, and Godot model registration. | `e8845a5e` targeted loading state/style test is GREEN for status, zone, tip, 25%/80% progress, and shell style. Native `loading_ui.gd` remains RED after `63de89d7`: PNG decoding reaches `LoadingBarBackground` decoration rejection. Three-slice projection is pending agent74. No GameClient Loading routing, world-readiness logic, native fixture GREEN, or rendered visual proof. |
| Transport/gameplay | Actual headless Bevy-state/Lightyear/Replicon transport bridge. It remains transport-only; Godot owns rendering/UI/scenes/gameplay. | Network verifier: 4/4 GREEN at `73339584`; the account-rejection path is runtime proof only for typed auth/status feedback. No login-to-world state application, collision, equipment, gameplay, or parity proof. |
| Build/readability | Native build preceded current fixture runs. `a89ab4d4` refactors UI input for readability. | Latest `login_flow.gd` and `ui_projection.gd` logs identify `3de5946b`, both exit 0 with empty stderr. They do not provide a standalone current compiler-warning report or runtime proof for `a89ab4d4`; agent68 testing remains pending. |
| Character-select UI | `d1641bc9`/`9249929e` construct native `CharacterSelectUI` from the original `CharacterSelectModel`, map protocol roster data, and hide LoginUI. It includes empty authored UI and Back action; EnterWorld remains enabled as in the original authored button. `1296be4b` adds host `SelectChar` dispatch; `b7070542` adds Frame left-press routing. | `character_select_flow.gd` and `character_select_ui.gd` are GREEN with empty stderr; `744f3e1b3` mapping tests are pure. Current `1296be4b` native-build and auth→Back-flow logs are GREEN with empty stderr; Back restores LoginUI. The real viewport card fixture is RED after `b7070542`: clicking `Elara` selected `Theron`; agent68 build/fixture GREEN remains pending. No positive EnterWorld/delete/create, character/world scene, background/appearance, or matched visual proof. |
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
- `fb54637f` report — 233/233 pure-core `--lib` GREEN/no warnings; Bevy adapter fixtures excluded and unexecuted.
- [Login-flow script](../../godot/tests/login_flow.gd) — historical credential-routing/rejection fixture.
- [Character-select-flow script](../../godot/tests/character_select_flow.gd) — successful local auth, protocol-roster mapping, and LoginUI replacement fixture.
- [Character-select UI script](../../godot/tests/character_select_ui.gd) — empty authored UI and Back-action fixture.
- [UI projection script](../../godot/tests/ui_projection.gd) — current native editing/layout/action fixture.
- [Loading model](../../godot/ui-model/src/lib.rs) — authored loading model and host-owned readiness boundary.
- [Loading model test](../../godot/ui-model/tests/loading.rs) — reported status/zone/tip/progress model assertion.
- [Loading UI script](../../godot/tests/loading_ui.gd) — current native decoration/three-slice RED fixture.
- [Terrain geometry script](../../godot/tests/terrain_geometry.gd) — historical `ac02b9a0` 256-chunk raw-geometry GREEN.
- `/tmp/claude/godot-login_flow-3de5946b.log` — historical login-flow exit 0, empty stderr.
- `/tmp/claude/godot-character_select_flow-d1641bc9.log` — successful local-auth/roster-flow exit 0, empty stderr.
- `/tmp/claude/godot-character-select-ui-corrected.log` — empty authored UI/Back exit 0, empty stderr.
- `/tmp/claude/godot-char-actions-build-1296be4b.log` — current native build GREEN, empty stderr.
- `/tmp/claude/godot-char-actions-flow-1296be4b.log` — current auth→Back flow GREEN, empty stderr.
- `/tmp/claude/godot-login_flow-host-revision-pending.log` — verifier67 login PASS on a timing-qualified `d1641bc9`/`12a23693` artifact.
- `/tmp/claude/godot-ui_projection-host-revision-pending.log` — verifier67 UI-projection PASS on the same timing-qualified artifact.
- `/tmp/claude/godot-ui_projection-3de5946b.log` — current UI-projection exit 0, empty stderr.
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
