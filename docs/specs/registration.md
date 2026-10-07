# Native account registration

The native login screen offers a username/password registration form using the authored rsx! login component. [Authentication](../authentication.md) describes the wire and token contract; [private live runs](../headless-live-run.md) describe approval testing.

## What it must do

- [x] Create Account switches the login form to Register; Back to Login preserves entered credentials.
- [x] Empty or whitespace-only username/password rejects submission with feedback and no request.
- [x] Register emits `RegisterRequest { username, password }` on AuthChannel to the currently selected server, never a cached login token.
- [x] Submitting disables Register and mode switching; a reply restores submission.
- [x] Pending approval retains Login, shows administrator-approval guidance, and neither authenticates nor persists a token, regardless of the response's success flag.
- [x] A nonpending success persists its token, clears the roster and opens character select; a refusal shows the server error and permits correction/retry.
- [x] After private administrator approval, Back to Login and password login reach character select.

## How it works

- [Authentication](../authentication.md)
- [Native UI automation](native-ui-automation.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/login_component.rs` — authored login/registration form and pending button state.
- `godot/rust/src/ui/mod.rs` — registration mode projection on existing login canvas.
- `godot/rust/src/lib.rs` — selected-server registration submission and feedback.
- `godot/session/src/lib.rs` — validation, token-free request and registration reply decisions.
- `godot/rust/src/account.rs` — AuthChannel transport and response decoding.

## Tests asserting this spec

- `godot/ui-model/tests/login.rs` — visible entry, form switching, preserved inputs and disabled buttons.
- `godot/session/tests/session.rs` — validation, emitted request, pending/success/refusal transitions.
- `godot/tests/registration_flow.gd` — rendered real-input registration, private pending reply, approval and password login.

## Known gaps (current cycle)

No registration-flow gaps remain in the requested scope.

## Current proof

- Production `e424d622` / `02099180`; corrected UI assertion `6d240e1a`; live fixture `4f03f962`.
- Six targeted session registration tests pass in `/tmp/claude/registration-green.out`. That invocation also exposed an incorrect UI assertion against `ButtonData.enabled` alone; native projection additionally checks `ButtonState::Disabled`. Corrected full login test target passes 4/4 in `/tmp/claude/registration-login-green.out`.
- Matching local native extension and CLI build passes in `/tmp/claude/registration-build.out`; Rust formatting passes in `/tmp/claude/registration-fmt-final.out`.
- `data/diagnostics/registration-2026-10-07/` contains `proof-ledger.json`, `client2.log`, `pending-ui.txt`, and inspected `shots/{pending,character-select}.png` with adjacent state JSON. Real mouse/key input registered `fb_regtest1` on private UDP 5300; pending state saved no token; approval used only the private admin socket; subsequent password login reached empty-roster character select and saved the endpoint token in canonical data. Two rendered client launches used; first observer incorrectly read `Button.text` instead of the authored child Label and never submitted registration.
- Weston/Dozen capability warnings and Godot exit resource-leak diagnostics remain in the retained logs. Registration evidence does not establish leak-free shutdown or full client parity.

## Out of scope

- Server approval policy changes, automatic approval polling, password resets, email fields, and production registration testing.
