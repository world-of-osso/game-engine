# Frozen core combat-data build

`game-engine-core` exposes its existing AssistedCombat evaluator and spell catalog to headless consumers without loading client asset modules.

## What it must do

- [ ] `default-features = false` exposes `assisted_combat`, `cooldown_assist`, `cooldown_conditions` and `spell_catalog` without asset/image dependencies.
- [ ] Default builds expose the existing complete client API.
- [ ] Evaluator, catalog and CSV/cache implementations remain unchanged from frozen revision `9ef2efe2d317b837d352fe8c5227086a4fdcd490`.

## How it works

- The default-enabled `client` feature selects client modules and their optional dependencies.

## Implementation inventory

- `godot/core/Cargo.toml`: optional client dependencies.
- `godot/core/src/lib.rs`: conditional client-module declarations.

## Tests asserting this spec

- Existing evaluator and catalog tests remain attached to their unchanged source modules.
- A data-only consumer check asserts the dependency boundary.

## Known gaps (current cycle)

- [ ] Check the data-only consumer and run existing evaluator tests.

## Out of scope

- Updating the frozen evaluator or changing any client behavior.
