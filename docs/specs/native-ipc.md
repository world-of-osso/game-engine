# Native IPC diagnostics

Native Godot diagnostics serve the existing public engine CLI through `godot/rust/src/ipc.rs`, using the original wire definitions exported by `game_engine_network::ipc_wire`. Architecture context: [Godot conversion](../wiki/systems/godot-conversion.md). This bounded port covers six requests; it does not complete the broader [conversion tooling contract](godot-conversion.md).

## What it must do

### Lifecycle and transport

- [ ] `NativeIpc::start() -> Result<NativeIpc, String>` returns success only after binding `/tmp/game-engine-<own PID>.sock`; runtime, thread and bind failures are explicit.
- [ ] Preserve `peercred-ipc` v0.2.0 at `dc0093cf4103b6bd887fc5941132fd9d4293e22e`: original MessagePack framing, socket mode, receive limit and single-request connection behavior. Identify peers without adding authorization absent from the legacy engine.
- [ ] Tokio handles only wire values/channels. Godot objects, registry reads, viewport readback and response construction stay on the main thread; no unsafe `Send` implementation.
- [ ] Main-thread exit disconnects capture callbacks, stops and joins the worker, cancels connection tasks and removes only the own-PID socket. Never scan/remove another instance's sockets or replace Godot's process signal handlers.
- [ ] All 129 original request variants deserialize through shared definitions; the 123 unported variants return `Response::Error`, not successful placeholders or gameplay side effects.

### Diagnostic surface

- [ ] `Ping` returns the original `Response::Pong`, preserving plain CLI `pong` and JSON `"Pong"`.
- [ ] `DumpTree` returns `Response::Tree` containing live node names, native instance identities and available local transforms/visibility. Preserve two-space hierarchy indentation and case-insensitive name-only filtering: matched nodes do not imply ancestor/descendant matches; no matches yield empty text.
- [ ] `DumpUiTree` reads every mounted `RegistryUi::registry()`, preserving the original frame text: type, resolved/stored size, visibility, strata/level, layout, position, alpha, scale, widget values, password masking and texture detail. Sort roots by name within each registry. Match the complete main frame line case-insensitively; a matched frame includes its descendants.
- [ ] `DumpScene` returns live rendering-semantic data, including actual camera FOV, light energy, model bounds, meshes and spatial hierarchy. Preserve the legacy dispatcher's behavior of ignoring its filter. Never report synthetic character/model properties.
- [ ] `Screenshot` captures the owning viewport after `RenderingServer.frame_post_draw`, returns actual WebP bytes in `Response::Screenshot`, and preserves original lossy quality 65%. Independently changed 2D and actual 3D pixels must appear in captures at viewport size; missing/freed viewport, missing pixels, encoding or signal connection failure is explicit.
- [ ] `Performance` preserves `fps`, `frame_time_ms`, `focused` in `Response::Performance`. FPS comes from the engine's measured average; frame time from consecutive main-thread process observations; focus from the containing window. Missing/nonpositive measurements remain `None`, never fabricated constants.

## How it works

- [Godot conversion architecture and current proof boundaries](../wiki/systems/godot-conversion.md)

## Implementation inventory

- `godot/rust/src/ipc.rs` — own-PID worker, typed request/reply queue, main-thread dispatcher, actual post-draw capture and lifecycle cleanup.
- `godot/rust/src/ipc/tree.rs` — actual native hierarchy and rendering-semantic dumps.
- `godot/rust/src/ipc/ui_tree.rs` — mounted registry traversal and original UI dump text/filter behavior.
- `game_engine_network::ipc_wire` — MAIN-owned export of the single original wire definitions, not a native protocol copy.
- `godot/rust/src/lib.rs` and `godot/rust/Cargo.toml` — MAIN-owned lifecycle hooks, registration and dependencies.
- `src/bin/game-engine-cli.rs`, `src/ipc/mod.rs`, `src/dump.rs`, `src/ipc/plugin/scene.rs` — original public CLI, protocol and legacy diagnostic contracts; unchanged by this module.

## Tests asserting this spec

- `godot/network/examples/native_ipc_fixture.rs` — unchanged parent invokes actual public CLI after native READY, observes replies, captures and own-PID normal-exit socket removal.
- `godot/tests/native_ipc_flow.gd` — unchanged external runtime oracles: live hierarchy/filter results, actual Login registry, semantic camera, finite positive settled performance, root viewport dimensions and independently changed rendered pixels.

MAIN-observed pre-implementation RED: `/tmp/claude/native-ipc-third-runtime-red.log`, October 1, 2026. Actual Login READY precedes public CLI `ping` failing with `No such file or directory`; this is socket absence, not a fixture setup failure. No GREEN/build/runtime acceptance is claimed by this spec.

## Known gaps (current cycle)

- [ ] MAIN must register `mod ipc`, store `Option<ipc::NativeIpc>`, call `start` in ready, call `poll(&Gd<Node>) -> Result<(), String>` every process frame, and drop/take it in exit_tree before Godot singleton teardown. Handle `poll` worker-disconnection errors by logging and dropping the service rather than repeatedly calling a dead worker.
- [ ] Native compilation and unchanged runtime fixture GREEN await MAIN. No local extension Cargo, client operations or final gate executed by the module worker.
- [ ] Full semantic parity is unproved: native spatial names/model bounds are live, but legacy character race/gender/name/ID, background model/doodad counts, equipment-slot anchors and camera-frustum/raycast `is_displayed` semantics have no equivalent typed native snapshot here. Mesh/model `is_displayed` means actual Godot visibility-in-tree, not legacy occlusion/frustum evidence.
- [ ] Focus transitions and numerical timing accuracy are not asserted by the existing fixture. Native frame time is observed wall-clock interval, not the legacy diagnostics smoothing algorithm.
- [ ] Unported-request errors, concurrent/malformed/disconnected requests, startup failures and repeated lifecycle boundaries lack additional runtime assertions. Preserve the original absence of a server response deadline; an unresponsive main thread/render loop can leave callers waiting until shutdown.
- [ ] The unchanged transport rejects received payloads above 4 MiB. Large captures/dumps can exceed that serialized limit; no resizing, truncation, alternate transport or invented fallback is supplied. HDR viewport color equivalence, viewport reparenting after initial capture registration, WebP maximum extent and general signal/crash cleanup remain unproved.

## Out of scope

- The other 123 IPC consumers, including Auction House, Mail, quest, Bank and Trade actions: explicit unported errors only; native gameplay/UI ownership is unchanged.
- JS automation, exports and debug-screen conversions: separate requests/features, not silently covered by `DumpScene`.
- CLI/wire redesign, stale socket cleanup, UID authorization, process signal replacement, synthetic screenshots/performance and alternate renderer paths: not authorized.
- Shared integration, dependency/lock changes, deployment, operational proof, readability/check/final gates and broad conversion acceptance: MAIN-owned.
