# Bevy UI login

Migrate `src/scenes/login/` from toolkit frames to Bevy UI entities, preserving login behavior and appearance. Other screens remain on their existing backend. See [UI system](../wiki/systems/ui-system.md).

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
- `src/scenes/login/form.rs` — standalone credential-field model: UTF-8-safe edits, limits and password display; not yet registered or connected to the running login screen.
- `src/ui/screens/login_component.rs` — existing declarative login layout and semantic names; migration reference.
- `src/ui/automation.rs` — shared automation queue and waits.
- `src/dump_systems.rs`, `src/dump.rs` — diagnostic tree requests and formatting.

## Tests asserting this spec

- `tests/unit/login_screen_tests.rs` — existing login behavior characterization.
- `tests/unit/login_screen_workflow_tests.rs` — existing automation/authentication workflows.
- `src/scenes/login/view_tests.rs` — existing login status/render synchronization characterization.
- `src/scenes/login/form.rs` — standalone model tests for filtering, limits, editing, password display and independent fields.

The standalone form model is covered independently, but the live login still uses toolkit edit boxes. Existing CPU characterization does not certify the replacement renderer. Checkboxes remain open until integration proof exists.

## Known gaps (current cycle)

- [ ] Register the standalone form model as the running login's single authority; replace toolkit field reads and mutations.
- [ ] Implement native login rendering.
- [ ] Adapt semantic automation and diagnostics.
- [ ] Verify modal layering, visual preservation and authentication behavior after migration.

## Out of scope

- Other-screen migration, toolkit optimization and rendering-quality changes.
- Authentication protocol redesign or new visible login controls.
- Production deployment and unmeasured performance claims.
