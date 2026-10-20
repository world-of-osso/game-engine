# Authored UI template mechanisms

Port recurring behavior once from the Retail R cache, then migrate every audited panel and remove the old corrections in the same gated slice. The approved order is blend propagation, per-state regions, font objects, named art, lifecycle/Escape/sounds, then tooltip owners. Existing domain models and server authority remain unchanged. [Audit](../../data/diagnostics/ui-template-generics-2026-10-20/report.md).

## What it must do

### Step 1: authored blend propagation

- [ ] Default and explicit BLEND use normal alpha blending; ADD uses additive blending. No other mode is advertised or silently accepted.
- [ ] Preserve different modes for the same atlas/source; material selection reads typed region state, never a frame name, FDID or atlas allowlist.
- [ ] Migrate all audited model setters, six ignored declarations, selection-row art and button highlights to authored data. Delete the replaced corrections and material guesses; no alternate paths remain.
- [ ] Preserve other properties unless the R cache explicitly establishes a corrected blend declaration. Record source differences separately from unrelated feature gaps.
- [ ] Behavioral model/projection controls exercise real authored templates. One native pass covers all migrated panels in both skins; there are no per-attribute capture gates.

### Ordered subsequent slices

- [ ] Step 2: Normal/Pushed/Disabled/Highlight/Checked regions carry source, crop, blend, tint and alpha through one mechanism; migrate listed consumers and delete old paths.
- [ ] Step 3: source font-object inheritance; migrate listed consumers and delete local inherited-style corrections.
- [ ] Step 4: named slice/tab/button art; migrate listed consumers and delete source-name/FDID geometry guesses.
- [ ] Step 5: lifecycle/Escape/sound descriptors; migrate listed consumers while retaining domain close actions and source ordering.
- [ ] Step 6: tooltip-owner bindings; migrate listed consumers while preserving shared live-content refresh and placement.

## How it works

- [Existing UI system](../wiki/systems/ui-system.md).
- Core stores authored properties; the native projection consumes them. Source-script semantics are not inferred from atlas names.
- Button Normal/Pushed/Disabled/Highlight slots contain shared `ButtonTexture { source, blend_mode }` records. Source changes preserve their authored mode. A mode-only record has no drawable artwork: it must not replace existing normal art or emit an empty highlight. Step 2 extends this record rather than adding a second state-art path.
- Geometry-only routing remains until Step 4; all routed parts consume the chosen state's mode. Minimap highlights default BLEND. UIPanelCloseButton's `RedButton-Highlight` explicitly uses ADD (R SharedUIPanelTemplates.xml139/146); this is distinct from the SharedButton Lua setter's omitted blend argument.

## Implementation inventory

- `godot/ui-model/src/` — production panel/template declarations and existing domain decisions.
- `godot/rust/src/ui/parts.rs` — native region projection.
- Sibling `ui-toolkit/core/src/attrs.rs` and widget data — canonical attribute/state representation.

## Tests asserting this spec

- Toolkit `core/src/attrs_layout_tests.rs` — source-identical normal/additive regions and unsupported-mode rejection.
- Engine model/projection `ui_template_blend` controls and `godot/tests/ui_template_blend_matrix.gd` cover authored panel data and its native projection. The matrix visits 21 panel/control groups in both skins; masked casting FX require raster inspection alongside model/projection mode controls. Unexecuted fixtures are not acceptance.

## Known gaps (current cycle)

- [ ] Complete Step 1 and submit coordinated toolkit/engine tips for the lead gate.

## Out of scope

- Unapproved blend modes, unrelated gameplay/server changes, or implementing later slices ahead of their order.
- Retired Bevy-renderer drift is not a native Godot acceptance claim.
