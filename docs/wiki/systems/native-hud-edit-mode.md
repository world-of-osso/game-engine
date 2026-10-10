# Native HUD edit mode

Native editor shares account-scoped layout storage and character selection with [[forever-preset]]. The [contract](../../specs/hud-edit-mode.md) owns controls, defaults and acceptance evidence.

Verified: 2026-10-09. Implementation branch `hudeditmode`; runtime proof is recorded in the contract, not inferred from this page.

## Draft and projection

`GameClient.hud_editor` holds an `EditDraft`, selected key, pointer grab offset and editor canvas. Priority key handling runs after static-popup dismissal; dragging consumes native mouse events before gameplay. Save writes the draft through the existing layout store. All native exit routes persist pending placements through the same Save transition before clearing the draft and republishing the saved layout. Unchanged exits create no copy; edited presets use the existing first-unused `Layout N` flow. The [contract](../../specs/hud-edit-mode.md#autosave-on-exit-user-decision-2026-10-09) records the explicit autosave deviation from Retail.

The host routes every manager and Options HUD operation through `Account::hud_layout_path`: the realm and server-accepted username select an account directory. `SessionEffect::PersistToken` publishes the authenticated owner only after success and records token ownership privately, so rotated credential tokens and token-only reconnects reach the same layouts. Failed/unidentified logins do not expose account layouts. The [contract](../../specs/hud-edit-mode.md) owns the namespace, unknown-token behavior and ownerless legacy-file boundary; window-position paths are not changed.

`UiProjection::sync` calculates authored rectangles first. `hud_edit_layout::apply_placements` then translates each saved root and its descendants in that bounds map. Frame position, translation, margin and authored anchor fields are never overwritten. Reset therefore removes a map entry rather than trying to reconstruct old frame fields. Every subsequent reactive rebuild and UI-scale change starts from current authored geometry.

Placements store logical UI-unit offsets from screen anchors. Each canvas projects under its existing effective scale; viewport changes recompute anchor positions and clamp rendering without rewriting saved offsets.

## Mounted roots and previews

One registry and `collect_selection_boxes` supply live and offline labels, visibility filtering and selected art. Idle selections show no label; hover shows instructions and selection shows the system name. Labels draw text only over the selection highlight; Retail's `EditModeSystemTemplates.xml:35–50` provides no label-only backing panel or texture. Explicit per-selection paint levels keep selected labels above intersecting highlights even when reactive diffing retains child order.

The manager's action rows use Retail's 15-unit side and16-unit bottom insets; the widened Reset Selected button keeps its full label inside the art. The [contract](../../specs/hud-edit-mode.md#manager-action-row-padding-2026-10-09) owns the compact manager geometry. The manager prefers Retail's top-centred y=100. `find_panel_position` chooses the nearest clear rectangle from mover-edge candidates; only the manager moves, never preset HUD roots. Its title drag keeps a transient, screen-clamped position in `EditDraft`, cleared on exit. Native editor selections use Fullscreen and the manager FullscreenDialog inside canvas 100: the relative layering matches Retail's MEDIUM selection / DIALOG manager while staying above independently mounted gameplay canvases. Unmounted/hidden roots are skipped. Unset bars 2/3 are temporarily mounted by their production screen while editing; explicit Off suppresses their movers. Bars 4/5 now use the production action-bar screen only when enabled in [Action Bars options](../../specs/extra-action-bars.md); no editor-only side roots remain. The existing settings store persists visibility; tracker placement stays unchanged even when enabled bars overlap it (user decision 2026-10-08).

`hud_edit_preview.rs` provides the secondary Godot capture methods. Its default inventory mounts 19 roots (21 when side bars 4/5 are enabled), including the bottom-bar previews and micro menu, five party members and one raid member per subgroup. Those explicit offline fixtures do not change preset settings. `GODOT_HUDEDIT_MOVER_INVENTORY=1` makes the existing capture helper select every mover, verify manager clearance and save each rendered label crop plus the selected tracker screen.

The manager receives account layout names and the inline name draft on each refresh; New's disabled atlas is selected for trimmed-empty/preset/occupied names. Persistence also rejects invalid New requests; only preset Save retains automatic `Layout N` naming. Delete stores the requested name in `EditDraft.pending_delete` and renders the native Yes/No dialog above a mouse blocker. Cancel clears only that request, preserving active layout and draft selection; confirmation deletes the requested layout and reloads its skin preset. Escape/F10 cancels an open confirmation before exiting Edit Mode. [Contract](../../specs/hud-edit-mode.md) owns the 2026-10-09 user decision and Retail text/source.

The manager's optional-system checkboxes use Retail's basic order, filtered to registered native movers ([contract and Retail citations](../../specs/hud-edit-mode.md#manager-system-checkboxes-2026-10-09)). `edit_mode.show_systems` is an account-wide map beside layouts, not a layout setting. Clicks persist immediately and publish the map to the shared selection collector; unchecked categories lose only mover art and selected/drag/hover state. Authored roots, gameplay visibility and Options bar/party settings stay unchanged. Entry reloads the authenticated account map; panel state redraws the checkmarks. The manager reflows its own controls to fit the list without moving HUD frames.

`apply_manager_action` is the persisted transition used by native clicks and CPU behavioral tests. The tests dispatch through the registry's actual enabled-button action queue before applying draft, layout cycling and RON changes; they do not instantiate a live `GameClient`. `capture_ui_screen.gd` shares its two-skin offline capture function with the live fixture, then clears transient preview state before `GameClient` creation. `hud_edit_live.gd` exercises native keys, mouse down/motion/up, Save, Escape, reset/discard and normal logout/login relog; it never writes mover offsets directly.

## Fractional opacity boundary

The pinned sibling `ui-toolkit-macros` parser truncates fractional numeric literals (`parse_attr_value`, `LitFloat` branch). The mover's literal `alpha: 0.7` therefore produced zero opacity despite correct art and geometry; waiting for textures did not fix it. The authorized client code uses the macro's numeric-expression path, `alpha: {0.7}`, and tests effective projected opacity. The dependency is outside this change's authorized paths; retire this syntax constraint when its literal parser preserves fractions.

## Sources

- [Contract](../../specs/hud-edit-mode.md)
- [Native controller](../../../godot/rust/src/hud_edit.rs)
- [Projection](../../../godot/rust/src/ui/hud_edit_layout.rs)
- [Layout persistence](../../../godot/core/src/ui_layout_data/edit_layouts.rs)
- [Live fixture](../../../godot/tests/hud_edit_live.gd)

## See Also

- [[forever-preset]] — skin, preset and per-character layout selection.
- [[ui-system]] — reactive Screens and registry projection.
