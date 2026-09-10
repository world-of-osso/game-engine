# Shared UI frame ordering

Implemented at toolkit `02a3049`, independently verified with tests through `3e61227`. The user required unchanged standalone setup, accepted named plugin scheduling sets, then authorized implementation before discussing Bevy UI migration. The [feature contract](../../specs/ui-frame-order.md) distinguishes behavioral test coverage from structural source-audit proof; [UI systems](../systems/ui-system.md) describes related rendering behavior.

## Previous repeated work

At toolkit `1050abb`, with processing, rendering and text enabled, six systems independently build the same sorted ID list: quads, main text, shadows, outlines, nine-slices and three-slices. Four build index maps; two build z maps. All use visible positive-size membership and the same strata/frame-level/raise-order/ID comparator. Layout runs before these consumers; existing internal render systems do not change ordering inputs.

This was source evidence of repeated work, not measured CPU cost. Shared plugin preparation now replaces those six independent builds; standalone calls intentionally remain independent. The local unstable-sort correction is retained.

## Implemented structure

Private `render::UiFrameOrder` holds `ids: Vec<u64>` and `indices: HashMap<u64, usize>`. `prepare_ui_frame_order` builds both using the existing sorted-ID helper. Preparation runs after layout and button nine-slice derivation, inside the enabled render pipeline. It rebuilds on every enabled render update. Resource storage may persist, but its contents are never reused across enabled render updates without rebuilding; no mutation-generation or cross-frame invalidation system is needed.

Each affected renderer has two entry points delegating to one rendering body:

| Entry point | Ordering source | Intended caller |
| --- | --- | --- |
| Existing public system | Fresh locally calculated order on every invocation | Standalone registration, including existing tests |
| New private prepared system | Required prepared-order resource | `UiPlugin` |

Public systems do not detect the presence of preparation resources or switch modes. They remain fresh even in a world that also contains the plugin resource. Prepared variants require the plugin's explicit setup; there is no silent fallback.

The common bodies retain discovery, texture resolution, component comparisons, missing-component repair and stale-entity removal. Slice consumers derive z from the shared index using their existing arithmetic rather than building separate maps. Existing missing-ID behavior must not be normalized as an incidental refactor.

Six prepared system wrappers and six public standalone entry points delegate to common renderer bodies. `UiPlugin` registers the prepared variants; public signatures/setup remain unchanged. Explicit resource/query adapter plumbing is repeated, not rendering logic.

## Scheduling contract

Public `plugin::UiRenderSet::{Prepare, Quads, Text, Shadows, Outlines, NineSlices, ThreeSlices}` is wired into `UiPlugin`. `Prepare` includes window synchronization, layout, button derivation and the final order producer. Thus `.before(UiRenderSet::Prepare)` runs before layout as well as sorting; same-update geometry updates are tested. Renderer-specific sets target the prepared variants, for example `.after(UiRenderSet::Text)`.

The plugin sequence remains:

```text
window synchronization → layout → button slice derivation
→ order preparation → existing renderer sequence → button input
```

The order producer and prepared consumers share the rendering gate. Earlier members of `Prepare` retain their processing-only gates: disabling rendering does not disable window/layout/button synchronization. Processing-disabled updates skip the pipeline. Text-disabled updates skip main text, shadows and outlines, while non-text rendering still uses current preparation. Re-enabling rendering rebuilds order before any consumer runs.

External systems changing visibility, effective dimensions, membership, strata, frame level or raise order must run before preparation to affect that pass. Ordering-affecting writes between preparation and its consumers are outside this contract. Mutations after the render phase are observed on the next preparation. Sets provide ordering points, not automatic invalidation or an atomic render transaction.

### Compatibility boundary

Standalone registration remains unchanged. Plugin scheduling by original function identity does not: `.before(sync_ui_text)` and similar constraints will not target its new private prepared variant. Integrations must target the corresponding named plugin set instead. This migration is accepted for the design.

Checked engine production registrations use `UiPlugin`; direct renderer registrations were found in toolkit tests. No function-relative plugin ordering callers were found in the inspected repositories. Unknown external users may exist; do not describe the change as universally transparent.

Bevy disallows assigning arbitrary systems to another function's implicit system-type set. Custom `System` plumbing to impersonate identities is rejected as disproportionate complexity.

## Alternatives rejected

- **Mandatory preparation for every caller:** changes standalone setup, contrary to the user's decision.
- **Optional-resource mode switching:** presence does not prove freshness for standalone calls; introduces implicit behavior.
- **Registry generation cache:** public frame fields and mutation access require a broader invalidation contract than once-per-render preparation needs.
- **One giant rendering system:** combines all queries/assets/locals and requires more invasive orchestration than thin wrappers.

## Verification

The [independent report](../../../data/diagnostics/unchanged-writes-20260910/shared-frame-order/verification/report.md) records 39 passing tests across seven focused targets within a cumulative 10-second execution budget, toolkit formatting/check/readability, and bounded engine compilation integration. Six shared-order cases cover the following boundaries alongside existing renderer and actual-plugin tests:

- Actual-plugin rendered order and membership across insertion/removal, layout/resize, visibility, strata, level and raise changes.
- Existing standalone calls without preparation, and calls after registry changes in a world containing prepared data.
- Processing/render/text toggles and the first re-enabled update, with original input-last hover timing retained.
- Named-set scheduling before preparation and after rendering, plus existing visual/component repair and lifecycle cases.

One preparation/map and common-body delegation are established by source-path audit, not internal helper-call or source-substring tests. These structural requirements are identified separately in the spec rather than labeled automated assertions. Existing outline entities still have creation-only updates; changed-order tests exercise newly created outlines, not a new retained-outline reconciliation feature. Engine evidence records 11 passing tests, one ignored GPU case and existing diagnostics; supplied build-provenance limits are retained. No native benchmark, whole-engine-suite or CPU-gain claim.

## Sources

- [Feature contract](../../specs/ui-frame-order.md) — implemented requirements and proof distinctions.
- [Plugin schedule](../../../../ui-toolkit/src/plugin.rs) — current ordering and enable gates.
- [Frame-order helper and quad renderer](../../../../ui-toolkit/src/render.rs) — current membership/comparator and local map.
- [Main text](../../../../ui-toolkit/src/render_text.rs), [shadow/outline text](../../../../ui-toolkit/src/render_text_fx.rs) — text consumers.
- [Nine-slices](../../../../ui-toolkit/src/render_nine_slice.rs), [three-slices](../../../../ui-toolkit/src/render_three_slice.rs) — shared-index consumers.

## See Also

- [[ui-system]] — implemented rendering and standalone behavior.
- [[movement-performance]] — measured evidence distinguished from source-level corrections.
