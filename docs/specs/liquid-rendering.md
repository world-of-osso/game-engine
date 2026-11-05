# Liquid rendering

WMO MLIQ and terrain MH2O liquids use LiquidType-backed material data. The [liquid investigation](../wiki/investigations/northshire-pale-water.md) records the renderer and source evidence.

## What it must do

- [x] Resolve the Neltharus Forge WMO group's authored LiquidType and its material and texture data, rather than selecting a shader from its liquid name.
- [x] Resolve magma and slime LiquidType data through their LiquidMaterial class and retain authored MH2O vertex data.
- [x] Magma and ProcMagma colour chains (BLS, all four permutations): neither family binds a shore. Magma at detail 0 is `t0(TC1.xy).rgb` with alpha 1 and no noise layer; ProcMagma adds the noise-weighted three-layer chain. The noise RGB supplies successive lerp weights, not UV offsets.
- [ ] ProcMagma's interpolated UV planes (TC1.zw, TC2, TC3), the per-draw cb1[12].w noise slice and the cb1[12].xyz baseline colour. The engine's liquid vertex format supplies one UV plane, so those inputs are not available yet.
- [x] Retry unknown liquid type IDs, including zero without a geometry-derived type, through same-product LiquidType8 and its material/textures; never select Material8 directly.
- [ ] Render Material 2/4 with Retail ProcMagma behavior at the applicable detail level using sourced inputs and equations.
- [x] Keep Forge on Material13/Fel: two base layers, detail>=1 emissive/procedural, detail>=2 shore and detail>=3 specular. G4 UV triples are (scale,phaseDegrees,velocity); phase adds LiquidObject.FlowDirection and velocity multiplies FlowSpeed. Missing/zero-flow records remain static, with no guessed WMO flow.
- [x] G8/Fel shore: D = saturate((linear scene depth - liquid surface depth) / Float_11), sampled as t5(TC1.z, 1-D) with `shore.rgb + (base - shore.rgb)*(D*shore.a)`. `cb1[12]` = {Float_13, 1/Float_11, Float_14, Float_15} and `cb1[13]` = {pair blend, Float_16, Float_17, 0} are bound from the row. The material stays opaque; the t16 screen composite is out of scope.
- [x] Use integer-millisecond wrap with G4's finite velocity domain (1e-5<abs(v)<1000), negative cycles ending at1. Scale does not rotate the source UV.
- [ ] ProcFel permutation/technique selection and its 18 techniques; the engine renders both `Fel` and `ProcFel` detail tiers from one detail-gated shader. The BLS samples t6 through the draw item's UV rectangle, so the exact CPU texture bound there is still open.
- [x] G7 terrain contract: an `MH2O` layer's id resolves verbatim with no 1..20 rewrite and no type-8 retry, and the draw skips a row whose material column is Fog (10). The rewrite and the retry remain WMO-only.
- [x] MH2O depth bytes are handed through verbatim (no unit conversion in the parser or material), matching G7's 9-units-per-yard asset measurement; the draw keeps the existing depth/255 cubic input.

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
