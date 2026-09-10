# Native login tint channels

## Symptom

The first bounded runtime capture wrote `data/diagnostics/login-bevy-ui/runtime/menu-final/menu.webp`, but native login image/text nodes appeared as opaque rectangular blocks. This image predates the correction; it does not prove the corrected presentation.

## Root cause

`sync_login_view` applied `LoginTint` to every available visual channel on an entity. Native Bevy `Node` entities default to `BackgroundColor(Color::NONE))`; setting that background while tinting an `ImageNode` or `Text` turned transparent node backgrounds opaque. The issue was in presentation synchronization, not artwork loading or camera ordering.

## Fix and proof

Engine `f3ae396c` makes tint application mutually exclusive: image nodes update `ImageNode::color`, text nodes update `TextColor`, and background-only nodes update `BackgroundColor`. The regression uses actual `Node + ImageNode` and `Node + Text` fixtures: it failed before the fix and passes after it.

At `f3ae396c`, all seven focused native-view presentation cases passed in 0.053176801 seconds. This adds one distinct tint regression to the 61 previously verified focused cases, for 62 distinct cases total. Evidence: [tint proof ledger](../../../data/diagnostics/login-bevy-ui/tint-fix/proof-ledger.md).

## Remaining proof

A new bounded rendered capture must inspect the corrected login and menu compositing. Physical input, menu appearance, and real authentication remain unverified.

## Sources

- [native view synchronization](../../../src/scenes/login/native_view.rs)
- [presentation tests](../../../src/scenes/login/native_view_tests.rs)
- [pre-fix menu capture](../../../data/diagnostics/login-bevy-ui/runtime/menu-final/menu.webp)
- [tint proof ledger](../../../data/diagnostics/login-bevy-ui/tint-fix/proof-ledger.md)

## See also

- [[login-camera-startup-order]]
- [[ui-system]]
