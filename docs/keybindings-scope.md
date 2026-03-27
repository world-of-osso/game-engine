# Keybindings Scope

This document records the intentional scope boundary for the current configurable keybinding system.

## Intent

The current keybinding work is meant to cover in-world gameplay input that benefits from player customization without turning every input path in the client into a bindable surface immediately.

That means the current implementation is intentionally split into:

- bindable in-world gameplay actions
- fixed UI, shell, and debug/editor controls

This is not an accident and should not be treated as partial or placeholder behavior inside the current scope.

## Bindable In-World Actions

The configurable keybinding system currently covers:

- movement: forward, backward, strafe left, strafe right, jump, run toggle, autorun
- camera keyboard controls: turn left/right, pitch up/down, zoom in/out
- targeting: target nearest
- action bar slots: 1 through 12
- audio: toggle mute

These bindings are persisted with client options and edited through the Options -> Keybindings screen.

## Intentionally Fixed Inputs

The following inputs remain fixed on purpose:

- `LMB + RMB` move-forward chord
  - This is treated as a classic WoW-style mouse movement chord, not a normal rebindable action.
- login screen bindings
  - Text input, focus movement, submit, and related screen-local controls remain screen-defined.
- char select bindings
  - Selection/navigation keys remain fixed to the screen flow.
- menu/options overlay bindings
  - Menu navigation, modal dismissal, and key capture controls remain fixed.
- action-bar edit/debug controls
  - Layout editing keys remain fixed and separate from gameplay bindings.

## Why These Stay Fixed

These paths are fixed for different reasons:

- Some are screen-local UI behavior, not gameplay actions.
- Some are shell/navigation controls that should remain stable even if gameplay bindings are broken.
- Some are editor/debug affordances that are intentionally outside the player-facing binding set.
- The mouse move-forward chord is a special combined input, not a single-button action.

## Non-Goals Of The Current System

The current keybinding system does not attempt to solve:

- full client-wide rebinding
- bindable menu navigation
- bindable login or char select UI behavior
- bindable debug/editor controls
- multi-input chords as first-class bindable actions

If scope expands later, this document should be updated before implementation so future fixed inputs are clearly intentional rather than ambiguous.
