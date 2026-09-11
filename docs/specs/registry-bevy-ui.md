# Registry-backed native Bevy UI

## Authority and compatibility

- `FrameRegistry` remains the UI model shared by engine code, existing `rsx!`/`Screen` authoring and JavaScript addons. Native Bevy UI is its rendering projection, not a separate set of screen definitions.
- Registry IDs, names, parent relationships, authored sizes/anchors/flex settings, widget state and existing mutation/error semantics remain intact. In particular, preserve current same-name `createFrame` reuse semantics; do not introduce a new ownership policy during rendering migration.
- Bevy owns layout calculation. The registry stores native-compatible authored dimensions, positioning, margins, translation and flex properties. Remove the old arbitrary-target anchor/dependency solver; computed bounds are observational input/measurement data and must never feed back into authored dimensions or positions.
- Bevy window/input events continue through registry hit testing, focus, text editing and event dispatch. Do not create a second text/edit-box authority or let native picking change existing hit-inset/order semantics.
- Registry mutation must update the corresponding native entities; unchanged values must not publish component changes. Reparenting, hiding, deleting, addon reload and screen teardown must preserve logical identity and remove obsolete projected visuals.

## Positioning API

- `setPos(x, y)` sets left/top pixel offsets (origin top-left, X rightward, Y downward) without changing positioning mode or target. It clears opposing right/bottom insets; callers wanting opposing-edge stretch author native insets explicitly.
- `setPosType(relative | absolute)` selects native layout participation. Default is relative; absolute removes the frame from its layout parent's flow, not from registry ownership.
- `setAnchor(parent | screen)` selects the layout parent. Default is the logical registry parent; screen projects beneath the screen canvas. Logical parent ownership, removal, effective visibility and alpha remain registry-controlled; positioning and clipping use the selected layout parent.
- RSX uses ordinary native-compatible positioning/inset/margin/translation attributes; legacy `anchor { point, relative_to, relative_point, ... }` and `setPoint` are removed, not emulated. Existing callers must explicitly restructure siblings or use the native parent layout.
- Centering, stretching and flex alignment remain native layout properties, not hidden operations inside `setPos`. Fixed zero dimensions remain zero; explicit auto and fill dimensions map to native auto/percentage sizing.

## Rendering

Replace the active toolkit Sprite/Text2d synchronization chain with native Node/ImageNode/Text rendering. Preserve existing backgrounds, texture sources/crops/tints/rotation, status fills, button states/highlights, nine/three slices, tiled textures, borders, text layout/styles/shadows/outlines, effective alpha, ordering and UI scale. Do not silently select the old renderer for unsupported native cases.

Legacy standalone rendering helpers may remain as compatibility/test utilities and reusable geometry code, but are not an active fallback backend. Texture-load errors must remain explicit; do not substitute a white texture for a failed native asset load.

## Edit-box caret

- Project raw `EditBoxData.text` and its byte cursor; passwords retain one displayed asterisk per raw byte.
- Cursor navigation/deletion must preserve UTF-8 boundaries.
- Show a two-pixel insertion caret at the actual shaped-text position, clipped to the edit-box content bounds.
- Use existing `blink_speed`; nonpositive values keep a focused caret visible. Reset blinking after focus/text/cursor changes; hide when unfocused, hidden or blocked by a modal.
- Native text/caret output is derived state. Authentication and addons continue reading/writing the registry values.

## Verification

- Compare registry-driven native layout, visuals, input focus, editing and lifecycle against the reduced native-layout contract using concrete fixtures. Replace tests for intentionally removed arbitrary-frame anchor semantics rather than retaining a hidden solver to satisfy them.
- Exercise real addon operations, including creation/reuse, setters, parent visibility/alpha, anchors and reload/removal, and inspect the resulting native Bevy output.
- Verify login/loading and representative decoration/widget families with actual layout/rendered evidence; do not substitute entity-type assertions for observable behavior.
- Preserve exact source/dependency/binary provenance and bounded native runtime limits. Keep independent proof scoped to the revisions actually checked.

## Current state

The native-only login/loading bypass was rolled back first: canonical engine revert `84003225`; retained worktree rollback `8662621e`. The active registry-native projector is toolkit `460e5e1`/`67b5287`, with UTF-8 cursor handling in `99632eb` and engine integration in `b4badaf9`. Verification remains in progress. Earlier native-only screen evidence does not certify this architecture.
