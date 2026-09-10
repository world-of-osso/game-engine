# Bevy UI login

`src/scenes/login/` now constructs its login view with Bevy UI entities rather than toolkit login frames; other screens remain on the toolkit backend. This migration still requires compile, behavioral, rendered, and runtime proof. See [UI system](../wiki/systems/ui-system.md).

## What it must do

### Rendering and lifecycle

- [ ] Render login through Bevy UI, without duplicate legacy login frames or an alternate login renderer.
- [ ] Preserve background, shade, logos, fonts, input borders, button states, configured button variants, layout, footer and fade-in.
- [ ] Preserve currently hidden realm, registration and reconnect controls; migration does not expose additional actions.
- [ ] Keep the existing game menu above login and block login input while that modal is open.
- [ ] Remove login entities on exit without changing subsequent screens' rendering or input.

### Input and authentication

- [ ] Use one authoritative value per credential field for physical input, automation and authentication; displayed password remains masked using the existing UTF-8 byte-count convention.
- [ ] Preserve focus, Tab/Escape/Enter, cursor editing, control-character filtering, letter/byte limits and clipboard insertion. Cursor offsets must remain valid UTF-8 boundaries.
- [ ] Preserve press/release-over-same-button activation and disabled Connect behavior.
- [ ] Preserve realm selection, credential prefill, registration mode, reconnect checks and authentication resource/state transitions. Empty credentials retain the existing error.

### Automation

- [ ] Preserve semantic selectors and existing JS/CLI automation actions. Automated Connect drives the same action as the visible control.
- [ ] Include native login controls in frame waits and UI-tree dumps, without exposing raw passwords. Legacy screen waits and dumps remain supported.

## How it works

- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `src/scenes/login/` — login lifecycle, form input, visuals and authentication dispatch.
- `src/scenes/login/form.rs` — authoritative credential fields, UTF-8-safe edits, limits and masked presentation.
- `src/scenes/login/native.rs` — native login lifecycle, input, automation actions and authentication dispatch.
- `src/scenes/login/native_view.rs` — native entities, artwork and presentation synchronization.
- `src/ui/native.rs` — semantic native-UI marker and diagnostic formatter.
- `src/ui/automation.rs` — shared automation queue and legacy/native semantic waits.
- `src/dump_systems.rs`, `src/dump.rs`, `src/ipc/plugin/scene.rs` — combined legacy/native diagnostic tree requests and formatting.

## Tests asserting this spec

- `src/scenes/login/form.rs` — filtering, limits, editing, password display and independent fields.
- `src/scenes/login/native_tests.rs` — lifecycle, input, actions and authentication dispatch.
- `src/scenes/login/native_view_tests.rs` — status, masking, focus/fade and button-state presentation.
- `src/ui/native.rs`, `src/ui/automation.rs`, and `src/ipc/plugin/scene.rs` — native semantic waits and combined diagnostic trees.
- `src/scenes/login/native_tests.rs` includes setup asset-failure, successful feedback/custom-realm/focus, real-update fade-alpha, and startup-camera-order/teardown cases. At `1ad57bc6`, the successful custom-realm feedback/focus and visible-update fade-alpha cases pass.

At `f3ae396c`, 62 distinct revision-scoped focused cases pass: 58 prior cases, three startup-camera ordering cases, and one tint-channel regression. At `1ad57bc6`, two additional setup-success/fade cases pass, bringing the revision-scoped total to 64; `verification/setup-success-1ad57bc6/report.md` records the exact compile/test provenance. `bf4e259f` subsequently changes native image rendering to `NodeImageMode::Stretch` after `runtime/tint-login/view.webp` showed detached pieces caused by Bevy's default aspect fit. The 62 cases remain valid only for their prior scopes; no new enum-shape test was added for the stretch correction. `data/diagnostics/login-bevy-ui/verification/focused-report.md`, `verification/final-slice/report.md`, and `tint-fix/proof-ledger.md` retain commands, executable identities and retained failures. `cargo check --features dev --bin game-engine` passed at the earlier final slice; `cargo fmt --check` reported only 104 unchanged vendor paths.

At `bf4e259f`, `runtime/settled-retry/view-12.webp` and `view-30.webp` show the fully rendered standalone login; the isolated client ran 31.470 seconds and was terminated. `runtime/stretch/view.webp` shows native artwork with the toolkit game-menu overlay. Its UI-tree dump contains `adminvisual-proof` and 23 displayed password asterisks, not the raw dummy password. This is rendered and automated semantic evidence, not exact baseline-pixel parity, physical input, or real authentication proof. Existing CPU characterization does not certify the replacement renderer. Checkboxes remain open until their full contract proof exists.

## Build isolation

This worktree resolves `asset-resolver`, `shared-protocol`, `ui-toolkit`, and `ui-toolkit-macros` through matching `*-bevy-ui-login` dependency worktrees in `Cargo.toml`. Canonical dependency checkouts remain untouched; compilation proof records their revisions. The shared Cargo target remains an output cache, not a source dependency path.

## Known gaps (current cycle)

- [x] `1ad57bc6` verifies successful custom-realm feedback/focus and real-update fade-alpha. Dev credential prefill and full-plugin camera-initialization coverage remain open.
- [ ] Verify exact baseline-pixel parity. Rendered fade-in is covered by the pending real-update alpha case; `runtime/settled-retry/view-12.webp` and `view-30.webp` establish rendered standalone login, and `runtime/stretch/view.webp` establishes menu overlay.
- [ ] Verify physical input and real authentication at runtime.
- [ ] Preserve the runtime semantic result: `runtime/stretch/tree.stdout` shows typed username and a 23-asterisk password display without the raw dummy password; expand this to live automation proof only if a new runtime window is authorized.
- [ ] Run final formatting and relevant checks after source finality.

`tests/unit/login_screen_tests.rs`, `login_screen_workflow_tests.rs`, and `login_screen_test_support.rs` still target removed toolkit login symbols and are stale removal candidates. Do not delete them until their remaining observable obligations—especially layout centering/order, setup feedback, disabled appearance and realm/server preservation—are covered by native tests.

## Out of scope

- Other-screen migration, toolkit optimization and rendering-quality changes.
- Authentication protocol redesign or new visible login controls.
- Production deployment and unmeasured performance claims.
