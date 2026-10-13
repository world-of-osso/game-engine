# Native M2 ribbons

Spell-kit M2 models must render their authored trailing ribbons. Source: authenticated Retail M2 records and local WebWowViewerCpp `M2FileHeader.h`, `animationManager.cpp::calcRibbonEmitters`, and `CRibbonEmitter`. [Format and current proof](../wiki/formats/m2-format.md#ribbon-emitter-scalars-2026-10-13).

## What it must do

### Source inputs
- [ ] Preserve bone-relative position, texture/material references, colour/opacity, width-above/below, texture slot and visibility tracks, density, lifetime and source gravity displacement semantics. Invalid records fail explicitly.
- [ ] Sample animation/global-sequence tracks and source-defined unauthored defaults: white/opaque, height1, enabled1, slot0. Missing source assets are errors, not substituted textures or models.

### Presentation and lifecycle
- [ ] Emit world-space trailing strips at authored edge density; retain each edge only for its authored lifetime while following the model's animated bone and object scale.
- [ ] Preserve texture tiles, colour/alpha and material blending/culling/depth/fog/light flags.
- [ ] Begin with the ordinary kit; end emission on kit end and remove remaining geometry after its natural tail. No orphan mesh after owner/kit removal.
- [x] Prismatic Bauble1417024 produces a visible rainbow behind actual player movement in both skins. Loaded model metadata is not rendered acceptance; no body/health replacement or cooldown reset.

## How it works
- [M2 format](../wiki/formats/m2-format.md#ribbon-emitter-scalars-2026-10-13)
- [Toy source/rendering boundary](../wiki/systems/toy-box.md#prismatic-bauble-rendering-boundary-2026-10-13)

## Implementation inventory
- `godot/core/src/asset/m2_format/m2_ribbon.rs` — source records/references/tracks.
- `godot/core/src/m2.rs` — parsed model ribbon inputs.
- `godot/core/src/m2_ribbons.rs` — source track sampling, retained world edges and triangle streams.
- `godot/rust/src/ribbons.rs` — animated emitter pose and native strip draw.
- `godot/rust/src/spell_effects/models.rs` and `godot/shaders/m2_ribbon.gdshader` — ordinary kit lifecycle and source material state.

## Tests asserting this spec
- `godot/core/tests/m2_ribbons.rs` — authenticated1417024 scalar/reference/track values.
- `godot/tests/toy_effects26_rainbow_live.gd` — physical movement and rendered trailing rainbow pixels.
- `godot/tests/toy_effects26_bauble_live.gd` — native aura/kit/recovery observations and natural removal.

## Known gaps (current cycle)
- [ ] Final natural-removal/cleanup and current-revision integration gate.
- [ ] Animated ribbon UV transforms and nonmatching texture/material reference counts remain explicit unsupported shapes; neither occurs in authenticated1417024.

## Out of scope

No additional exclusions are established by this spec; unrelated renderer redesign is not required to close this source-defined trail.
