# Native UI automation

Native startup runs the existing synchronous JavaScript UI automation API against mounted Godot UI. Native action consumption lives in `godot/rust/src/js_automation.rs`; the compiler is shared with the original client. See [Godot conversion](../wiki/systems/godot-conversion.md) for host context.

## What it must do

### Script and lifecycle

- [ ] `--run-js-ui-script <path>` loads the chosen UTF-8 script through production startup and synchronously compiles it before frame-driven consumption. QuickJS must not access Godot objects.
- [ ] Preserve all ten original action variants and the original `ui.*`, `env`, state aliases, and key-chord parser semantics. Missing environment values are empty strings.
- [ ] Execute queued actions in order on the main thread, after the native client frame, without a GameClient bind during input dispatch. Waits and deadlines advance by frame delta; no blocking or UI-thread sleeps.
- [ ] Report explicit errors for failed actions, invalid timing, unknown/unconverted keys, missing or hidden frames, or unmet state/frame deadlines. State/frame deadline failures retain the last wait error, pop only that wait, reset its clock, and continue following actions; other errors stop the queue. Never bypass authentication or substitute callbacks.

### Native input and waits

- [ ] Click the center of the nearest actual visible mounted Control with the requested name, using real motion and mouse press/release events through the root viewport's existing input route. Right-click and Shift-click remain InWorld-only.
- [ ] Type text using Unicode key press/release events into an actual focused editable native LineEdit or TextEdit. Never set text, insert text directly, emit pressed, or call account callbacks instead of dispatching input.
- [ ] Dispatch parsed key chords through native physical/logical key events with modifier press/release semantics. Modifier chords remain InWorld-only, matching the original screen handlers; unconverted native keys fail explicitly.
- [ ] `waitForState` supports Login, Connecting, CharSelect, Loading, and InWorld from actual native phase/session state, not inferred fixture markers. `waitForFrame` requires a visible mounted named Control.
- [ ] `dumpTree` and `dumpUiTree` print actual live native IPC formatter Tree text to stdout; formatter failures stop the queue.

### Behavioral proof

- [x] Run unchanged `debug/login.js` through startup. Observe actual LineEdit changes, actual ConnectButton pressed, real Account UDP authentication, authoritative CharSelect roster projection, and production stdout `ui.dumpUiTree`.

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

## Current proof and gaps

- Independent1542 accepts bounded unchanged `debug/login.js` and missing-frame deadline → successor live UI dump, plus four introduced readability corrections. Functional/readability/root compilation PASS; overall FAIL because root formatting remains blocked on transferred quest-owned comment spacing (owner836 notified). This is not full-feature or root CI readiness.
- `bab8597c` records the deadline error, pops only the failed wait, resets its clock and continues following actions; other errors remain terminal. Current frame-continuation and normal Login supplied runtimes exit0. Historical genuine continuation RED and earlier observer setup failure remain distinct.
- Independent1550 accepts offline predicate cleanup `b87fc590` by source equivalence with retained executed `3a696582` proof; independent1557 accepts current world GDS `35a3921e` source/order/runtime and introduced readability17/17. Historical1547/1548/1555 FAIL reports remain historical. Combined independently accepted normal/offline/world examples traverse all ten action variants, not full semantics, 71 keys, contexts, negatives, GUI visibility, elapsed-time accuracy or lifecycle parity. Broad requirement checkboxes remain open.
- Tests-only negative startup adds MAIN-observed absent-focus typing rejection and successor suppression; independent1568 report and MAIN acceptance remain pending. [Evidence SSOT](../wiki/systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail) owns exact script, revisions, artifacts, setup failure and observer-owned exit distinction. General negative/lifecycle coverage and broad JS/root-format FULL goal remain open.
- [Godot conversion evidence SSOT](../wiki/systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail) owns exact revisions, artifacts, coverage and retained warnings/errors. Broad requirement checkboxes above remain open where bounded proof does not cover the whole requirement.

## Out of scope

- Global world/UI input ownership changes; existing feature-specific native routes remain authoritative.
- Auction House, Mail, quest, Bank, and Trade host conversion or new IPC consumers.
- Export/distribution acceptance and full Godot conversion parity.
