# Native account registration

The native login screen offers a username/password registration form using the authored rsx! login component. [Authentication](../authentication.md) describes the wire and token contract; [private live runs](../headless-live-run.md) describe approval testing.

## What it must do

- [ ] Create Account switches the login form to Register; Back to Login preserves entered credentials.
- [ ] Empty or whitespace-only username/password rejects submission with feedback and no request.
- [ ] Register emits `RegisterRequest { username, password }` on AuthChannel to the currently selected server, never a cached login token.
- [ ] Submitting disables Register and mode switching until a reply or transport failure.
- [ ] Pending approval retains Login, shows administrator-approval guidance, and neither authenticates nor persists a token, regardless of the response's success flag.
- [ ] A nonpending success persists its token, clears the roster and opens character select; a refusal shows the server error and permits correction/retry.
- [ ] After private administrator approval, Back to Login and password login reach character select.

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

- [ ] Targeted test results and live private-server captures pending.

## Out of scope

- Server approval policy changes, automatic approval polling, password resets, email fields, and production registration testing.
