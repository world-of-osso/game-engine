# Bevy UI login

`src/scenes/login/` renders through Bevy UI entities while its layout hierarchy is authored with native `rsx!`; other screens remain on the toolkit backend. The native-RSX restoration at macro `35c1ddc` and engine `81f13187` is pending verification. See [UI system](../wiki/systems/ui-system.md).

## What it must do

### Rendering and lifecycle

- [x] Render login through Bevy UI, without duplicate legacy login frames or an alternate login renderer.
- [x] Preserve background, shade, logos, fonts, input borders, button states, configured button variants, layout, footer and fade-in.
- [x] Preserve currently hidden realm, registration and reconnect controls; migration does not expose additional actions.
- [x] Keep the existing game menu above login and block login input while that modal is open.
- [x] Remove login entities on exit without changing subsequent screens' rendering or input.

### Input and authentication

- [x] Use one authoritative value per credential field for physical input, automation and authentication; displayed password remains masked using the existing UTF-8 byte-count convention.
- [x] Preserve focus, Tab/Escape/Enter, cursor editing, control-character filtering, letter/byte limits and clipboard insertion. Cursor offsets remain valid UTF-8 boundaries.
- [x] Show a blinking insertion caret in the focused native field at the actual edit position. Map username UTF-8 cursor boundaries and password byte-count masking correctly; reset blink after edits, navigation and focus; hide when unfocused or modal-blocked; align and clip it with field text.
- [x] Preserve press/release-over-same-button activation and disabled Connect behavior.
- [x] Preserve realm selection, credential prefill, registration mode, reconnect checks and authentication resource/state transitions. Empty credentials retain the existing error.

### Automation

- [x] Preserve semantic selectors and existing JS/CLI automation actions. Automated Connect drives the same action as the visible control.
- [x] Include native login controls in frame waits and UI-tree dumps, without exposing raw passwords. Legacy screen waits and dumps remain supported.

## How it works

- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `src/scenes/login/` — login lifecycle, form input, visuals and authentication dispatch.
- `src/scenes/login/form.rs` — authoritative credential fields, UTF-8-safe edits, limits and masked presentation.
- `src/scenes/login/native.rs` — native login lifecycle, input, automation actions and authentication dispatch.
- `src/scenes/login/native_view.rs` — native entities, artwork, presentation synchronization, insertion-caret presentation, and the native `rsx!` login hierarchy.
- `src/scenes/login/native_caret.rs` — shaped-text cursor geometry and blink visibility.
- `src/scenes/login/native_caret_tests.rs` — cursor geometry, password masking, focus/modal hiding and blink tests.
- `src/ui/native.rs` — semantic native-UI marker and diagnostic formatter.
- `src/ui/automation.rs` — shared automation queue and legacy/native semantic waits.
- `src/dump_systems.rs`, `src/dump.rs`, `src/ipc/plugin/scene.rs` — combined legacy/native diagnostic tree requests and formatting.

## Native RSX authoring

`rsx! { @native(commands, parent) { ... } => result }` emits Bevy entities directly. It does not construct toolkit `WidgetDef`s, use `Screen`, or update `FrameRegistry`.

- `node` creates a `Node`; nested nodes receive `ChildOf` from their lexical parent.
- `id:` binds the spawned entity for sibling Rust statements and the trailing result expression.
- `name:` accepts an ordinary `String` expression. This differs from legacy RSX `name:`, which accepts `FrameName`.
- `layout:` supplies a `Node` base; typed node fields override it.
- `components:` inserts typed Bevy components alongside the node.
- Rust statement blocks may call existing native helpers for fields, nine-slices, buttons, and carets without introducing a second UI tree.

The initial native mode intentionally covers only the direct Bevy construction required by login. Its compile, behavioral, and rendered-equivalence tests remain pending verification.

## Tests asserting this spec

- `src/scenes/login/form.rs` — filtering, limits, editing, password display and independent fields.
- `src/scenes/login/native_tests.rs` — lifecycle, input, actions and authentication dispatch.
- `src/scenes/login/native_view_tests.rs` — status, masking, focus/fade and button-state presentation.
- `src/ui/native.rs`, `src/ui/automation.rs`, and `src/ipc/plugin/scene.rs` — native semantic waits and combined diagnostic trees.
- `src/scenes/login/native_tests.rs` includes setup asset-failure, successful feedback/custom-realm/focus, real-update fade-alpha, and startup-camera-order/teardown cases. At `1ad57bc6`, the successful custom-realm feedback/focus and visible-update fade-alpha cases pass.

The migration has 72 distinct revision-scoped focused cases: 64 prior behavior/presentation/setup cases, six caret cases, and two computed-layout cases. `verification/layout-f97b43e8/report.md` records real UiPlugin/TextPlugin layout at 1280×720 and 1600×900: centered fields, ordering, action placement, hidden controls, footer/background/logo, and camera restoration. `verification/caret/acceptance/report.md` records shaped UTF-8/password caret positions, blink/focus/modal behavior, 1×/2× clipping, and rendered off/on captures.

`runtime/settled-retry/view-12.webp` and `view-30.webp` show rendered standalone login; `runtime/stretch/view.webp` shows the toolkit menu above native login. The user manually confirmed physical typing, Tab, Menu, and Quit. `runtime/live-auth/prefilled/` verifies native ConnectButton submission with local `admin/admin`: credentials reached the server, two characters returned, and CharSelect replaced the native login UI. The timeout interrupted shutdown after `AppExit::Success`, so this does not claim a clean process exit. Existing global `cargo fmt --check` reports 104 unchanged vendor files; scoped formatting and relevant checks pass.

## Build isolation

Development used matching `*-bevy-ui-login` dependency worktrees to isolate migration work. Canonical Cargo paths are restored before the requested master merge; the shared Cargo target remains an output cache, not a source dependency path.

## Evidence limits

- No before/after pixel baseline exists. Rendered screenshots and computed layout verify the contract, not image-identical output.
- Physical input is user-confirmed because current Wayland tooling has no safe window-targeted injection; no additional injection framework is required.
- The local authentication run proves the tested credential path to CharSelect, not remote-error handling or clean timeout shutdown.
- Long overflowing/bidi caret editing is outside this login migration contract.

`f7d84588` removes the three unregistered toolkit-login test fixtures after porting their observable layout, visibility, setup, status, realm/server, input, and workflow coverage to native tests.

## Out of scope

- Other-screen migration, toolkit optimization and rendering-quality changes.
- Authentication protocol redesign or new visible login controls.
- Production deployment and unmeasured performance claims.
