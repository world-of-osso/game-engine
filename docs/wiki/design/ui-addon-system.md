# UI & Addon System

## Authority

`FrameRegistry` is the UI authority. It owns frames, parentage, authored properties, resolved layout, visibility/effective alpha, focus, hit testing, text editing, and addon ownership. `rsx!`/`Screen` remain registry authors through `screen.sync(&shared, registry)`.

The native Bevy renderer is a one-way runtime projection: registry state produces `Node`, `ImageNode`, and `Text` entities. Bevy layout and arbitrary ECS component changes never write authored properties back into the registry. Raw Bevy input continues through registry hit testing, focus, edit-box updates, and event dispatch.

## Addons

The implemented addon backend is QuickJS, not the earlier planned WASM/wasmtime design. Addon JavaScript calls the engine frame API, which mutates owned registry frames. It does not author native entities directly.

`createFrame` follows existing ownership logic: same-name, same-type reuse is permitted through `collect_owned`/`ensure_owned`. Rendering migration does not add an engine-frame access restriction.

Addon reload and teardown remove owned registry state; the native projection removes the corresponding derived entities.

## Edit boxes

Caret output is derived from `EditBoxData.text`, its byte cursor, `blink_speed`, and `UiState.focused_frame`. No login-specific form or native caret state is authoritative. Password display retains its existing byte-based masking behavior; cursor movement and deletion preserve UTF-8 scalar boundaries.

## Migration status

The native-only login/loading bypass was rolled back first: engine `84003225`, retained worktree `8662621e`. The active registry-native projector is toolkit `460e5e1`/`67b5287`; UTF-8 cursor correction is `99632eb`; engine integration is `b4badaf9`. Verification remains in progress. Earlier native-only screen proof does not certify this architecture.

## See also

- [UI system](../systems/ui-system.md)
- [Registry-backed native Bevy UI](../../specs/registry-bevy-ui.md)
