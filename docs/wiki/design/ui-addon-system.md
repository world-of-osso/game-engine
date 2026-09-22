# UI & Addon System

## Authority

`FrameRegistry` is the UI authority. It owns frames, parentage, authored properties, resolved layout, visibility/effective alpha, focus, hit testing, text editing, and addon ownership. `rsx!`/`Screen` remain registry authors through `screen.sync(&shared, registry)`.

The native Bevy renderer is a one-way runtime projection: registry state produces `Node`, `ImageNode`, and `Text` entities. Bevy layout and arbitrary ECS component changes never write authored properties back into the registry. Raw Bevy input continues through registry hit testing, focus, edit-box updates, and event dispatch.

## Addons

The implemented addon backend is QuickJS, not the earlier planned WASM/wasmtime design. Addon JavaScript calls the engine frame API, which mutates owned registry frames. It does not author native entities directly.

`createFrame` follows existing ownership logic: same-name, same-type reuse is permitted through `collect_owned`/`ensure_owned`. Rendering migration does not add an engine-frame access restriction.

Addon reload and teardown remove owned registry state; the native projection removes the corresponding derived entities. Toolkit `7e0323f` clears registry/UI-state focus for removed IDs and unregisters their EventBus listeners before native despawn; engine `de5d2bbf`/`018a7724` regress addon reparent-reload cleanup and focused EditBox unload.

## Edit boxes

Caret output is derived from `EditBoxData.text`, its byte cursor, `blink_speed`, and `UiState.focused_frame`. No login-specific form or native caret state is authoritative. Password display retains its existing byte-based masking behavior; cursor movement and deletion preserve UTF-8 scalar boundaries.

Addon-owned EditBoxes initialize `WidgetData::EditBox` and mouse enablement through `initialize_widget` (`fac467b8`), so registry hit testing can select, redirect, and clear focus through the normal input path.

## Migration status

The native-only login/loading bypass was rolled back first: engine `84003225`, retained worktree `8662621e`. The active registry-native projector is toolkit `460e5e1`/`67b5287`; UTF-8 cursor correction is `99632eb`; engine integration is `b4badaf9`. Later conformance work retracts unchanged registry writes (`1072518`), removes stale focus/listeners (`7e0323f`), deletes obsolete anchor geometry (`e584425`), and covers native addon cleanup/EditBox focus (`de5d2bbf`, `018a7724`, `fac467b8`). Verification remains in progress. Earlier native-only screen proof does not certify this architecture.

## Sources

- [addon operation application](../../../src/ui/addon_runtime/apply.rs) — owned-frame initialization and EditBox widget setup
- [addon lifecycle regressions](../../../src/ui/addon_runtime/tests.rs) — reparent/reload, native cleanup, and focused EditBox unload coverage
- [registry removal cleanup](../../../../ui-toolkit/src/registry.rs) — focus clearing and listener unregistration inputs

## See also

- [UI system](../systems/ui-system.md)
- [Registry-backed native Bevy UI](../../specs/registry-bevy-ui.md)
