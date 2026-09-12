# Split ADT Shadows

Terrain loading combines root geometry with shadow payloads from its `_tex0.adt` companion before validating shadow declarations. See [waterfall loading investigation](../wiki/investigations/character-select-waterfall-loading.md).

## What it must do

- [ ] Load valid split tiles whose root declares shadows stored in the texture companion.
- [ ] Preserve shadow bytes and existing per-chunk edge-fix flags.
- [ ] Keep standalone/monolithic shadow validation strict when no companion supplies required data.
- [ ] Reject mismatched root/companion chunk counts and conflicting shadow payloads explicitly.
- [ ] Load Adventurer's Rest's supplemental waterfall tile and its waterfall/ripple placements.
- [ ] Report failed terrain loads with the affected path and error rather than silently skipping scenery.

## How it works

- [Terrain](../wiki/systems/terrain.md)
- [Waterfall investigation](../wiki/investigations/character-select-waterfall-loading.md)

## Implementation inventory

- `src/asset/adt_format/adt.rs` — aligns root and companion chunks.
- `src/asset/adt_format/adt/parsing.rs` — merges shadow inputs before validation.
- `src/asset/adt.rs` — mesh-producing split-input entry point.
- `src/rendering/terrain/terrain_spawn.rs` — resolves and reads companion data.
- `src/scenes/char_select/scene_tree.rs` — reports backdrop load failures.

## Tests asserting this spec

- `src/asset/adt_format/adt_tests/mcnk.rs`
- `src/rendering/terrain/terrain_spawn/tests.rs`

## Known gaps (current cycle)

- [ ] Complete corrected regression, focused checks, and native waterfall verification.

## Out of scope

- Waterfall-specific substitute geometry, ignored shadow flags, fog/camera changes, or unrelated water-format repairs.
