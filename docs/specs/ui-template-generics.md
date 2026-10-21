# Authored UI template mechanisms

Port recurring behavior once from the Retail R cache, then migrate every audited panel and remove the old corrections in the same gated slice. The approved order is blend propagation, per-state regions, font objects and text sizing, named art, tooltip owners plus lifecycle/Escape/sounds, then keyboard navigation and scroll configuration. Existing domain models and server authority remain unchanged. [Audit](../../data/diagnostics/ui-template-generics-2026-10-20/report.md).

## What it must do

### Step 1: authored blend propagation

- [x] Default and explicit BLEND use normal alpha blending; ADD uses additive blending. No other mode is advertised or silently accepted.
- [x] Preserve different modes for the same atlas/source; material selection reads typed region state, never a frame name, FDID or atlas allowlist.
- [x] Migrate all audited model setters, six ignored declarations, selection-row art and button highlights to authored data. Delete the replaced corrections and material guesses; no alternate paths remain.
- [x] Preserve other properties unless the R cache explicitly establishes a corrected blend declaration. Record source differences separately from unrelated feature gaps.
- [x] Behavioral model/projection controls exercise real authored templates. One native pass covers all migrated panels in both skins; there are no per-attribute capture gates.

### Ordered subsequent slices

- [x] Step 2: Normal/Pushed/Disabled/Highlight/Checked regions carry source, crop, blend, tint and alpha through one mechanism; migrate listed consumers and delete old paths.
- [x] Step 3: source font-object inheritance and measured text sizing; migrated audited consumers and removed inherited-style copies. Lead integration/visual acceptance remains separate.
- [ ] Step 4: named slice/tab/button art; migrate listed consumers and delete source-name/FDID geometry guesses.
- [ ] Step 5: tooltip-owner/content bindings and lifecycle/Escape/sound descriptors; migrate listed consumers while preserving live-content refresh, placement, domain close actions and source ordering.
- [ ] Step 6: keyboard navigation and scroll configuration; migrate the audited list/search routing and scroll name dispatch while preserving domain eligibility and distinct row/pixel units.

## How it works

- [Existing UI system](../wiki/systems/ui-system.md).
- Core stores authored properties; the native projection consumes them. Source-script semantics are not inferred from atlas names.
- Button Normal/Pushed/Disabled/Highlight/Checked/DisabledChecked slots contain shared `ButtonTexture` records: source, blend, normalized crop, tint, alpha, desaturation, size and screen offset. Source changes preserve metadata. A metadata-only record has no drawable artwork: it must not replace existing normal art or emit an empty overlay. `ButtonData` selects base/highlight/checked regions; disabled checked art remains independently authored, not synthesized from an enabled checked source.
- Region alpha defaults to1, matching Retail's ordinary texture alpha. Hover opacity and dimensions are region metadata, not separate ButtonData fields. Perks rotation's pressed(+1,+1)/highlight0.6, Minimap disabled desaturation and Housing drawer(-4,0) belong to their authored declarations, not source-name projectors. Transmog hover sizes are declared alongside their artwork rather than repaired after sync.
- FontString declarations may name `font_object`; core resolves source font-parent chains into existing FontStringData before explicit local attributes. Rebuilds resolve the object again, so removing a local override restores inherited values; deliberate local face/size/color/shadow/justification still wins regardless declaration order. Unknown names fail explicitly. Native projection consumes the resolved data, not a panel-name font correction. Source subset is in toolkit `core/src/font_object.rs`; autoscale and arbitrary XML/Lua interpretation remain outside this slice.
- PlayerSpells font references are explicit per authored role: unshadowed System fonts for book text, shadowed GameFont category tabs, and the distinct specialization/talent roots. Local scale/size/color/justification remain overrides, not inferred font identities. TabSystem sizing measures the named normal font with shared font metrics rather than multiplying character count; source cap/padding/min/max policy is unchanged.
- Step4 named button templates/art kits populate state sources in core before local region overrides. Shared-red narrow caps follow `ThreeSliceButtonMixin:UpdateScale`; Panel strips retain fixed12px caps; source identity no longer selects geometry or default-skin policy. Cart-kit suffixes follow `UIButtonMixin:SetButtonArtKit`, and close controls use the shared state preset rather than static normal-only children.
- Named Inset/Dialog/Translucent and Panel/PanelTop/System/SystemTop art expand ordinary texture declarations in core. Dialog crossed side anchors normalize between source corner endpoints; Inset bottom corners extend1px and edges meet those corners. Rotated SystemTop and cropped75%-height PanelTop remain distinct. Tab width formulas remain independently source-derived with caller padding/min/max.
- The existing `ThreeSlice` now has independent cap extents, per-region UVs, vertical orientation and blend metadata. Shared core geometry feeds native projection and ordinary texture expansion; audited chat/auction/entrance/currency/header/scroll helpers no longer own this arithmetic. Existing Loading symmetric25px caps are unchanged. Step4 native acceptance remains pending.
- Minimap highlights default BLEND. UIPanelCloseButton's `RedButton-Highlight` explicitly uses ADD (R SharedUIPanelTemplates.xml139/146); this is distinct from the SharedButton Lua setter's omitted blend argument.

## Implementation inventory

- `godot/ui-model/src/` — production panel/template declarations and existing domain decisions.
- `godot/rust/src/ui/parts.rs` — native region projection.
- Sibling `ui-toolkit/core/src/attrs.rs` and widget data — canonical attribute/state representation.

## Tests asserting this spec

- Step3 core inheritance/rebuild/local-override controls4 and source-root1, model font/text-width controls3, and raster metric controls7 passed. `godot/tests/ui_template_fonts_matrix.gd` strict38/38 covers19 supplied scopes in both skins, all38 originals1920×1080,0 Godot errors. PNGs: `/syncthing/AgentShared/2026-10-21/ui-template-generics/step3-4d54697d4/`. Native checks preserve unshadowed book/tooltip, Meter13px/shadow, GroupLoot12px/shadow and map outline4. Final scoped Spellbook14/projection-allocation2 tests and helper cargo check passed with no compiler warnings; package format checks passed. Lead acceptance remains separate; not whole-client Retail pixel parity.
- Toolkit `core/src/attrs_layout_tests.rs` — source-identical normal/additive regions and unsupported-mode rejection.
- Step2 toolkit attrs21/projection30 passed, including independent crop/tint/alpha/Checked state and source changes. Whole ui-model integration1170 passed with0 failures (7 existing ignored) before native capture; scoped cargo check and format checks passed. `godot/tests/ui_template_regions_matrix.gd` strict16/16 covers8 supplied production states in both skins at1920×1080. All16 original PNGs inspected; PNGs: `/syncthing/AgentShared/2026-10-21/ui-template-generics/step2-1d857264b/`. Perks Normal-hover versus Pushed is strictly distinguished by base rect[0,0,48,48] versus[1,1,48,48]; the shared ADD highlight remains alpha0.6 at[0,0,48,48]. Native texture-identity comparison is not claimed.
- Engine model/projection `ui_template_blend` controls and `godot/tests/ui_template_blend_matrix.gd` cover authored panel data and its native projection. The matrix visits 21 panel/control groups in both skins; masked casting FX require raster inspection alongside model/projection mode controls. Unexecuted fixtures are not acceptance.

## Known gaps (current cycle)

- Step3 implementation/scoped proof/native capture is complete; lead gate remains pending. Steps4–6 remain unimplemented. Shared-red/UIPanel geometry routing is intentionally retained until Step4; Step2 does not claim named-art migration or realm/UDP parity.
- Step1 is landed at engine a982e08c7+ and toolkit e8c9f6f. Step2 lead integration gate follows its submitted E/T tips.

## Out of scope

- Unapproved blend modes, unrelated gameplay/server changes, or implementing later slices ahead of their order.
- Retired Bevy-renderer drift is not a native Godot acceptance claim.
