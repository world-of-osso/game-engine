# Split ADT Shadows

Terrain loading combines root geometry with shadow payloads from its `_tex0.adt` companion before validating shadow declarations. See [waterfall loading investigation](../wiki/investigations/character-select-waterfall-loading.md).

## What it must do

- [x] Load valid split tiles whose root declares shadows stored in the texture companion.
- [x] Preserve shadow bytes and existing per-chunk edge-fix flags.
- [x] Keep standalone/monolithic shadow validation strict when no companion supplies required data.
- [x] Reject mismatched root/companion chunk counts and conflicting shadow payloads explicitly.
- [x] Load Adventurer's Rest's supplemental waterfall tile and its waterfall/ripple placements.
- [x] Include the 14 authored waterfall/ripple backdrops on the primary tile beyond the nearby-prop radius; keep ordinary props radius-limited.
- [x] Report failed terrain loads with the affected path and error rather than silently skipping scenery.
- [x] Use the shader-encoded second UV set for modern waterfall modulation textures when the legacy coordinate lookup is absent; preserve authored alpha combination.
- [ ] Keep waterfall texture animation moving after its first cycle by sampling each track with its own authored global-sequence duration.

## How it works

- [Terrain](../wiki/systems/terrain.md)
- [Waterfall investigation](../wiki/investigations/character-select-waterfall-loading.md)

## Implementation inventory

- `src/asset/adt_format/adt.rs` — aligns root and companion chunks.
- `src/asset/adt_format/adt/parsing.rs` — merges shadow inputs before validation.
- `src/asset/adt.rs` — mesh-producing split-input entry point.
- `src/rendering/terrain/terrain_spawn.rs` — resolves and reads companion data.
- `src/scenes/char_select/scene_tree.rs` — reports backdrop load failures.
- `src/asset/m2_batch.rs` — resolves waterfall shader UV inputs.
- `src/rendering/model/m2_effect_material.rs` — samples animated texture offsets using model global-sequence periods.
- `src/rendering/model/m2_spawn_material.rs`, `m2_spawn.rs`, `m2_scene/mod.rs` — retain model timing data when creating ordinary effect materials.

## Tests asserting this spec

- `src/asset/adt_format/adt_tests/mcnk.rs`
- `src/rendering/terrain/terrain_spawn/tests.rs`
- `src/rendering/model/m2_spawn_material_tests.rs` — actual waterfall model sampled across multiple global periods.

## Known gaps (current cycle)

- [ ] Final production native visibility audit is running. Completed loading, UV, and focused RED/GREEN regressions do not alone prove a readable waterfall cascade.

## Out of scope

- Waterfall-specific substitute geometry, ignored shadow flags, fog/camera changes, or unrelated water-format repairs.
