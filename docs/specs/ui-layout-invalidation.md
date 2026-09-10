# UI layout invalidation

The shared `ui-toolkit` resolves frame geometry only when layout inputs are dirty. See [UI system](../wiki/systems/ui-system.md) for implementation context.

## What it must do

- [x] An empty layout-dirty set leaves settled rectangles and render dirtiness unchanged.
- [x] Initial insertion, removal, screen resize, and anchor changes invalidate affected layouts and dependent frames.
- [x] Dimension and flex attribute changes trigger layout updates even when unrelated frames are dirty.
- [x] Flex child changes reflow the parent and anchored dependents.
- [x] Text/editbox auto-sizing and resolved named anchors invalidate affected geometry; explicit unanchored rectangles retain existing semantics.
- [x] Addon size changes use the same invalidation contract while preserving frame ownership checks.

## How it works

- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `../ui-toolkit/src/layout.rs` — dirty-only resolution and flex settling.
- `../ui-toolkit/src/registry.rs` — explicit layout invalidation and dependency tracking.
- `../ui-toolkit/src/attrs.rs` — dimension/flex attribute invalidation.
- `../ui-toolkit/src/anchor_resolve.rs` — named-anchor registration.
- `../ui-toolkit/src/screen.rs` — auto-sizing invalidation.
- `src/ui/addon_runtime/apply.rs` — addon geometry updates.

## Tests asserting this spec

- `../ui-toolkit/src/registry.rs` — clean, resize, insertion/removal, anchor and flex behavior.
- `../ui-toolkit/src/attrs_layout_tests.rs` — dimension and flex updates.
- Screen/anchor integration fixtures recorded in `data/diagnostics/ui-layout-dirty-20260909/screen-anchors/report.md`.
- `src/ui/addon_runtime/tests.rs` — addon size/ownership integration (7 GREEN in `data/diagnostics/ui-layout-dirty-20260909/engine-addon/green.log`).
- Toolkit core: 14 focused regressions plus 41 existing layout/screen/diff/plugin tests = 55 passing toolkit tests.

## Known gaps (current cycle)

No remaining integration gate for this correction. The [final artifact audit](../../data/diagnostics/ui-layout-dirty-20260909/verification/final-integration-report.md) records engine compilation at `9d22c7fb` with toolkit `0fdcf3f`, retaining supplied-command provenance limits.

## Out of scope

- Bevy UI migration, layout algorithm redesign, or automatic invalidation for arbitrary raw frame mutations.
- Full-client CPU measurements or quantitative performance claims.
