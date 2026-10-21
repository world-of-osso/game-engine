# Authored UI template mechanisms

Port recurring behavior once from the Retail R cache, then migrate every audited panel and remove the old corrections in the same gated slice. The approved order is blend propagation, per-state regions, font objects, named art, lifecycle/Escape/sounds, then tooltip owners. Existing domain models and server authority remain unchanged. [Audit](../../data/diagnostics/ui-template-generics-2026-10-20/report.md).

## What it must do

### Step 1: authored blend propagation

- [x] Default and explicit BLEND use normal alpha blending; ADD uses additive blending. No other mode is advertised or silently accepted.
- [x] Preserve different modes for the same atlas/source; material selection reads typed region state, never a frame name, FDID or atlas allowlist.
- [x] Migrate all audited model setters, six ignored declarations, selection-row art and button highlights to authored data. Delete the replaced corrections and material guesses; no alternate paths remain.
- [x] Preserve other properties unless the R cache explicitly establishes a corrected blend declaration. Record source differences separately from unrelated feature gaps.
- [x] Behavioral model/projection controls exercise real authored templates. One native pass covers all migrated panels in both skins; there are no per-attribute capture gates.

### Ordered subsequent slices

- [x] Step 2: Normal/Pushed/Disabled/Highlight/Checked regions carry source, crop, blend, tint and alpha through one mechanism; migrate listed consumers and delete old paths.
- [ ] Step 3: source font-object inheritance; migrate listed consumers and delete local inherited-style corrections.
- [ ] Step 4: named slice/tab/button art; migrate listed consumers and delete source-name/FDID geometry guesses.
- [ ] Step 5: lifecycle/Escape/sound descriptors; migrate listed consumers while retaining domain close actions and source ordering.
- [ ] Step 6: tooltip-owner bindings; migrate listed consumers while preserving shared live-content refresh and placement.

## How it works

- [Existing UI system](../wiki/systems/ui-system.md).
- Core stores authored properties; the native projection consumes them. Source-script semantics are not inferred from atlas names.
- Button Normal/Pushed/Disabled/Highlight/Checked/DisabledChecked slots contain shared `ButtonTexture` records: source, blend, normalized crop, tint, alpha, desaturation, size and screen offset. Source changes preserve metadata. A metadata-only record has no drawable artwork: it must not replace existing normal art or emit an empty overlay. `ButtonData` selects base/highlight/checked regions; disabled checked art remains independently authored, not synthesized from an enabled checked source.
- Region alpha defaults to1, matching Retail's ordinary texture alpha. Hover opacity and dimensions are region metadata, not separate ButtonData fields. Perks rotation's pressed(+1,+1)/highlight0.6, Minimap disabled desaturation and Housing drawer(-4,0) belong to their authored declarations, not source-name projectors. Transmog hover sizes are declared alongside their artwork rather than repaired after sync.
- Geometry-only routing remains until Step 4; all routed parts consume the chosen state's mode. Minimap highlights default BLEND. UIPanelCloseButton's `RedButton-Highlight` explicitly uses ADD (R SharedUIPanelTemplates.xml139/146); this is distinct from the SharedButton Lua setter's omitted blend argument.

## Implementation inventory

- `godot/ui-model/src/` — production panel/template declarations and existing domain decisions.
- `godot/rust/src/ui/parts.rs` — native region projection.
- Sibling `ui-toolkit/core/src/attrs.rs` and widget data — canonical attribute/state representation.

## Tests asserting this spec

- Toolkit `core/src/attrs_layout_tests.rs` — source-identical normal/additive regions and unsupported-mode rejection.
- Step2 toolkit attrs21/projection30 passed, including independent crop/tint/alpha/Checked state and source changes. Whole ui-model integration1170 passed with0 failures (7 existing ignored) before native capture; scoped cargo check and format checks passed. `godot/tests/ui_template_regions_matrix.gd` strict16/16 covers8 supplied production states in both skins at1920×1080. All16 original PNGs inspected; PNGs: `/syncthing/AgentShared/2026-10-21/ui-template-generics/step2-1d857264b/`. Perks Normal-hover versus Pushed is strictly distinguished by base rect[0,0,48,48] versus[1,1,48,48]; the shared ADD highlight remains alpha0.6 at[0,0,48,48]. Native texture-identity comparison is not claimed.
- Engine model/projection `ui_template_blend` controls and `godot/tests/ui_template_blend_matrix.gd` cover authored panel data and its native projection. The matrix visits 21 panel/control groups in both skins; masked casting FX require raster inspection alongside model/projection mode controls. Unexecuted fixtures are not acceptance.

## Known gaps (current cycle)

- Steps3–6 remain unimplemented. Shared-red/UIPanel geometry routing is intentionally retained until Step4; Step2 does not claim named-art migration or realm/UDP parity.
- Step1 is landed at engine a982e08c7+ and toolkit e8c9f6f. Step2 lead integration gate follows its submitted E/T tips.

## Out of scope

- Unapproved blend modes, unrelated gameplay/server changes, or implementing later slices ahead of their order.
- Retired Bevy-renderer drift is not a native Godot acceptance claim.
