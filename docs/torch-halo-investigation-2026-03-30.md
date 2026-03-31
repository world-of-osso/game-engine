# Torch halo investigation (2026-03-30)

## Problem

The torch model (`club_1h_torch_a_01.m2`) renders a large golden halo that persists even with particle emitters disabled. This halo is not present in WoW.

## Findings

### Re-checking the actual asset

The earlier parser diagnosis was wrong. The current torch asset parses as:

```
Materials (2):
  mat[0]: render_flags=0x0000 blend_mode=0
  mat[1]: render_flags=0x0011 blend_mode=2

Skin batches (2):
  batch[0]: submesh=0 mat_idx=0 texture_id=0
  batch[1]: submesh=1 mat_idx=1 texture_id=1
```

So the torch `.skin` file is using valid 24-byte batch records, and the batch-to-material mapping is correct.

### Blend mode fallback

```rust
match blend_mode {
    0 => AlphaMode::Opaque,
    1 => AlphaMode::Mask(224.0 / 255.0),
    2 | 3 | 7 => AlphaMode::Blend,
    4..=6 => AlphaMode::Add,
    _ => AlphaMode::Add,
}
```

Unknown `blend_mode > 7` values now fall back to additive instead of opaque, which is a safer default for emissive/fx textures.

### WMVx blend mode reference

From `~/Repos/WMVx/src/ModelRenderPassRenderer.cpp`:
- 0: Opaque (no blending)
- 1: Alpha test (GL_GEQUAL 0.7)
- 2: Alpha blend (SRC_ALPHA, ONE_MINUS_SRC_ALPHA)
- 3: Additive (SRC_COLOR, ONE)
- 4: Additive alpha (SRC_ALPHA, ONE)
- 5: Modulate (DST_COLOR, SRC_COLOR)
- 6: ModulateX2 (DST_COLOR, SRC_COLOR)
- 7: Blend add (ONE, ONE_MINUS_SRC_ALPHA)
- default: assert(false) — invalid

## Resolution

1. Keep the existing skin batch parser behavior; the torch asset is already parsed correctly
2. Add a regression test so the torch batch/material mapping stays valid
3. Treat unknown `blend_mode` values as additive instead of opaque
