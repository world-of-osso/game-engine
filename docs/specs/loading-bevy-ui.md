# Native Bevy loading screen

## Requirements

- Render the loading screen using native Bevy UI authored with `rsx!`; do not retain a second toolkit loading renderer or duplicate readiness state.
- Preserve the fixed cathedral artwork, parchment bands, logo, black matte, three-piece steel shell, alchemical fill, text styling, layout and semantic names.
- Derive status and target progress from existing `evaluate_world_loading`; preserve its local-player/terrain boundaries and existing transitions to InWorld.
- Preserve displayed progress at six percentage points per second, integer rounding, target reduction, and preview mode advancing to 100% without entering the world automatically.
- Preserve zone names/default text, tip, status and percentage text, debug-source layout overrides, and same-entity updates when progress or layout changes.
- Resize the fullscreen root with the window. Keep loading above world rendering and legacy overlays above loading; restore prior camera orders and remove only loading entities/resources on exit.
- Expose native loading names through existing semantic waits and UI-tree dumps. Loading has no user controls; do not add dropdowns or input actions.

## Compatibility details

The existing steel-shell style uses fixed 25-pixel caps; `bar_cap_width` source overrides do not change it. The baseline draws status text below the shell/fill, so it can be occluded while its semantic text still updates. This migration preserves both behaviors rather than redesigning them.

## Implementation

At engine `7a4b8eb3`, `5f978988`, and `a5f17dfa`, loading uses a native Bevy view authored with `rsx! { @native(...) }`. It replaces the loading `Screen`, `SharedContext`, `FrameRegistry`, registry-size synchronization, and legacy renderer; readiness and progress remain in the existing loading lifecycle resources. The native view retains the legacy semantic names and camera ordering model. No loading controls or dropdowns were added.

## Evidence

- Before migration: eight loading tests passed using the existing compiler-emitted executable; no rebuild. Baseline screenshot/tree and exact provenance are retained locally under `data/diagnostics/loading-bevy-ui/baseline/`.
- Native implementation and updated preservation tests require verification. Compare computed layout and a rendered preview against the baseline; CPU tests alone do not establish visual equivalence.

## Scope

Only the loading presentation/lifecycle is replaced. Login/caret, readiness producers, network/world-loading transitions, other screens and dropdown features are unchanged. Keep development worktrees; no merge, push, cleanup or deployment is requested.
