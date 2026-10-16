# Terrain MCSH surface shadows

Carry authored ADT shadow data to native terrain-surface materials without replacing MCAL, MCCV or dynamic lighting. User-supplied S3 clean-room contract (2026-10-15); implementation in `godot/core` and native terrain material/shader code. [Terrain wiki](../wiki/systems/terrain.md#mcsh-surface-shadow-clean-room-continuation).

## What it must do

- [ ] Preserve the resolved chunk mask from root/texture-companion parsing through native material construction, separately from MCAL and MCCV.
- [ ] Zero/full/half masks produce distinct uploaded coverage; missing masks have no payload. Half-mask texels follow the actual chunk UV orientation.
- [ ] A real nontrivial masked chunk has native 1920×1080 coverage proof with no grass/doodads or UI-skin dependency.
- [ ] Apply the target-established modulation equation only under its established enable policy, preserving authored layers, color and lighting.
- [ ] Establish composition with Godot dynamic shadows rather than double-darkening overlapping shadows.

## How it works

- [Terrain — MCSH continuation](../wiki/systems/terrain.md#mcsh-surface-shadow-clean-room-continuation)
- [Existing split-file shadow ownership](split-adt-shadows.md)

## Implementation inventory

- `godot/core/src/asset/adt_format/adt/parsing.rs`, `godot/core/src/adt.rs` — existing split-mask resolution and retained chunk payload.
- `godot/rust/src/terrain/material.rs` — chunk material inputs and GPU resource construction.
- `godot/rust/src/terrain/mcsh_probe.rs` — cache-only actual-chunk acceptance entry; no runtime WDT selection changes.
- `godot/shaders/terrain.gdshader` — terrain surface shader.

## Tests asserting this spec

- `godot/tests/terrain_mcsh_pixels.gd` — native material binding, mask upload, unchanged geometry/MCCV/MCAL, and explicitly diagnostic coverage pixels for zero/full/half/missing/real/disabled cases.
- `godot/core/src/asset/adt_format/adt_tests/mcnk.rs` — existing split ownership, conflict rejection and edge-fix behavior.

The diagnostic maps mask data to visible texels; black/white diagnostic pixels are **not** target shadow brightness assertions. Full surface-shading acceptance is blocked on the gaps below.

## Known gaps (current cycle)

- [ ] **S3-modulation-equation:** inspected allowed WWV terrain shaders do not consume MCSH; pinned wow.export c2fd7bd reads only its offset. No target modulation texture/equation is established. Never invent a darkness constant.
- [ ] **S3-enable-policy:** supplied spec establishes a renderer-internal world enable bit, not its mapping to native graphics settings or target renderer modes. Diagnostic enable/disable controls do not establish that policy.
- [ ] **S3-dynamic-composition:** allowed sources do not establish how authored coverage combines with Godot's directional shadow attenuation.
- [ ] **S3-modern-applicability:** non-gating local70391 census is separate. The cached2703_31_36 chunk fixture is genuine data but not current-WDT active-tile or exact-build authentication evidence.

## Out of scope

- Grass/detail-doodad shadow shading (separate existing consumer), new terrain visibility/streaming rules, replacing authored textures or coloring terrain as a shadow substitute.
- Decompiled/leaked client sources, external CDN downloads and guessed rendering constants.
