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
- [x] Keep waterfall texture animation moving after its first cycle by sampling each track with its own authored global-sequence duration.
- [x] Forward parsed emitters only for selected waterfall/ripple backdrops, including spawned skeleton joints and the graphics particle-effects gate.
- [x] Interpret particle flags as raw M2 file flags, not internal runtime-property flags.
- [x] Preserve authored packed texture stages, UV scales and signed per-particle UV motion for waterfall mist.
- [x] Bind every required particle texture and report missing or undecodable stages instead of rendering an incomplete material.
- [x] Parse particle-tail, twinkle, burst-multiplier, and drag fields at their authored offsets; `1028937` must retain burst `1.0` separately from drag `5.0`.
- [x] Render raw `XY_QUAD` waterfall particles in their world XZ plane while retaining authored spin in native and library effect-builder paths.
- [x] Preserve authored Y coordinates for selected waterfall/ripple backdrops; retain terrain-height grounding for ordinary props.

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
- `src/rendering/model/m2_spawn.rs` — forwards authored emitters and spawned M2 joints to `particle::spawn_emitters` when the terrain caller selects waterfall/ripple backdrops.
- `src/rendering/terrain/terrain_objects.rs` — limits emitter restoration to `is_waterfall_backdrop_doodad` rather than activating unrelated terrain-prop emitters, and preserves authored backdrop heights while grounding ordinary props.
- `src/asset/m2_format/m2_particle.rs` — parses particle-tail/twinkle/burst/drag fields at their authored tail offsets.
- `src/rendering/particles/effect_builder_shared.rs` — selects a world-XZ-plane modifier for raw `XY_QUAD` orientation while preserving spin.
- `src/rendering/particles/effect_builder.rs`, `src/particle_effect_builder.rs` — apply that orientation consistently in native and library paths.

## Tests asserting this spec

- `src/asset/adt_format/adt_tests/mcnk.rs`
- `src/rendering/terrain/terrain_spawn/tests.rs`
- `src/rendering/model/m2_spawn_material_tests.rs` — actual waterfall model sampled across multiple global periods.
- `src/scenes/char_select/scene/tests/supplemental_waterfall_tests.rs` — actual mist M2 terrain attachment emitter regression.
- `src/rendering/particles/xy_quad_gpu_tests.rs` — actual raw `2904370` controlled-white-quad XY top/side visibility against ordinary billboard controls. RED measured `[900, 900, 900, 900]`; GREEN `[900, 0, 900, 900]`.
- `src/scenes/char_select/scene/tests/supplemental_waterfall_tests.rs` — actual primary-ripple authored-height regression and ordinary-prop grounding control.

## Known gaps (current cycle)

- [ ] Native proof must show a readable, moving waterfall cascade plus authored mist/spray. Loading, UV, texture-track, emitter-wiring, parser, and XY-orientation regressions alone do not prove this.
- [ ] Do not infer or implement refraction material selection from the remaining appearance gap without direct Retail selection evidence; proven parser and geometry defects need no such assumption.

## Out of scope

- Waterfall-specific substitute geometry, ignored shadow flags, fog/camera changes, or unrelated water-format repairs.
