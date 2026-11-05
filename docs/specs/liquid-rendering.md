# Liquid rendering

WMO MLIQ and terrain MH2O liquids use LiquidType-backed material data. The [liquid investigation](../wiki/investigations/northshire-pale-water.md) records the renderer and source evidence.

## What it must do

- [x] Resolve the Neltharus Forge WMO group's authored LiquidType and its material and texture data, rather than selecting a shader from its liquid name.
- [x] Resolve magma and slime LiquidType data through their LiquidMaterial class and retain authored MH2O vertex data.
- [x] Retry unknown liquid type IDs, including zero without a geometry-derived type, through same-product LiquidType8 and its material/textures; never select Material8 directly.
- [ ] Render Material 2/4 with Retail ProcMagma behavior at the applicable detail level using sourced inputs and equations.
- [x] Keep the Forge on Material13/Fel with two base layers, detail>=1 emissive/procedural/shore and detail>1 specular; WMO UV scrolling must not be multiplied by absent LiquidObject flow.
- [ ] Complete sourced Retail Material13 UV equations and authored MLLD shoreline integration. Current Float triples and fragment equations remain a qualified WebWowViewerCpp adaptation.

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
- [ ] Confirm Retail header flags and coverage bit order independently of the observed payloads; depth world units remain unknown.
- [ ] Exact Retail environment-to-DB2 UV-triple and definition scale/bias mapping remains unestablished. The fix removes zero LiquidObject-flow multiplication without choosing an arbitrary LiquidObject row.
- [ ] Matched before/after Forge captures and visual proof are pending a sourced rendering change.

## Out of scope

- Invented shoreline or flow policy without source evidence.
