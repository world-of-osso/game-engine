# Native IPC diagnostics

Native Godot diagnostics serve the existing public engine CLI through `godot/rust/src/ipc.rs`, using the original wire definitions exported by `game_engine_network::ipc_wire`. Architecture context: [Godot conversion](../wiki/systems/godot-conversion.md). Six diagnostics have bounded accepted proof; `ExportScene` is an additional implementation slice pending MAIN integration/runtime proof. Neither completes the broader [conversion tooling contract](godot-conversion.md).

## What it must do

### Lifecycle and transport

- [ ] `NativeIpc::start() -> Result<NativeIpc, String>` returns success only after binding `/tmp/game-engine-<own PID>.sock`; runtime, thread and bind failures are explicit.
- [x] Preserve `peercred-ipc` v0.2.0 at `dc0093cf4103b6bd887fc5941132fd9d4293e22e`: original MessagePack framing, socket mode, receive limit and single-request connection behavior. Identify peers without adding authorization absent from the legacy engine.
- [x] Tokio handles only wire values/channels. Godot objects, registry reads, viewport readback and response construction stay on the main thread; no unsafe `Send` implementation.
- [ ] Main-thread exit disconnects capture callbacks, stops and joins the worker, cancels connection tasks and removes only the own-PID socket. Never scan/remove another instance's sockets or replace Godot's process signal handlers.
- [x] Preserve all 129 original request definitions through the shared wire source (accepted six-diagnostic source proof).
- [ ] After MAIN integrates `ExportScene`, the other 122 consumers dispatch to `Response::Error`, not successful placeholders or gameplay side effects. All 129 variants deserialize through shared definitions; exhaustive runtime coverage remains unproved.
- [x] The unchanged fixture exits normally within its 10-second bound and removes its own-PID socket.

### Diagnostic surface

- [x] `Ping` returns the original `Response::Pong`, preserving plain CLI `pong` and JSON `"Pong"`.
- [x] `DumpTree` returns `Response::Tree` containing live node names, native instance identities and available local transforms/visibility. Preserve two-space hierarchy indentation and case-insensitive name-only filtering: matched nodes do not imply ancestor/descendant matches; no matches yield empty text.
- [x] `DumpUiTree` reads every mounted `RegistryUi::registry()`, preserving the original frame text: type, resolved/stored size, visibility, strata/level, layout, position, alpha, scale, widget values, password masking and texture detail. Sort roots by name within each registry. Match the complete main frame line case-insensitively; a matched frame includes its descendants.
- [x] `DumpScene` exposes the fixture's live camera FOV/local position/current state and mesh surface count/visibility-in-tree.
- [ ] `DumpScene` returns live rendering-semantic data, including actual camera FOV, light energy, model bounds, meshes and spatial hierarchy. Preserve the legacy dispatcher's behavior of ignoring its filter. Never report synthetic character/model properties.
- [x] `Screenshot` captures the owning viewport after `RenderingServer.frame_post_draw`, returns actual WebP bytes in `Response::Screenshot`, and preserves original lossy quality 65%. Independently changed 2D and actual 3D pixels must appear in captures at viewport size; missing/freed viewport, missing pixels, encoding or signal connection failure is explicit.
- [x] `Performance` preserves `fps`, `frame_time_ms`, `focused` in `Response::Performance`. FPS comes from the engine's measured average; frame time from consecutive main-thread process observations; focus from the containing window. Missing/nonpositive measurements remain `None`, never fabricated constants.

### Scene export

- [ ] `ExportScene` writes the original `SceneSnapshot` JSON through the shared `game_engine_core::scene_snapshot` types/writer; preserve externally tagged `NodeProps`, labels, optional transforms and recursive children without schema changes. Success returns original `Response::Text("scene exported to {output_path}")`; tree, metadata, encoding and file-write failures return `Response::Error`.
- [ ] Export the live containing Window as the root: actual node name, `Scene` props, `transform: null`. A scene with no selected semantic children may serialize normally.
- [ ] Select actual `Camera3D` as `Camera { fov }` in native degrees and `Light3D` as `Light { kind, intensity }` using the actual native class and energy. Select M2 roots only from typed `m2_source_path` GString metadata containing the exact loader input; emit `Object { kind: "M2", model }`. Empty/malformed metadata fails explicitly; never infer model paths from names, bounds, skeletons or batches.
- [ ] Omit nonsemantic spatial groups, UI containers and procedural meshes without retyping them as Scene/Object. Retain nearest exported semantic ancestry; accumulate local `Transform3D` through skipped Node3D groups, resetting for children of selected nodes. Export translation/rotation-quaternion/scale arrays relative to the nearest exported ancestor; no global-transform substitution or coordinate fallback.
- [ ] Do not fabricate character race/gender/name/ID, background/doodad counts, equipment slots/anchors, generic mesh-resource identity or unsupported Player/Npc/Terrain/Ground semantics. Legacy JSON compatibility does not mean full legacy semantic parity.

## How it works

- [Godot conversion architecture and current proof boundaries](../wiki/systems/godot-conversion.md)

## Implementation inventory

- `godot/rust/src/ipc.rs` — own-PID worker, typed request/reply queue, main-thread dispatcher, actual post-draw capture and lifecycle cleanup.
- `godot/rust/src/ipc/tree.rs` — actual native hierarchy and rendering-semantic dumps.
- `godot/rust/src/ipc/export.rs` — selected live camera/light/M2 snapshot export with compensated local TRS; dispatcher wiring is MAIN-owned.
- `game_engine_core::scene_snapshot` — MAIN-owned shared original JSON types and file writer; schema unchanged.
- `godot/rust/src/assets/mod.rs` — MAIN-owned `M2_SOURCE_META` loader-input metadata on real M2 roots.
- `godot/rust/src/ipc/ui_tree.rs` — mounted registry traversal and original UI dump text/filter behavior.
- `game_engine_network::ipc_wire` — MAIN-owned export of the single original wire definitions, not a native protocol copy.
- `godot/rust/src/lib.rs` and `godot/rust/Cargo.toml` — MAIN-owned lifecycle hooks, registration and dependencies.
- `src/bin/game-engine-cli/`, `src/ipc/mod.rs`, `src/dump.rs`, `src/ipc/plugin/scene.rs` — original public CLI and legacy diagnostic contracts. Shared wire definitions are extracted to `src/ipc/wire.rs`; root API re-exports remain compatible.

## Tests asserting this spec

- `godot/network/examples/native_ipc_fixture.rs` — parent invokes actual public CLI after native READY; retained six diagnostics/captures/normal-exit checks plus public `export-scene` before captures, exact plain-text response and written JSON.
- `godot/tests/native_ipc_flow.gd` — retained diagnostic/pixel oracles plus independent exported-JSON checks: schema/tag/TRS arrays, omitted groups/UI/procedural mesh, direct Camera→Light/M2 ancestry, exact loader input, native FOV75/energy2.5 and analytically authored compensated transforms. Camera `(0,0,3)`; skipped group `(4,2,-6)`/Y90; light `(1,3,2)`/X60 becomes camera-relative `(6,5,-7)`; M2 `(-2,1,3)`/Y−90/scale0.5 becomes `(7,3,-4)`/identity quaternion/scale0.5. These new assertions have RED evidence only; no runtime GREEN claimed.

MAIN-observed pre-implementation RED: `/tmp/claude/native-ipc-third-runtime-red.log`, October 1, 2026. Actual Login READY precedes public CLI `ping` failing with `No such file or directory`; this is socket absence, not a fixture setup failure. MAIN accepts independent1531 **bounded PASS** at formatter fix `68dfe530`: [acceptance SSOT](/tmp/claude/verify-native-ipc-diagnostics-accepted.md). Scope: six public diagnostics and the unchanged fixture's normal exit/own socket cleanup, not full conversion.

Export RED at `d1981968`, October 1, 2026: `/tmp/claude/native-export-scene-first-red-runtime.log` records READY PID635186; retained `data/diagnostics/native-ipc-635183/export-scene.stderr` records public CLI `ExportScene` unported failure (parent exit1). Depot `330ww90wlr0`; the existing six diagnostic requests passed before RED. This is missing consumer behavior, not setup failure. New export implementation has no compile/runtime GREEN or full semantic acceptance yet.

## Known gaps (current cycle)

- [ ] MAIN must integrate dispatcher, shared schema/core dependency and real M2 metadata, compile on Depot and run the independent export JSON oracle. Malformed/empty metadata and write errors lack dedicated native runtime assertions; nonuniform-scale shear/top-level spatial exceptions are not established by this bounded TRS fixture.
- [ ] Root formatter gaps remain MAIN-owned; formatting this slice does not establish root `cargo fmt --check` acceptance.

- Native hooks register `mod ipc`, store `Option<ipc::NativeIpc>`, start in ready, poll every process frame and take/drop in exit_tree before singleton teardown. A worker-disconnection error logs and drops the service. Accepted proof uses fresh Depot `tc318dqwqh` extension build0, retained unchanged fixture from `66r8d2ncdx`, and fresh native PID421404 runtime0; exact provenance and boundaries live in the acceptance SSOT above.
- Historical first-build E0599 and independent1524 overall FAIL for `semantic_label` length remain preserved in [historical report](/tmp/claude/verify-native-ipc-diagnostics.md). Extraction `68dfe530` resolves that finding under independent1531; inherited `NativeWmoGroup.fdid` warning and root/UI readability debt remain uncleared. Lifecycle/startup error requirements remain unchecked beyond the demonstrated normal own-fixture boundary.
- [ ] Full semantic parity and light/model runtime cases are unproved: native spatial names/model bounds are live, but legacy character race/gender/name/ID, background model/doodad counts, equipment-slot anchors and camera-frustum/raycast `is_displayed` semantics have no equivalent typed native snapshot here. Mesh/model `is_displayed` means actual Godot visibility-in-tree, not legacy occlusion/frustum evidence.
- [ ] Focus transitions and numerical timing accuracy are not asserted by the existing fixture. Native frame time is observed wall-clock interval, not the legacy diagnostics smoothing algorithm.
- [ ] Unported-request errors, concurrent/malformed/disconnected requests, startup failures and repeated lifecycle boundaries lack additional runtime assertions. Preserve the original absence of a server response deadline; an unresponsive main thread/render loop can leave callers waiting until shutdown.
- [ ] The unchanged transport rejects received payloads above 4 MiB. Large captures/dumps can exceed that serialized limit; no resizing, truncation, alternate transport or invented fallback is supplied. HDR viewport color equivalence, viewport reparenting after initial capture registration, WebP maximum extent and general signal/crash cleanup remain unproved.

## Out of scope

- The other 122 IPC consumers after export integration, including Auction House, Mail, quest, Bank and Trade actions: explicit unported errors only; native gameplay/UI ownership is unchanged.
- JS automation, other exports and debug-screen conversions: separate requests/features, not silently covered by `DumpScene` or this selected-node `ExportScene`.
- CLI/wire redesign, stale socket cleanup, UID authorization, process signal replacement, synthetic screenshots/performance and alternate renderer paths: not authorized.
- Shared integration, dependency/lock changes, deployment, operational proof, readability/check/final gates and broad conversion acceptance: MAIN-owned.
