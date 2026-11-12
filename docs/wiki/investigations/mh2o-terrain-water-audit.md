# MH2O terrain water audit (G7)

Audit of the ADT `MH2O` loader and terrain water rendering against
[R-3d-terrain-water-mh2o](../../../../.local/state/wow-ui-sim/handoff/native-evidence-70688/R-3d-terrain-water-mh2o.md)
(Retail 12.1.5.70688). Every fact is matched or a gap, with the code that proves it.

## Fact table

| Decomp fact | Status | Evidence |
|---|---|---|
| The type id is the layer's first `u16`, verbatim, with no 1..20 rewrite and no type-8 retry (those are WMO-only). | Matched | `wow-adt/src/adt_tex_water.rs:12` (`liquid_type: u16` is the first field of `LiquidInstanceHeader`); `godot/core/src/liquid_data.rs:583` (`u32::from(liquid_type)`); `liquid_data.rs:660-670` (`resolve_type_row` returns `Err` for `LiquidFamily::Terrain`); the 1..20 rewrite lives only in `wmo_liquid::group_liquid_type`. |
| Skip the layer when the material column is 10. | Matched | `liquid_data.rs:265-267` (`terrain_draws_surface`), called at `godot/rust/src/terrain/water.rs:111`. |
| Per-vertex heights sit on a 9x9 grid per chunk, plus 1 depth byte per vertex in units of 1/9 yd (as in MLLD). | Matched | `wow-adt/src/adt_tex_water.rs:181-215` reads the height floats then the depth bytes; `wow-adt/src/adt_water_geometry.rs:22` (`WATER_STEP = CHUNK_SIZE / 8.0`); `adt_water_geometry.rs:60-70` (`depth / 255`); `godot/core/tests/adt_water_geometry.rs:165` proves the 1/9 yd unit against the ADT terrain on 847 samples. |
| Coverage is 1 bit per tile. | Matched | `wow-adt/src/adt_tex_water.rs:110-145` (`read_exists_bitmask`, bit `y * width + x`); `adt_water_geometry.rs:24-26` (`quad_exists`). |
| Ocean is type 2 at height 0, plus a MapWater object (definition key 42). | Matched in effect; one gap fixed | Type 2 at world y = 0: `godot/rust/src/terrain/water.rs:118-124`. Object 42 is the sentinel and is never looked up: `liquid_data.rs:578-583`. The LVF 2 rule is now keyed on the layer's type with a sentinel format field (`liquid_data.rs:594-600`, commit `6cf912f36`). No separate MapWater object exists: the per-chunk layers cover the same tiles, so the rendered surface is equivalent; the tile-wide object is an internal client structure, not a second visible surface. |
| The Northshire water shader's depth scaling uses the 1/9 yd unit, and terrain water applies no type remap. | Matched | `godot/shaders/water.gdshader:100-102` (`vertex_depth = UV2.y`), `:190-200` (`d = vertex_depth`, `color_mix = d * f2`); `byte / 255 = depthYards / 28.333`, so the 1/9 yd unit is the shader's depth scale. `liquid_data.rs:252-258` (`LiquidFamily::Terrain`) applies no type remap. |

## Gap fixed

`6cf912f36` — the ocean vertex-format rule was keyed on `liquid_object == 42`
instead of the layer's own type with a sentinel format field. A type 2 layer on a
real `LiquidObject` above 42 would have read its material's LVF 0 heights from a
depth-only block. The rule is now `liquid_object >= 42 && liquid_type == 2`, as the
decomp states ("the client returns 2 for type 2 whenever the format field is a
sentinel"). The same commit updates the two stale magma/fel tests that passed a
sub-42 value as the format field.

## Proof

- `godot/core/tests/liquid_water.rs` 21/21, `adt_water_geometry.rs` 5/5.
- New `ocean_coast_tile_places_type_two_layers_at_sea_level` on the real
  `azeroth_31_58` tile (FDID 777877): 35 type 2 object 42 layers with sentinel
  format 42, surface and minimum height 0.0, LVF 2 depths and emitted world y = 0;
  20 type 5/1 water layers keep their material LVF 0 and authored heights.
- Northshire `azeroth_32_48` (FDID 778027) keeps its 42 type 5 object 427 layers
  with authored depths (`northshire_stream_layers_carry_varying_vertex_depth`).

## Captures

Forever, 960 px JPEG, same pose before (master `fb61fee77`) and after
(`6cf912f36`): `/syncthing/AgentShared/2026-11-08/mh2o-water/`
(`northshire-{before,after}.jpg`, `ocean-{before,after}.jpg`). The fix changes no
shipped layer, so the pairs are identical within the water-animation repeat floor
(Northshire PSNR 74.6 dB, ocean 45.3 dB).

## Per-frame cost

`frame_benchmark.gd` at the Northshire water pose (1920x1080, 10 s segments,
private server UDP 5960). The fix is a no-op for shipped data, so the per-frame
cost is unchanged within host-load noise:

| Build | idle client ms | idle main-thread ms | idle render ms | draws | load 1 min |
|---|---:|---:|---:|---:|---:|
| master `fb61fee77` | 3.571 | 4.309 | 2.713 | 1737 | 41.59 |
| `6cf912f36` | 1.693 | 2.147 | 0.780 | 1735 | 37.41 |

The after run is faster at a lower host load; no regression.

## Sources

- [R-3d-terrain-water-mh2o.md](../../../../.local/state/wow-ui-sim/handoff/native-evidence-70688/R-3d-terrain-water-mh2o.md) — the decomp facts.
- [R-3d-terrain-depth-curve-stage-and-data.md](../../../../.local/state/wow-ui-sim/handoff/native-evidence-70688/R-3d-terrain-depth-curve-stage-and-data.md) — the depth curve and flat-layer placement.
- [northshire-pale-water](northshire-pale-water.md) — the earlier MH2O depth and coverage work.

## See Also

- [liquid-rendering](../../specs/liquid-rendering.md) — the liquid rendering contract.
- [terrain](../systems/terrain.md) — the terrain system.
