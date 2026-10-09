# Keybindings

The keybinding system covers in-world gameplay actions only. UI, shell, and debug controls are intentionally fixed. This split is by design, not placeholder behavior.

## Bindable Actions

Persisted with client options; configurable via Options → Keybindings.

**Movement**: forward, backward, strafe left, strafe right, jump, run toggle, autorun

**Camera**: turn left/right, pitch up/down, zoom in/out

**Targeting**: target nearest

**Action bars**: main buttons 1–12 plus Action Bars 2–5 (`MULTIACTIONBAR1..4BUTTON1..12`), unbound by default on extras. Current native contract: [key bindings](../../specs/key-bindings.md); assignment/input flow: [[spellbook-action-bar]].

**Audio**: toggle mute (Ctrl+S)

**Interface**: panel toggles — character C, professions K, achievements Y, talents N, adventure guide J, social O, mail unbound (mailbox opens it), loot rules L, world map M

Bindings are a key, Shift+key, Ctrl+key, or mouse button; defaults follow Retail with no duplicates. All fire only in World input mode (no focused editbox, no game menu); see `src/ui_input_mode.rs`.

## Fixed Inputs (Intentional)

| Input | Reason |
|-------|--------|
| `LMB + RMB` move-forward chord | Multi-button chord, not a single bindable action |
| Login screen keys | Screen-local text/focus/submit behavior |
| Char select navigation | Fixed to screen flow |
| Menu/options overlay | Navigation and modal dismissal must stay stable even if gameplay bindings break |
| Action-bar edit/debug controls | Editor affordances outside the player-facing binding set |
| In-world `Escape` | Single chain: clear focus → cursor/spell cancel (hook) → top popup (hook) → close all panels → clear target → game menu |
| `F9` World Builder toggle | Fixed diagnostic UI control; active only when launched with `--world-builder` |

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
- [[world-builder]] — opt-in diagnostic sidebar controlled by F9
