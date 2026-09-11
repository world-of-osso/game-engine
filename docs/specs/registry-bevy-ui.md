# Registry-backed native Bevy UI

## Authority and compatibility

- `FrameRegistry` remains the UI model shared by engine code, existing `rsx!`/`Screen` authoring and JavaScript addons. Native Bevy UI is its rendering projection, not a separate set of screen definitions.
- Registry IDs, names, parent relationships, authored sizes/anchors/flex settings, widget state and existing mutation/error semantics remain intact. In particular, preserve current same-name `createFrame` reuse semantics; do not introduce a new ownership policy during rendering migration.
- The existing registry layout solver owns resolved rectangles and anchor dependencies. Bevy nodes use those rectangles; computed Bevy layout must not write back into authored anchors or dimensions.
- Bevy window/input events continue through registry hit testing, focus, text editing and event dispatch. Do not create a second text/edit-box authority or let native picking change existing hit-inset/order semantics.
- Registry mutation must update the corresponding native entities; unchanged values must not publish component changes. Reparenting, hiding, deleting, addon reload and screen teardown must preserve logical identity and remove obsolete projected visuals.

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

- Compare registry-driven native layout, visuals, input focus, editing and lifecycle against the existing registry contract using concrete fixtures.
- Exercise real addon operations, including creation/reuse, setters, parent visibility/alpha, anchors and reload/removal, and inspect the resulting native Bevy output.
- Verify login/loading and representative decoration/widget families with actual layout/rendered evidence; do not substitute entity-type assertions for observable behavior.
- Preserve exact source/dependency/binary provenance and bounded native runtime limits. Keep independent proof scoped to the revisions actually checked.

## Current state

The native-only login/loading bypass was rolled back first: canonical engine revert `84003225`; retained worktree rollback `8662621e`. The active registry-native projector is toolkit `460e5e1`/`67b5287`, with UTF-8 cursor handling in `99632eb` and engine integration in `b4badaf9`. Verification remains in progress. Earlier native-only screen evidence does not certify this architecture.
