# Shared UI frame ordering

Approved design; implementation started September 10, 2026. Consolidate ordering preparation in `ui-toolkit`'s `UiPlugin` render pipeline while preserving standalone renderer setup. Current code independently sorts in six render systems. Architecture and compatibility rationale: [shared ordering design](../wiki/design/ui-frame-order.md).

## What it must do

### Ordering and rendering

- [ ] Prepare one ordered ID list and one ID-to-index map per enabled plugin render update, after layout and button preparation.
- [ ] Preserve current membership: visible frames with positive effective width and height. Preserve ordering by strata, frame level, raise order, then frame ID.
- [ ] All six plugin consumers use that preparation while preserving their geometry, z offsets, text/texture content, missing-component repair and entity lifecycle.
- [ ] Rebuild from current registry data on every enabled render update, including the first update after rendering is re-enabled; do not introduce persistent dirty-generation caching.

### Compatibility and scheduling

- [ ] Existing public standalone render systems retain their setup requirements and calculate fresh ordering for each invocation, even when a plugin preparation resource exists in the same world.
- [ ] Standalone and plugin-prepared entry points delegate to the same rendering bodies, without resource-presence fallback or duplicated rendering logic.
- [ ] Expose named plugin scheduling sets for preparation and the six affected renderer stages. Document that function-based ordering against the old public renderer functions does not target the new plugin variants.
- [ ] Ordering-affecting external registry updates must precede preparation to affect the current render pass. Preserve the existing input-last sequence and next-update hover visuals.
- [ ] Preserve processing/render/text enable flags: disabled rendering skips preparation and consumers; disabled text skips its consumers without disabling non-text rendering.

## How it works

- [Approved architecture and alternatives](../wiki/design/ui-frame-order.md)
- [Layout invalidation](ui-layout-invalidation.md) — preparation observes geometry after layout resolution.
- [Current UI implementation](../wiki/systems/ui-system.md)

## Implementation inventory

Current files. `plugin::UiRenderSet::{Prepare, Quads, Text, Shadows, Outlines, NineSlices, ThreeSlices}` exists at toolkit `752305f`, but no set is wired and the resource/prepared variants remain pending:

- `../ui-toolkit/src/plugin.rs` — current chained schedule and enable flags.
- `../ui-toolkit/src/render.rs` — ordering helper and quad rendering.
- `../ui-toolkit/src/render_text.rs`, `render_text_fx.rs` — main text, shadows and outlines.
- `../ui-toolkit/src/render_nine_slice.rs`, `render_three_slice.rs` — slice ordering and rendering.

## Tests asserting this spec

No tests yet assert shared preparation or the new scheduling contract. Existing characterization is input to future verification, not proof that this feature is implemented:

- `../ui-toolkit/src/render_tests.rs` — exact mixed-key ordering, effective-size membership and rendered z-order.
- `../ui-toolkit/tests/ui_plugin_integration.rs` — actual plugin ordering, settled ticks, resize and repair.
- `../ui-toolkit/tests/text_components.rs`, `shadow_components.rs`, `nine_slice_components.rs`, `remaining_sprites.rs`, `render_dirty_clear.rs` — standalone behavior and reconciliation boundaries.

## Known gaps (current cycle)

- [x] User authorized implementation; `UiRenderSet` enum declaration landed at toolkit `752305f`. Its scheduling behavior remains unverified.
- [ ] Add prepared path, shared bodies and scheduling sets without changing standalone setup.
- [ ] Verify actual-plugin and standalone outputs, enable/re-enable transitions, external ordering-field updates and scheduling boundaries.

## Out of scope

- Cross-frame generation caches or registry mutation redesign: rebuilding once per render update avoids those contracts.
- Changing membership, missing-ID behavior, input timing, renderer appearance or standalone setup: this is ordering-work consolidation only.
- Bevy UI migration, new native benchmarks and quantitative CPU claims: not part of this design approval.
