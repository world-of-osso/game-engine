# Render-set isolation

`--remove-render-set-after NAME SECONDS` removes a whole render-system set after startup for frozen-scene CPU attribution. Supported groups are Prepare, PrepareAssets, Specialize, and Queue. This is a destructive diagnostic, not an equivalent-work optimization.

## What it must do

- [ ] Remove the chosen set once at the deadline and report removed-system count.
- [ ] Preserve render execution, command application, cleanup, normal pools, and other sets.
- [ ] Keep an independent main Last update counter.
- [ ] Fail explicitly on invalid arguments or an empty removal.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).

## Implementation inventory

- `src/render_set_isolation.rs`: timed removal and update reporting.
- `src/main.rs`: opt-in configuration.

## Tests asserting this spec

- Module behavioral tests cover selected-set removal and outside-set execution.

## Known gaps

- [ ] Native cutoff result and visual validity.

## Out of scope

Deployment, preserving dynamic scene behavior after cutoff, CPU/FPS caps, worker changes, profiler testing.
