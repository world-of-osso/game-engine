# Liquid rendering

WMO MLIQ and terrain MH2O liquids use LiquidType-backed material data. The [liquid investigation](../wiki/investigations/northshire-pale-water.md) records the renderer and source evidence.

## What it must do

- [x] Resolve the Neltharus Forge WMO group's authored LiquidType and its material and texture data, rather than selecting a shader from its liquid name.
- [x] Resolve magma and slime LiquidType data through their LiquidMaterial class and retain authored MH2O vertex data.
- [x] Magma and ProcMagma colour chains (BLS, all four permutations): neither family binds a shore. Magma at detail 0 is `t0(TC1.xy).rgb` with alpha 1 and no noise layer; ProcMagma adds the noise-weighted three-layer chain. The noise RGB supplies successive lerp weights, not UV offsets.
- [ ] ProcMagma's interpolated UV planes (TC1.zw, TC2, TC3), the per-draw cb1[12].w noise slice and the cb1[12].xyz baseline colour. The engine's liquid vertex format supplies one UV plane, so those inputs are not available yet.
- [x] Retry unknown liquid type IDs, including zero without a geometry-derived type, through same-product LiquidType8 and its material/textures; never select Material8 directly.
- [x] Implement Forever Material130's authored PBR path (six LiquidTypeXTexture foam masks, EF2 38-float batch, row Color fields) behind the explicit `GAME_ENGINE_PBR_WATER=1` opt-in. Default remains the LiquidType5 fallback until the procedural depth texture and prepass exist; the uber's unit/cb hops stay labelled adaptations.
- [ ] Render Material 2/4 with Retail ProcMagma behavior at the applicable detail level using sourced inputs and equations.
- [x] Keep Forge on Material13/Fel: two base layers, detail>=1 emissive/procedural, detail>=2 shore and detail>=3 specular. G4 UV triples are (scale,phaseDegrees,velocity); phase adds LiquidObject.FlowDirection and velocity multiplies FlowSpeed. Missing/zero-flow records remain static, with no guessed WMO flow.
- [x] G8/Fel shore: D = saturate((linear scene depth - liquid surface depth) / Float_11), sampled as t5(TC1.z, 1-D) with `shore.rgb + (base - shore.rgb)*(D*shore.a)`. `cb1[12]` = {Float_13, 1/Float_11, Float_14, Float_15} and `cb1[13]` = {pair blend, Float_16, Float_17, 0} are bound from the row. The material stays opaque; the t16 screen composite is out of scope.
- [x] Use integer-millisecond wrap with G4's finite velocity domain (1e-5<abs(v)<1000), negative cycles ending at1. Scale does not rotate the source UV.
- [ ] ProcFel permutation/technique selection and its 18 techniques; the engine renders both `Fel` and `ProcFel` detail tiers from one detail-gated shader. The BLS samples t6 through the draw item's UV rectangle, so the exact CPU texture bound there is still open.
- [x] G7 terrain contract: an `MH2O` layer's id resolves verbatim with no 1..20 rewrite and no type-8 retry, and the draw skips a row whose material column is Fog (10). The rewrite and the retry remain WMO-only.
- [x] Shore depth source: `D = saturate((view-space Z of the opaque surface behind the pixel - the liquid fragment's view-space Z) / Float_11)`, sampled at the pixel's screen position. This is water thickness along the view ray, active at liquid detail >= 2; the same prepass target also serves water, ley line, fog, azerite and M2 depth-based opacity.
- [x] The water family reads the same screen-position view-depth sample and the same Float_11 fade width, so `alpha = saturate(thickness / Float_11)`, and the foam fade is its complement. The previous fresnel-widened `smoothstep` over an invented 1.2 constant is gone; the view-depth gap already grows with the view angle. `water_material_pixels.gd` proves the fade is confined to Float_11 (Northshire LiquidType 5: 0.2 yd).
- [x] A per-surface bottom-height bake is **refuted** as the shore source: Neltharus bakes 37-110 yd and saturates everywhere. No render path referenced `game_engine_core::liquid_shore::ShoreDepthMap` and no lane referenced its contract, so the module and its five tests are deleted (2026-11-05). The shore has one source: the view-depth prepass.
- [x] MH2O depth bytes are handed through verbatim (no unit conversion in the parser or material), matching G7's 9-units-per-yard asset measurement. End-to-end check: rebuilding the emitted mesh and sampling the ADT terrain at every vertex keeps every nonzero depth byte within one quantum (0.111 yd) of `surface − terrain` on 847 samples, and every byte-0 shore vertex within one quantum above ground on 37 samples (`emitted_depth_bytes_match_terrain_at_nine_units_per_yard`). The draw-side input stays `byte/255`; the 1/9 yard is asset-side only.
- [x] MH2O coverage is binary and selects triangles only (I2 B.5). A layer without a mask offset is fully present over its rectangle; masking tiles away never changes a surviving vertex's position, UV or depth channel, and no coverage value reaches an attribute. Tests: `a_layer_without_a_mask_marks_every_rectangle_tile_present`, `coverage_selects_tiles_and_leaves_authored_vertex_values_untouched`.
- [x] G13 depth-curve stage (2026-11-05): the cubic is applied **once, on the CPU at the draw-vertex write** — `d = byte/255`, `c0 + c1·d + c2·d² + c3·d³`, upper-clamped to 1, stored in `UV2.y`. No liquid pixel permutation contains the polynomial; the water shader now consumes the interpolated curved value through its affine mix only. Inputs: `LiquidType.Int_0` (selector) and `Coefficient_0..3`; `Int_0 = 0` uses the hard-coded `(0, 255/42, 0, 0)` (saturates at byte 42 = 4.67 yd), `Int_0 = 1` is linear, `Int_0 >= 2` uses the row coefficients for `LVF` 0/3, and the level-1 magma format uses `(0, 1, 0, 1)`. Applied in `LiquidMaterial::curved_depth` and written by `build_mesh`; the Fel, LeyLine and Swamp pixel equations keep their own row-coefficient cubic because they were derived from their own BLS payloads (their shipped rows are linear-curve except the solid-lava coefficient rows).
- [x] G13 flat-layer placement: a layer whose format has no heights (depth-only format 2) draws at world y = 0, not at the layer's surface/minimum height. Implemented for the terrain liquid mesh; every sampled ocean layer already carries 0.0, so no shipped pixel changes.
- [x] G14 shore-detail gate: ProcFel (material 13) binds the depth register and its shore layer at liquid detail >= 3; Water, Mercury, LeyLine and the base draw use >= 2; Magma binds no depth. `forge_liquid_pixels` now proves shore/specular at detail 3 and ignores both at 2. I2 B.4 confirms the byte is not dropped by retail: the draw vertex carries it on the second texture pair's second component, whose per-material cubic (a cubic in depth/255 with the row's four `Coefficient` values, clamped to one) the vertex program forwards as the depth interpolant. The liquid mesh therefore writes the byte to `UV2.y` and keeps the vertex colour white, and every liquid shader reads `vertex_depth = UV2.y`. It is not a shore input.
- [x] Terrain fallback greys (I2 A): an unresolved or zero diffuse FileDataID binds dark grey `0x3f3f3f`, which covers both reachable cases — a tile with no layer records and a layer whose reference never loads. Mid grey `0x808080` belongs only to the area-LOD layer builder when a record's flag bit 0 is clear; the engine has no area-LOD layer path, so that branch is unreachable here.

- [x] RF3 Forever PBR depth (2026-11-05): the PBRWater shading pass binds no scene depth and `Float_11` is not a fade scale. Thickness now comes from the water's own inputs: the `_depth` element of the row's six-mask `LiquidTypeXTexture` set (slot 4 canonically; slot 3 in rows 1250 and 1296, which the catalog detects by comparing each row's slot-3/slot-4 pair with the modal pair). The six masks are bound by role (`foam_high/mid/low/rim/depth/river`) for the PBR shader only, and `pbr_water_pixels` proves `foam_depth` drives the water (shallow 0.32/0.34/0.30 vs deep 0.008). Register 36's hard-coded `(100000.0 or 50000.0, 1.0)` distance pair is uploaded by an unpublished per-draw float, so only its unit scale participates (labelled ADAPTED). The screen-space refraction keeps Godot's screen copy at a small depth-art-scaled offset because there is no scene depth to reject an offset.
- [ ] PBR default flip: **not flipped.** The Forever three-pose comparison is inconclusive: the same build renders the Zephras coast water in two states (water-region mean 28.7 or 229.4) at roughly one run in four, so same-mode repeats differ by 66-75% of pixels, which swamps every between-mode delta; the PBR path is also markedly brighter than the fallback (mean 192 vs 68). The opt-in `GAME_ENGINE_PBR_WATER=1` stays.

## How it works

- [Liquid investigation](../wiki/investigations/northshire-pale-water.md)

## Implementation inventory

- `godot/core/src/liquid_data.rs` — LiquidType, LiquidObject, LiquidMaterial and texture resolution.
- `godot/core/src/wmo_liquid.rs` — WMO liquid type and geometry.
- `godot/rust/src/terrain/wmo_liquid.rs` and `water.rs` — shared material creation for WMO and terrain.
- `godot/shaders/liquid_magma.gdshader` and `liquid_fel.gdshader` — current reference ports, not proven Retail procedural variants.

## Tests asserting this spec

- `godot/core/tests/wmo_liquid.rs` — WMO resolution and geometry.
- `godot/core/tests/liquid_water.rs` — magma/slime material and real MH2O vertex inputs.

## Known gaps (current cycle)

- [ ] Complete E5 ProcMagma detail0 versus1..4 routing and exact UV/sampling equations. G1 confirms the existing FDID768431 loader is authored16-plane RGBA noise, not synthetic generation; array/volume semantics and cadence still await evidence.
- [x] Decode far-terrain `_lod.adt` MLLD maps: depth128x128 bytes; alpha2048-byte bitmap expanded to0/255. Big-endian RLE zero tokens are no-ops. Retain owned payload/maps independently of mesh lifetime; concrete Neltharus outer-LOD payloads cover the header/bitmap layout.
- [ ] Render those authored maps in the far-terrain LOD liquid renderer. Do not bind them to WMO liquids: the Forge tile's LOD has no MLLD.
- [x] Preserve G2's confirmed12-byte header/flags and truncate final RLE runs/literals at output capacity. Interpret depth linearly as byte/9yd (G2 shipped-asset measurement); preserve zero-depth texels that still have coverage. Coverage bit order is corroborated by real border maps.
- [x] G4 establishes Float[0..8] triples and LiquidObject direction/speed definition; restore flow multiplication, superseding the earlier zero-flow animation adaptation. Do not choose an arbitrary LiquidObject row for WMO surfaces.
- [ ] Matched before/after Forge captures and visual proof are pending a sourced rendering change.

## Out of scope

- Invented shoreline or flow policy without source evidence.
