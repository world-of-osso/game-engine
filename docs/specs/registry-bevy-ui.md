# Registry-backed native Bevy UI

## Authority and compatibility

- `FrameRegistry` remains the UI model shared by engine code, existing `rsx!`/`Screen` authoring and JavaScript addons. Native Bevy UI is its rendering projection, not a separate set of screen definitions.
- Registry IDs, names, logical parent relationships, authored native insets, margins, translation, size and flex settings, widget state and existing mutation/error semantics remain intact. In particular, preserve current same-name `createFrame` reuse/type policy; do not introduce a new ownership policy during rendering migration.
- Bevy owns layout calculation. The registry stores authored native properties; Bevy writes computed bounds back only for observation by input and measurement. There is no `AnchorPoint`, `set_point`/`setPoint`, arbitrary target relationship, anchor solver, or registry-owned screen text measurement; only `AnchorTarget::Parent|Screen` remains.
- Bevy window/input events continue through registry hit testing, focus, text editing and event dispatch. Do not create a second text/edit-box authority or let native picking change existing hit-inset/order semantics.
- Registry mutation must update the corresponding native entities. A pending write that leaves the frame unchanged must retract its dirty publication at the registry mutation boundary, so native components are not changed. Reparenting, hiding, deleting, addon reload and screen teardown must preserve logical identity and remove obsolete projected visuals.
- Removing a frame must clear both registry and UI-state focus when they reference it, and unregister its EventBus listeners before native descendants are despawned. Addon-owned EditBoxes must initialize their widget data and mouse input so registry hit testing can focus them.

## Positioning API

- `setPos(x, y)` sets left/top pixel offsets (origin top-left, X rightward, Y downward) without changing positioning mode or target. It clears opposing right/bottom insets; callers wanting opposing-edge stretch author native insets explicitly.
- `setPosType(relative | absolute)` selects native layout participation. Default is relative; absolute removes the frame from its layout parent's flow, not from registry ownership.
- `setAnchor(parent | screen)` selects the layout parent. Default is `parent`; `screen` projects beneath the screen canvas. Logical parent ownership, removal, effective visibility and alpha remain registry-controlled when the layout parent is `screen`.
- RSX accepts `pos_type`, `pos_x`, `pos_y`, `left`, `right`, `top`, `bottom`, `translate_x`, `translate_y`, `margin_*`, `anchor: parent|screen`, and `width`/`height` values `auto` or `fill`. Legacy `anchor { ... }` is rejected by the macro and runtime; it is not emulated.
- Centering, stretching and flex alignment remain native layout properties, not hidden operations inside `setPos`. Fixed zero dimensions remain zero; explicit `auto` and `fill` dimensions map to native auto/percentage sizing.

## Rendering

Replace the active toolkit Sprite/Text2d synchronization chain with native Node/ImageNode/Text rendering. Preserve existing backgrounds, texture sources/crops/tints/rotation, status fills, button states/highlights, nine/three slices, tiled textures, borders, text layout/styles/shadows/outlines, effective alpha, ordering and UI scale. Do not silently select the old renderer for unsupported native cases.

Legacy standalone rendering helpers may remain as compatibility/test utilities and reusable geometry code, but are not an active fallback backend. Texture-load errors must remain explicit; do not substitute a white texture for a failed native asset load.

Menu titles attached above panels must overlap the panel by two pixels; a gap is a visual regression. Loading zone, tip, status, and percentage labels must stay legible above the artwork, bar background, and progress fill at every progress value.

Character-creation race/class icons must preserve all 32 authored artworks using exact local-listfile FileDataIDs and the existing local CASC loader. They must render without the obsolete `/home/osso/Projects/wow/Interface/` export directory; alternate art, alternate directories, and text substitutes are not compatibility paths. `tests/unit/charcreate_icon_source_tests.rs` exercises all 32 through local resolution/decoding and compares their RSX-driven native image content.

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

Registry-native UI was merged into engine `master` by `6fa018d6`; bounded nine-second captures were made from engine source `4415f5fc` and toolkit `791b282`. They verify visible loading labels above the bar/fill and a menu title from y337–373 overlapping its panel from y371 by two pixels. Earlier bounded captures at compatible runtime code verify login, local authentication to character select, UTF-8 edit input (`adminé`, byte cursor `7`), and sampled caret blinking. See `data/diagnostics/native-layout-api/{fixed-loading,fixed-menu,final-auth,final-caret}/`.

CPU proof is revision-scoped: engine `965c462d` integration covers 887 cases; toolkit `bc901a6` covers 42 native-render cases; toolkit `821c2a0` covers 72 remaining registry/attrs/Screen/diff/parser cases. The behavior-preserving toolkit `791b282` extraction has four focused native-render cases plus check/format proof, while engine visual follow-up has 36 affected cases. Toolkit `1072518` adds registry-boundary unchanged-write retraction; `7e0323f` adds focused-frame and listener teardown; `e584425` deletes obsolete anchor geometry and converts native layout fixtures to computed bounds. Engine `de5d2bbf`/`018a7724` add addon native-cleanup and stale-focus regressions, `fac467b8` covers addon EditBox initialization/click focus, and `6ad8cf75` replaces character-select anchor-math assertions with computed-bounds assertions. Existing global engine format failure is 104 vendor paths. `binrw` was updated to 0.15.2 to remove its prior future-incompatibility warning. Missing character-screen assets limit rendered character validation. Do not treat these bounded tests or captures as full lifecycle, asset-complete, or final acceptance.
