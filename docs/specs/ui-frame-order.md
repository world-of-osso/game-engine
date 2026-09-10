# Shared UI frame ordering

Implemented at toolkit `02a3049`, verified with tests through `3e61227`. `UiPlugin` shares prepared frame ordering across six renderers while existing standalone renderer setup remains unchanged. Architecture and compatibility rationale: [shared ordering design](../wiki/design/ui-frame-order.md).

## What it must do

### Ordering and rendering

- [ ] Prepare one ordered ID list and one ID-to-index map per enabled plugin render update, after layout and button preparation. Implemented and source-audited; internal work counts are not asserted by behavioral tests.
- [x] Preserve current membership: visible frames with positive effective width and height. Preserve ordering by strata, frame level, raise order, then frame ID.
- [x] All six plugin consumers use that preparation while preserving their geometry, z offsets, text/texture content, missing-component repair and entity lifecycle.
- [x] Rebuild from current registry data on every enabled render update, including the first update after rendering is re-enabled; do not introduce persistent dirty-generation caching.

### Compatibility and scheduling

- [x] Existing public standalone render systems retain their setup requirements and calculate fresh ordering for each invocation, even when a plugin preparation resource exists in the same world.
- [ ] Standalone and plugin-prepared entry points delegate to the same rendering bodies, without resource-presence fallback or duplicated rendering logic. Implemented and source-audited; helper structure is not asserted by behavioral tests.
- [x] Expose named plugin scheduling sets for preparation and the six affected renderer stages. Document that function-based ordering against the old public renderer functions does not target the new plugin variants.
- [x] Ordering-affecting external registry updates must precede preparation to affect the current render pass. Preserve the existing input-last sequence and next-update hover visuals.
- [x] Preserve processing/render/text enable flags: disabled rendering skips preparation and consumers; disabled text skips its consumers without disabling non-text rendering.

## How it works

- [Approved architecture and alternatives](../wiki/design/ui-frame-order.md)
- [Layout invalidation](ui-layout-invalidation.md) — preparation observes geometry after layout resolution.
- [Current UI implementation](../wiki/systems/ui-system.md)

## Implementation inventory

- `../ui-toolkit/src/plugin.rs` — initializes ordering, wires prepared variants and `UiRenderSet::{Prepare, Quads, Text, Shadows, Outlines, NineSlices, ThreeSlices}` with existing gates.
- `../ui-toolkit/src/render.rs` — private `UiFrameOrder`, producer, standalone/prepared quad adapters and common rendering body.
- `../ui-toolkit/src/render_text.rs`, `render_text_fx.rs` — standalone/prepared text adapters and common main/shadow/outline bodies.
- `../ui-toolkit/src/render_nine_slice.rs`, `render_three_slice.rs` — standalone/prepared slice adapters using shared indices for z.

## Tests asserting this spec

The [independent report](../../data/diagnostics/unchanged-writes-20260910/shared-frame-order/verification/report.md) records 39 passing tests across seven focused targets, toolkit formatting/check/readability, and shared engine compilation evidence. Computational work count and common-body structure are explicitly source-audited, not test-instrumented; their checklist entries remain unchecked rather than claiming automated structural assertions.

- `../ui-toolkit/tests/shared_frame_order.rs` — six actual-plugin/standalone cases: ordering/membership changes, flags/re-enable, stale-resource independence, named-set publication and same-update geometry before preparation.
- `../ui-toolkit/tests/ui_plugin_integration.rs` — actual plugin ordering, settled ticks, resize and repair.
- `../ui-toolkit/tests/text_components.rs`, `shadow_components.rs`, `nine_slice_components.rs`, `remaining_sprites.rs`, `render_dirty_clear.rs` — standalone behavior and reconciliation boundaries.

## Known gaps (current cycle)

No remaining implementation gate under the approved verification strategy. The two structural requirements above have source-audit proof, not internal-call/shape tests. Existing outlines retain creation-only updates; this work does not add retained-outline reconciliation. Engine proof is bounded to the recorded compilation/tests, including one ignored GPU case and existing diagnostics; no native or whole-suite proof is claimed.

## Out of scope

- Cross-frame generation caches or registry mutation redesign: rebuilding once per render update avoids those contracts.
- Changing membership, missing-ID behavior, input timing, renderer appearance or standalone setup: this is ordering-work consolidation only.
- Bevy UI migration is the user's next topic after this implementation; new native benchmarks and quantitative CPU claims remain outside this task.
