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

- `tests/unit/login_screen_tests.rs` — pre-migration login behavior characterization.
- `tests/unit/login_screen_workflow_tests.rs` — pre-migration automation/authentication workflows.
- `src/scenes/login/form.rs` — standalone model tests for filtering, limits, editing, password display and independent fields.
- `src/scenes/login/native_tests.rs` — native lifecycle, input, action and automation coverage.

The native integration has not been compiled or tested as a whole. Existing CPU characterization does not certify the replacement renderer. Checkboxes remain open until integration proof exists.

## Known gaps (current cycle)

- [ ] Compile and exercise the native login replacement against the preserved behavioral contract.
- [ ] Verify native artwork/layout, modal layering and fade through rendered proof.
- [ ] Verify an actual authentication flow and automation/dump compatibility at runtime.

## Out of scope

- Other-screen migration, toolkit optimization and rendering-quality changes.
- Authentication protocol redesign or new visible login controls.
- Production deployment and unmeasured performance claims.
