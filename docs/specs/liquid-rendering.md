# Liquid rendering

WMO MLIQ and terrain MH2O liquids use LiquidType-backed material data. The [liquid investigation](../wiki/investigations/northshire-pale-water.md) records the renderer and source evidence.

## What it must do

- [x] Resolve the Neltharus Forge WMO group's authored LiquidType and its material and texture data, rather than selecting a shader from its liquid name.
- [x] Resolve magma and slime LiquidType data through their LiquidMaterial class and retain authored MH2O vertex data.
- [ ] Render Material 2/4 with Retail ProcMagma behavior at the applicable detail level using sourced inputs and equations.
- [ ] Render the reported Neltharus Forge liquid with sourced Retail Material 13 behavior, including any applicable flow and shoreline data.

## How it works

- [Liquid investigation](../wiki/investigations/northshire-pale-water.md)

## Implementation inventory

- `godot/core/src/liquid_data.rs` — LiquidType, LiquidObject, LiquidMaterial and texture resolution.
- `godot/core/src/wmo_liquid.rs` — WMO liquid type and geometry.
- `godot/rust/src/terrain/wmo_liquid.rs` and `water.rs` — shared material creation for WMO and terrain.
- `godot/shaders/liquid_magma.gdshader` and `liquid_fel.gdshader` — current reference ports, not proven Retail procedural variants.

## Tests asserting this spec

- `godot/core/tests/wmo_liquid.rs` — WMO resolution, geometry and Neltharus material inputs.
- `godot/core/tests/liquid_water.rs` — magma/slime material and real MH2O vertex inputs.

## Known gaps (current cycle)

- [ ] ProcMagma draw bindings and displacement equations are not established.
- [ ] Area depth/alpha blob provenance and its use by the Forge lake are not established.
- [ ] The cause of the reported hard edges and the correct Material 13 WMO flow are not established.
- [ ] Matched before/after Forge captures and visual proof are pending a sourced rendering change.

## Out of scope

- Invented shoreline or flow policy without source evidence.
