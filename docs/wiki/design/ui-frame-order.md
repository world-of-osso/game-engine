# Shared UI frame ordering

Approved design, implementation in progress. On 2026-09-10 the user selected **“Preserve standalone setup”**, then **“Accept and document design”** for named plugin scheduling sets. The user later requested implementation; toolkit `752305f` declares the public `UiRenderSet` API, while preparation and consumers remain pending. The [feature contract](../../specs/ui-frame-order.md) records unverified requirements; [UI systems](../systems/ui-system.md) describes current behavior.

## Current repeated work

At toolkit `1050abb`, with processing, rendering and text enabled, six systems independently build the same sorted ID list: quads, main text, shadows, outlines, nine-slices and three-slices. Four build index maps; two build z maps. All use visible positive-size membership and the same strata/frame-level/raise-order/ID comparator. Layout runs before these consumers; existing internal render systems do not change ordering inputs.

This is source evidence of repeated work, not measured CPU cost. The local unstable-sort correction remains useful; it does not consolidate these calls.

## Approved structure

Introduce one private prepared-order resource with an ordered `Vec<u64>` and `HashMap<u64, usize>`. Preparation runs after layout and button nine-slice derivation, inside the enabled render pipeline. It rebuilds on every enabled render update. Resource storage may persist, but its contents are never reused across enabled render updates without rebuilding; no mutation-generation or cross-frame invalidation system is needed.

Each affected renderer has two entry points delegating to one rendering body:

| Entry point | Ordering source | Intended caller |
| --- | --- | --- |
| Existing public system | Fresh locally calculated order on every invocation | Standalone registration, including existing tests |
| New private prepared system | Required prepared-order resource | `UiPlugin` |

Public systems do not detect the presence of preparation resources or switch modes. They remain fresh even in a world that also contains the plugin resource. Prepared variants require the plugin's explicit setup; there is no silent fallback.

The common bodies retain discovery, texture resolution, component comparisons, missing-component repair and stale-entity removal. Slice consumers derive z from the shared index using their existing arithmetic rather than building separate maps. Existing missing-ID behavior must not be normalized as an incidental refactor.

Expected structural cost: six thin private system wrappers, extraction of six common rendering bodies, one preparation system/resource, and changed plugin registration. Public signatures/setup stay unchanged; renderer logic must not be duplicated.

## Scheduling contract

Toolkit `752305f` declares public `plugin::UiRenderSet::{Prepare, Quads, Text, Shadows, Outlines, NineSlices, ThreeSlices}`. They are not yet assigned to systems; their behavioral contract remains unverified.

The plugin sequence remains:

```text
window synchronization → layout → button slice derivation
→ order preparation → existing renderer sequence → button input
```

Preparation and prepared consumers share the rendering gate. Processing-disabled updates skip the pipeline. Text-disabled updates skip main text, shadows and outlines, while non-text rendering still uses current preparation. Re-enabling rendering rebuilds order before any consumer runs.

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

## Acceptance strategy

The spec remains unchecked until implementation and evidence exist. Behavioral tests must cover:

- Actual-plugin rendered order and membership across insertion/removal, layout/resize, visibility, strata, level and raise changes.
- Existing standalone calls without preparation, and calls after registry changes in a world containing prepared data.
- Processing/render/text toggles and the first re-enabled update, with original input-last hover timing retained.
- Named-set scheduling before preparation and after rendering, plus existing visual/component repair and lifecycle cases.

Use source-path audit to establish one preparation and no renderer-local recomputation in the plugin path. Do not substitute source-substring assertions or internal helper-call counts for behavioral tests. Native benchmarks and CPU-gain claims are not part of this approval.

## Sources

- [Feature contract](../../specs/ui-frame-order.md) — approved requirements and pending implementation.
- [Plugin schedule](../../../../ui-toolkit/src/plugin.rs) — current ordering and enable gates.
- [Frame-order helper and quad renderer](../../../../ui-toolkit/src/render.rs) — current membership/comparator and local map.
- [Main text](../../../../ui-toolkit/src/render_text.rs), [shadow/outline text](../../../../ui-toolkit/src/render_text_fx.rs) — text consumers.
- [Nine-slices](../../../../ui-toolkit/src/render_nine_slice.rs), [three-slices](../../../../ui-toolkit/src/render_three_slice.rs) — z-map consumers.

## See Also

- [[ui-system]] — implemented rendering and standalone behavior.
- [[movement-performance]] — measured evidence distinguished from source-level corrections.
