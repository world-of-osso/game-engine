# Native UI automation

Native startup runs the existing synchronous JavaScript UI automation API against mounted Godot UI. Native action consumption lives in `godot/rust/src/js_automation.rs`; the compiler is shared with the original client. See [Godot conversion](../wiki/systems/godot-conversion.md) for host context.

## What it must do

### Script and lifecycle

- [ ] `--run-js-ui-script <path>` loads the chosen UTF-8 script through production startup and synchronously compiles it before frame-driven consumption. QuickJS must not access Godot objects.
- [ ] Preserve all ten original action variants and the original `ui.*`, `env`, state aliases, and key-chord parser semantics. Missing environment values are empty strings.
- [ ] Execute queued actions in order on the main thread, after the native client frame, without a GameClient bind during input dispatch. Waits and deadlines advance by frame delta; no blocking or UI-thread sleeps.
- [ ] Stop consumption and report an explicit error on failed actions, invalid timing, unknown/unconverted keys, missing or hidden frames, or unmet state/frame deadlines. Never bypass authentication or substitute callbacks.

### Native input and waits

- [ ] Click the center of the nearest actual visible mounted Control with the requested name, using real motion and mouse press/release events through the root viewport's existing input route. Right-click and Shift-click remain InWorld-only.
- [ ] Type text using Unicode key press/release events into an actual focused editable native LineEdit or TextEdit. Never set text, insert text directly, emit pressed, or call account callbacks instead of dispatching input.
- [ ] Dispatch parsed key chords through native physical/logical key events with modifier press/release semantics. Modifier chords remain InWorld-only, matching the original screen handlers; unconverted native keys fail explicitly.
- [ ] `waitForState` supports Login, Connecting, CharSelect, Loading, and InWorld from actual native phase/session state, not inferred fixture markers. `waitForFrame` requires a visible mounted named Control.
- [ ] `dumpTree` and `dumpUiTree` print actual live native IPC formatter Tree text to stdout; formatter failures stop the queue.

### Behavioral proof

- [ ] Run unchanged `debug/login.js` through startup. Observe actual LineEdit changes, actual ConnectButton pressed, real Account UDP authentication, authoritative CharSelect roster projection, and production stdout `ui.dumpUiTree`.

## How it works

- [Godot conversion host](../wiki/systems/godot-conversion.md)
- [Native IPC diagnostics contract](native-ipc.md)

## Implementation inventory

- `godot/rust/src/js_automation.rs` — native ordered queue, frame waits, input-event dispatch, and dedicated Node host.
- `src/ui/js_automation.rs` — original synchronous JS compiler, environment bridge, and state aliases, exported by the native network crate.
- `src/ui/automation_data.rs` — shared original action variants and key-chord grammar.
- `godot/rust/src/startup.rs` — native startup intent and script-host attachment integration.
- `godot/rust/src/ipc.rs` — live diagnostic formatter entry points used by dump actions.

## Tests asserting this spec

- `godot/network/examples/native_js_automation_fixture.rs` — real account UDP fixture and production child-process/stdout checks.
- `godot/tests/native_js_automation_flow.gd` — observation-only authored Login input and authoritative CharSelect checks.
- `src/ui/js_automation.rs` tests — compiler action/key/wait semantics, not native input proof.

## Known gaps (current cycle)

- [ ] Consumer source is not compile/runtime acceptance. Main owns locked dependency compilation, startup integration, and fixture GREEN evidence.
- [ ] Supplied first-runtime evidence is genuine RED: authored Login mounted, but no script credential entry or Connect click. Login fixture alone cannot prove all ten actions, key/modifier behavior, every state wait, or timeout/error boundaries.

## Out of scope

- Global world/UI input ownership changes; existing feature-specific native routes remain authoritative.
- Auction House, Mail, quest, Bank, and Trade host conversion or new IPC consumers.
- Extra JS fixtures, export/distribution acceptance, and full Godot conversion parity.
