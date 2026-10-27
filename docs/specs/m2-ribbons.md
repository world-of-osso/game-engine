# Native M2 ribbons

Spell-kit M2 models must render their authored trailing ribbons. Source: authenticated Retail M2 records and local WebWowViewerCpp `M2FileHeader.h`, `animationManager.cpp::calcRibbonEmitters`, and `CRibbonEmitter`. [Format and current proof](../wiki/formats/m2-format.md#ribbon-emitter-scalars-2026-10-13).

## What it must do

### Source inputs
- [ ] Preserve bone-relative position, texture/material references, colour/opacity, width-above/below, texture slot and visibility tracks, density, lifetime and source gravity displacement semantics. Invalid records fail explicitly.
- [x] Sample animation/global-sequence tracks and source-defined unauthored defaults: white/opaque, height1, enabled1, slot0. Missing source assets are errors, not substituted textures or models.
- [x] When model flag0x20000 enables ribbon UV transforms, each material pass resolves emitter lookup base+pass. A negative lookup selects identity; valid transforms follow local/global playback and transform sampled ribbon UVs.

### Presentation and lifecycle
- [x] Emit world-space trailing strips at source-normalized density (ceil) and lifetime (minimum0.25s), following animated bone and object scale. The first update warms one interval plus0.0001s; later long-frame delta clamps to lifetime while interpolating the complete movement segment.
- [x] Current animated texture slot applies to every surviving edge. Tile axes follow M2Object's Initialize(textureCols, textureRows) call: authored rows2/cols1/slot1 selects the horizontal half, not the vertical half. Preserve colour/alpha and material culling/depth/fog/light flags; source opaque ribbons alpha-blend, matching CRibbonEmitter.
- [x] Apply authored priority plane to native material draw ordering. Priorities outside Godot's-128..127 range fail explicitly; no clamping or substituted order.
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
- `godot/core/tests/m2_ribbons.rs` — authenticated1417024 scalar/reference/track values; real1865728 enabled per-pass UV lookup/global translation; concrete fractional rate, minimum lifetime, long-frame movement and rows2/cols1 current-tile regressions.
- `godot/tests/m2_ribbon_material_pixels.gd` — production shader/mesh raster: source blend0 with0.25 alpha, farther high-priority green over nearer low-priority red, explicit out-of-range priority refusal. `RIBBON_MATERIAL_OUTPUT` selects the required output directory; the fixture saves `native-ribbon-alpha.png`, `native-ribbon-priority.png`, and sanitized `material-raster-receipts.json` with pixels, inputs, dimensions and `RIBBON_MATERIAL_REVISION`. The same production update path samples real1865728 global-sequence translation at0/633ms and moves a synthetic red/green test texture from green to red; this proves texture-matrix application, not that fixture model's art.
- `godot/tests/toy_effects26_rainbow_live.gd` — physical movement and rendered trailing rainbow pixels.
- `godot/tests/toy_effects26_bauble_live.gd` — native aura/kit/recovery observations and natural removal.

## Known gaps (current cycle)
- [ ] Final natural-removal/cleanup and current-revision integration gate.
- [ ] Material passes use the corresponding texture references; additional texture references do not create passes. Missing textures for a material pass remain an error.

## Out of scope

No additional exclusions are established by this spec; unrelated renderer redesign is not required to close this source-defined trail.
