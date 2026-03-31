# Torch halo investigation (2026-03-30)

## Problem

The torch model (`club_1h_torch_a_01.m2`) renders a large golden halo that persists even with particle emitters disabled. This halo is not present in WoW.

## Findings

### Material data

```
Materials (3):
  mat[0]: render_flags=0x0002 blend_mode=0xFFFF  ← invalid
  mat[1]: render_flags=0x0000 blend_mode=0
  mat[2]: render_flags=0x0000 blend_mode=0
```

`blend_mode=0xFFFF` is invalid (valid range 0-7). WMVx triggers `assert(false)` on values outside 0-7 — it considers this a data error.

### Skin batch parsing issue

```
Skin batches (count=2, offset=0x230):
  batch[0]: mat_idx=99  ← out of range (only 3 materials exist)
  batch[1]: garbage values throughout
```

The skin file parser is producing invalid material indices. `mat_idx=99` for a model with 3 materials means the batch-to-material mapping is wrong. This likely causes fallback behavior where our renderer picks up `blend_mode=0xFFFF` incorrectly.

### Our blend mode fallback

```rust
// src/m2_spawn.rs:514
let alpha_mode = match batch.blend_mode {
    1 => AlphaMode::Mask(224.0 / 255.0),
    2 | 3 | 7 => AlphaMode::Blend,
    4..=6 => AlphaMode::Add,
    _ => AlphaMode::Opaque,  // ← 0xFFFF lands here
};
```

`0xFFFF` maps to Opaque, rendering flame effect textures as solid quads = visible halo.

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

## Next steps

1. Investigate why the skin file batch parser produces `mat_idx=99` — likely a stride or offset error in the skin parsing code
2. Once skin parsing is correct, the batch should reference a valid material with a proper blend mode
3. As a safety net, treat `blend_mode > 7` as additive or skip the batch entirely
