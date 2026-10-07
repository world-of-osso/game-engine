# Native HUD edit mode

Native editor shares the existing account layout file and character selection with [[forever-preset]]. The [contract](../../specs/hud-edit-mode.md) owns controls, defaults and acceptance evidence.

Verified: 2026-10-07. Implementation branch `hudeditmode`; runtime proof is recorded in the contract, not inferred from this page.

## Draft and projection

`GameClient.hud_editor` holds an `EditDraft`, selected key, pointer grab offset and editor canvas. Priority key handling runs after static-popup dismissal; dragging consumes native mouse events before gameplay. Save writes the draft through the existing layout store. Exit republishes the saved placements, discarding the draft.

`UiProjection::sync` calculates authored rectangles first. `hud_edit_layout::apply_placements` then translates each saved root and its descendants in that bounds map. Frame position, translation, margin and authored anchor fields are never overwritten. Reset therefore removes a map entry rather than trying to reconstruct old frame fields. Every subsequent reactive rebuild and UI-scale change starts from current authored geometry.

Placements store logical UI-unit offsets from screen anchors. Each canvas projects under its existing effective scale; viewport changes recompute anchor positions and clamp rendering without rewriting saved offsets.

## Mounted roots and previews

One registry and `collect_selection_boxes` supply live and offline labels, visibility filtering and selected art. Unmounted/hidden roots are skipped. Bars 2/3 are temporarily mounted by their production screen while editing. Bars 4/5 have editor-only empty preview roots; this does not add their missing gameplay consumers. Exit removes previews without changing preset visibility or Options settings.

`hud_edit_preview.rs` provides the secondary Godot capture methods. `hud_edit_live.gd` exercises native keys, mouse down/motion/up, Save, Escape, reset/discard and cold-process relog; it never writes mover offsets directly.

## Sources

- [Contract](../../specs/hud-edit-mode.md)
- [Native controller](../../../godot/rust/src/hud_edit.rs)
- [Projection](../../../godot/rust/src/ui/hud_edit_layout.rs)
- [Layout persistence](../../../godot/core/src/ui_layout_data/edit_layouts.rs)
- [Live fixture](../../../godot/tests/hud_edit_live.gd)

## See Also

- [[forever-preset]] — skin, preset and per-character layout selection.
- [[ui-system]] — reactive Screens and registry projection.
