# Torch Halo Blend Modes

The torch model (`club_1h_torch_a_01.m2`) rendered a large golden halo even with particle emitters disabled. The halo was caused by an incorrect blend mode fallback that treated unknown blend mode values as opaque instead of additive.

## Finding

The torch has two skin batches: batch 0 uses material with `blend_mode=0` (Opaque), batch 1 uses `blend_mode=2` (Alpha blend). The skin batch parser and material mapping were already correct. The golden halo came from a different problem: the blend mode match arm used `_ => AlphaMode::Add` (fallback to additive), which was correct, but an earlier version used `_ => AlphaMode::Opaque`. Opaque fallback on an emissive/fx texture forces the entire texture quad to render as a solid golden rectangle.

The torch asset itself was not the bug — the batch-to-material mapping was valid.

## Root Cause

The blend mode fallback arm (`_ =>`) in the M2 material builder was `AlphaMode::Opaque`. For `blend_mode` values outside the `0..=7` range on fx/emissive textures, opaque is the wrong default and produces fully-solid visual artifacts.

## Resolution

- Fallback changed to `AlphaMode::Add` for unknown blend mode values — safer default for emissive/fx textures since unlit black pixels contribute nothing.
- Regression test added to assert the torch skin batch/material mapping stays valid.

**WMVx blend mode reference** (from `~/Repos/WMVx/src/ModelRenderPassRenderer.cpp`):
- 0: Opaque, 1: Alpha test, 2: Alpha blend, 3: Additive (SRC_COLOR), 4: Additive alpha (SRC_ALPHA), 5: Modulate, 6: ModulateX2, 7: Blend add (ONE, ONE_MINUS_SRC_ALPHA)

## Sources

- [torch-halo-investigation-2026-03-30.md](../../torch-halo-investigation-2026-03-30.md) — findings and resolution

## See Also

- [[m2-materials]] — M2 material and blend mode handling (if page exists)
- [[target-circle-rendering]] — related blend mode decisions for BLP ring textures
