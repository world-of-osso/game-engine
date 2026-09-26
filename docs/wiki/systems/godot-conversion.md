# Godot conversion

The Godot replacement remains an incomplete client-host experiment, not a completed migration. Godot is authorized to own rendering, UI, scenes, and gameplay while Bevy remains a transport-only headless networking worker. `94c4e6ae` reuses the original authored `LoadingModel`, `1be11d6c` exposes it through `RegistryUi.show_loading`, and `e8845a5e` centralizes original shell construction, panel styles, and Godot model registration. At `b90efecf`, shared original character-select postsetup registry sizing corrects the viewport action path, not a generic layout workaround. The clean native build exits 0 at `/tmp/claude/godot-enter-world-build-b90efecf.log`; a real viewport selects `Elara`, activates EnterWorld, and an actual local-server response establishes the selected character before `LoadingUI` (`/tmp/claude/godot-enter_world_flow-b90efecf.log`, exit 0, empty stderr). The authored PNG/shell begin at 0%; no fabricated `InWorld` occurs. World readiness remains absent. At `d1641bc9`/`9249929e`, successful `admin`/`admin` authentication against explicitly local UDP `127.0.0.1:5000` creates native `CharacterSelectUI`, maps protocol roster data through the original `CharacterSelectModel`, and hides `LoginUI`; raw ADT and M2 proof retain their prior limits. None establishes character-select actions, a character/world scene, terrain rendering, matched visuals, or full conversion parity.

## Bootstrap and workspace boundary

- Godot 4.7.2 project configuration starts `scenes/client.tscn`; `game_engine.gdextension` declares the Rust debug/release Linux libraries.
- `GameClient` is the exposed native `Node3D`. Historical `extension_smoke.gd` registered, instantiated, and attached it to a Godot scene tree. That is not current integrated proof; the former editor/import question also remains unresolved.
- `godot/` now contains `core`, `network`, `rust`, and `ui-model`. The workspace patches the shared protocol, UI macros, and local Taffy implementation.
- `godot/network` contains an actual headless Bevy-state/Lightyear/Replicon transport bridge, not only workspace dependencies. This is the authorized Bevy boundary; Godot owns rendering/UI/scenes/gameplay. `shared` still brings Bevy camera/mesh dependencies into the Godot workspace. Network verification ran 4/4 checks GREEN at `73339584`; later network work only re-exported `WireMessage` for typed host decoding.

## Portable UI and native login boundary

Sibling `ui-toolkit-core` commit `70db707` removes all Bevy dependencies. Its portable texture representation is `DynamicTextureId` plus RGBA8 registry data and atlas `PixelRect` arrays. Agent46 reports its two targeted model tests GREEN; main has not independently verified that result. This does not prove a Godot projection or UI parity.

`94c4e6ae` reuses the original authored `LoadingModel`; `1be11d6c` exposes it through `RegistryUi.show_loading`; `e8845a5e` centralizes original loading-bar shell construction, panel styles, and Godot model registration; and `203d2d85` projects authored three-slice loading art. The `c3911dff`-inclusive native build exits 0 at `/tmp/claude/godot-loading-shell-build.log`. `loading_ui.gd` exits 0 with empty stderr at `/tmp/claude/godot-loading-shell-fixture.log`, proving authored loading PNG artwork, shell, and initial progress. `63038a05` extracts the exact original readiness outcomes into `src/game/state/loading_readiness.rs`, shared by the Bevy adapter and a Godot-core export; agent86 reports 3/3 targeted tests GREEN. It is not native readiness integration. `LoadingModel` still assigns world readiness to the host. At `b90efecf`, the real selected-character success route reaches `LoadingUI` after the actual local response and shows the authored PNG/shell at 0%. It has no world-readiness implementation; fixture evidence is not rendered visual parity.

Main startup wires `ui::RegistryUi` and `create_login_ui`. At `50cd3c63`/`33ed3a57`, native `sync_input`/`pop_action` route connect, reconnect, exit, selected-host `set_server`, and pending status. `GameClient.connect_account` resolves the realm, starts `NetworkBridge`, and sends typed login/register requests through `AuthChannel`; `account_state` exposes `Session` effects. Before `8fff3ebf`, polling emitted `screen_requested` without character-select, creation, Loading, or world-scene routing. At `b90efecf`, shared original character-select postsetup registry sizing corrects the viewport action path; selected-character entry sends through `AuthChannel`, the actual local-server response transitions Loading and projects `LoadingUI`, and `account_state` exposes `selected_character_id`/`selected_character_name`. `enter_world_flow.gd` proves that bounded route; it proves neither world readiness nor a world scene.

The historical `login_flow.gd` log identifies `3de5946b`, exits 0, and has empty stderr: the existing local UDP server on port 5000 rejects wrong `admin` credentials, with credentials supplied by the native login button and rejection projected into status. Current `c3911dff`-inclusive `character_select_flow.gd` exits 0 with empty stderr at `/tmp/claude/character_select_flow.gd.green.log`: successful local auth projects an authored roster of two and Back restores login. `b7070542` Frame left-press routing is GREEN at `/tmp/claude/character_card_input.gd.green.log`: a viewport left press selects the authored second card. Current `ui_projection.gd` is GREEN at `/tmp/claude/ui_projection.gd.green.log`, covering the `a89ab4d4` input readability refactor. The `dd45d9bc` RED fixture is resolved in this bounded slice by `b90efecf` shared original character-select postsetup registry sizing, not a generic layout workaround. `enter_world_flow.gd` selects `Elara`, activates EnterWorld, receives the actual local-server response, establishes the selected character, and reaches `LoadingUI` (exit 0, empty stderr at `/tmp/claude/godot-enter_world_flow-b90efecf.log`). It observes the authored PNG/shell at 0% and does not fabricate `InWorld`. `world_units_flow.gd` is expected RED: it times out waiting for the selected-character unit node. Campsite/menu remain unsupported. This is bounded auth-to-Loading UI proof, not workflow or scene parity.

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
| UI model/projection | Bevy-free sibling frame/layout/widget/atlas/screen/registry model; native projection wires `sync_input`/`pop_action`. `1251724a` projects disabled buttons and suppresses their callbacks; `a89ab4d4` refactors input handling for readability. | Current `c3911dff`-inclusive `ui_projection.gd` exits 0 with empty stderr: native login layout, editing, focus, actions, updates, and removal. Behavioral fixture only; exact visual baseline is unverified. |
| Account host | `50cd3c63`/`33ed3a57` wire connect/reconnect/exit, selected-host `set_server`, pending status, and typed auth through `NetworkBridge`/`Session`. `d1641bc9`/`9249929e` create `CharacterSelectUI` with the original `CharacterSelectModel`, protocol roster mapping, and LoginUI removal after successful auth. `1296be4b` adds host `SelectChar` dispatch and routes Back; `b7070542` adds Frame left-press routing. At `b90efecf`, shared original character-select postsetup registry sizing corrects the viewport action path; selected-character entry sends through `AuthChannel`, and the successful response transitions Loading and projects `LoadingUI`. | Clean `b90efecf` build and `enter_world_flow.gd` are GREEN: real viewport selection of `Elara`, EnterWorld, actual local response, selected-character establishment, and `LoadingUI`. Authored PNG/shell are initially 0%; no fabricated `InWorld`. `world_units_flow.gd` is expected RED for missing selected-character unit nodes. No world route or scene proof follows. Campsite/menu remain unsupported. |
| Loading UI | `94c4e6ae` reuses original authored `LoadingModel`; `1be11d6c` exposes it through `RegistryUi.show_loading`; `e8845a5e` centralizes original shell construction, panel styles, and Godot model registration; `203d2d85` projects authored three-slice loading art. At `b90efecf`, selected-character success projects `LoadingUI`. | The clean `b90efecf` build exits 0. `loading_ui.gd` proves authored PNG artwork, shell, and initial progress; `enter_world_flow.gd` observes the authored PNG/shell at 0% after the actual local response. No world readiness, `InWorld`, world scene, or rendered visual-parity proof. |
| Transport/gameplay | Actual headless Bevy-state/Lightyear/Replicon transport bridge. It remains transport-only; Godot owns rendering/UI/scenes/gameplay. | Network verifier: 4/4 GREEN at `73339584`; the account-rejection path is runtime proof only for typed auth/status feedback. No login-to-world state application, collision, equipment, gameplay, or parity proof. |
| Build/readability | `a89ab4d4` refactors UI input for readability; `203d2d85` adds native loading three-slice projection; `b90efecf` corrects shared original character-select postsetup registry sizing. `cc324f51` fixes native Button theme content margins that enlarged authored 64px buttons to 71px and adds a size assertion. | Clean `b90efecf` native build plus `character_select_ui.gd` and `enter_world_flow.gd` are GREEN. `world_units_flow.gd` is expected RED for missing selected-character unit nodes. `/tmp/claude/godot-button-size-red.log` records genuine `actual(256,71)`; `cc324f51` is committed but unbuilt and unproven, so `b90efecf` does not cover it. Bounded fixture evidence only; not full integration or visual-parity proof. |
| Character-select UI | `d1641bc9`/`9249929e` construct native `CharacterSelectUI` from the original `CharacterSelectModel`, map protocol roster data, and hide LoginUI. It includes empty authored UI and Back action; EnterWorld remains enabled as in the original authored button. `1296be4b` adds host `SelectChar` dispatch; `b7070542` adds Frame left-press routing; `b90efecf` corrects shared original postsetup registry sizing. | `character_select_ui.gd` and `enter_world_flow.gd` are GREEN at `b90efecf`. The latter selects `Elara`, activates EnterWorld, and reaches `LoadingUI` after actual local response. No world readiness, `InWorld`, delete/create, character/world scene, background/appearance, or matched visual proof. |
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
- [Loading UI script](../../godot/tests/loading_ui.gd) — authored PNG/shell initial-progress fixture GREEN.
- [Terrain geometry script](../../godot/tests/terrain_geometry.gd) — historical `ac02b9a0` 256-chunk raw-geometry GREEN.
- `/tmp/claude/godot-login_flow-3de5946b.log` — historical login-flow exit 0, empty stderr.
- `/tmp/claude/godot-character_select_flow-d1641bc9.log` — successful local-auth/roster-flow exit 0, empty stderr.
- `/tmp/claude/godot-character-select-ui-corrected.log` — empty authored UI/Back exit 0, empty stderr.
- `/tmp/claude/godot-char-actions-build-1296be4b.log` — current native build GREEN, empty stderr.
- `/tmp/claude/godot-enter-world-red.log` — historical `dd45d9bc` viewport EnterWorld fixture RED; `b90efecf` resolves its no-dispatch root cause through shared original postsetup registry sizing.
- `/tmp/claude/godot-enter-world-build-b90efecf.log` — clean native build exit 0.
- `/tmp/claude/godot-character_select_ui-b90efecf.log` — authored empty UI/Back fixture exit 0, empty stderr.
- `/tmp/claude/godot-enter_world_flow-b90efecf.log` — real viewport Elara→EnterWorld→actual local response→LoadingUI exit 0, empty stderr.
- `/tmp/claude/godot-world_units_flow-b90efecf.log` — expected RED: selected-character unit node absent.
- `/tmp/claude/godot-button-size-red.log` — genuine pre-`cc324f51` native Button size RED (`actual(256,71)`); fix unbuilt/unproven.
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
