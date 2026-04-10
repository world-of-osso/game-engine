# Keybindings

The keybinding system covers in-world gameplay actions only. UI, shell, and debug controls are intentionally fixed. This split is by design, not placeholder behavior.

## Bindable Actions

Persisted with client options; configurable via Options → Keybindings.

**Movement**: forward, backward, strafe left, strafe right, jump, run toggle, autorun

**Camera**: turn left/right, pitch up/down, zoom in/out

**Targeting**: target nearest

**Action bar**: slots 1–12

**Audio**: toggle mute

## Fixed Inputs (Intentional)

| Input | Reason |
|-------|--------|
| `LMB + RMB` move-forward chord | Multi-button chord, not a single bindable action |
| Login screen keys | Screen-local text/focus/submit behavior |
| Char select navigation | Fixed to screen flow |
| Menu/options overlay | Navigation and modal dismissal must stay stable even if gameplay bindings break |
| Action-bar edit/debug controls | Editor affordances outside the player-facing binding set |

## Non-Goals of the Current System

The current implementation explicitly does not solve:

- Full client-wide rebinding
- Bindable menu/login/char-select UI navigation
- Bindable debug/editor controls
- Multi-input chords as first-class bindable actions

If scope expands, the source document should be updated before implementation so future fixed inputs remain clearly intentional.

## Sources

- [keybindings-scope.md](../../keybindings-scope.md) — scope definition and rationale

## See Also

- [[ui-addon-system]] — menu/overlay inputs that remain fixed live in the UI layer
