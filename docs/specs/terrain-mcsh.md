# Terrain MCSH surface shadows

Carry authored ADT shadow data to native terrain-surface materials without replacing MCAL, MCCV or dynamic lighting. User-supplied S3 clean-room contract (2026-10-15); implementation in `godot/core` and native terrain material/shader code. [Terrain wiki](../wiki/systems/terrain.md#mcsh-surface-shadow-clean-room-continuation).

## What it must do

- [x] Preserve the resolved chunk mask from root/texture-companion parsing through native material construction, separately from MCAL and MCCV.
- [x] Zero/full/half masks produce distinct uploaded coverage; missing masks have no payload. Half-mask texels follow the actual chunk UV orientation.
- [x] A real nontrivial masked chunk has native 1920×1080 coverage proof with no grass/doodads or UI-skin dependency.
- [ ] Apply the supplied byte RGB equation as an ordinary alpha-blended second draw, gated solely by world word bit0x10; fixed-ambient per-texel RED/GREEN proof pending. Production alpha-index mapping and flag ownership remain gaps.
- [ ] Establish composition with Godot dynamic shadows rather than double-darkening overlapping shadows.

## How it works

- [Terrain — MCSH continuation](../wiki/systems/terrain.md#mcsh-surface-shadow-clean-room-continuation)
- [Existing split-file shadow ownership](split-adt-shadows.md)

## Implementation inventory

- `godot/core/src/rendering/terrain/terrain_material_data.rs` — linear R8 data coverage unpacking; not a target modulation equation.
- `godot/core/src/asset/adt_format/adt/parsing.rs`, `godot/core/src/adt.rs` — existing split-mask resolution and retained chunk payload.
- `godot/rust/src/terrain/material.rs` — chunk material inputs and GPU resource construction.
- `godot/rust/src/terrain/mcsh_probe.rs` — cache-only actual-chunk acceptance entry; no runtime WDT selection changes.
- `godot/shaders/terrain.gdshader` — terrain surface shader.

## Tests asserting this spec

- `godot/core/tests/terrain_mcsh.rs` — every mask bit, half orientation/corners and real split-companion ownership.
- `godot/tests/terrain_mcsh_pixels.gd` — native material binding, mask upload, unchanged geometry/MCCV/MCAL, and explicitly diagnostic coverage pixels for zero/full/half/missing/real/disabled cases.
- `godot/core/src/asset/adt_format/adt_tests/mcnk.rs` — existing split ownership, conflict rejection and edge-fix behavior.

The diagnostic maps mask data to visible texels; black/white diagnostic pixels are **not** target shadow brightness assertions. Full surface-shading acceptance is blocked on the gaps below.

## Known gaps (current cycle)

- [ ] **S3-alpha-index2:** user-supplied evidence establishes RGB `(85*(ambient_byte+3))>>8` and ordinary alpha-blended second draw. Historical `m_colors[2].r` is not yet mapped to a modern LightData channel; `ShadowOpacity` and CSV column order are not assumed equivalent. Fixed tests provide that operand explicitly; production alpha is left unset rather than invented.
- [ ] **S3-enable-policy:** supplied spec establishes a renderer-internal world enable bit, not its mapping to native graphics settings or target renderer modes. Diagnostic enable/disable controls do not establish that policy.
- [ ] **S3-dynamic-composition:** allowed sources do not establish how authored coverage combines with Godot's directional shadow attenuation.
- [ ] **S3-active-world-applicability:** the non-gating70391 census is separate. The selected root/texture companion, WDT and eight terrain textures match their content keys in the fresh current70391 CASC namespace (11/11); that does not make this tile active in current WDT MAIN.

## Out of scope

- Grass/detail-doodad shadow shading (separate existing consumer), new terrain visibility/streaming rules, replacing authored textures or coloring terrain as a shadow substitute.
- Decompiled/leaked client sources, external CDN downloads and guessed rendering constants.

## Proof scope

Native binding RED at0e87e1ad2 precedes the e3f79f0b1 implementation. Forward-integratedca760b954 passes CPU2/2, native diagnostic6/6 (covered cells0/4096/2048/0/2230/0), existing terrain material14/14 and dynamic-shadow6/6 GPU controls, native build and fmt. Seven1920×1080 PNGs are under `/syncthing/AgentShared/2026-10-15/terrain-mcsh/`; real/half coverage and the unmodified textured surface were inspected. These are DATA/UV/regression proofs, not a target MCSH modulation pass. Full S3 remains blocked on the named equation/enable/composition gaps. [Provenance and ledger](../wiki/systems/terrain.md#mcsh-surface-shadow-clean-room-continuation).

### Supplied modulation continuation

Native RED (61aa7aa8a plus fixture) reports `Authored MCSH has no surface modulation pass` in `terrain-mcsh-modulation-red.log`. Implementation now draws a next-pass unshaded MCSH overlay with exact supplied RGB byte arithmetic and ordinary alpha blend in the framebuffer. Ambient uses the same `light_ambient()` scene pipeline as terrain; fixed fixture supplies24/90/180-byte ambient and128/255 alpha. Neither alpha128 nor gate16 is installed as a production default: alpha mapping and ownership of the renderer enable word remain unresolved. Dynamic attenuation is unchanged in the original terrain pass; an explicitly enabled MCSH draw currently overlays that result, with no guessed overlap suppression.

**Goldshire data boundary:** current Azeroth WDT775971 MAID tiles31_49/32_49 resolve roots777832/778032 and tex0777835/778035; both roots have0/256 MCSH flags and both companions contain zero MCSH payloads. Authentic masked Goldshire before/after cannot be claimed from those bytes. Existing authenticated masked2703 chunk remains the nontrivial raster fixture.
