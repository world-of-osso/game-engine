# Terrain ground detail (Godot client)

Grass and flower cards ("ground clutter", detail doodads) that a terrain layer's GroundEffectTexture scatters over its MCNK cells. Scatter and mesh expansion live in `godot/core/src/ground_detail.rs`, a port of solarityclient `crates/rendering/src/terrain/detail_doodad` (client build 12340 functions 7D3390, 7B1B50, 7B31E0); drawing in `godot/rust/src/terrain/ground_detail.rs` and `godot/shaders/ground_detail.gdshader`. See [terrain](../wiki/systems/terrain.md#ground-detail-godot).

## What it must do

- [x] Select `groundEffectDensity` (64) cells per chunk with Blizzard's table generator seeded by the chunk's global row/column; a cell's effect is the MCLY layer chosen by MCNK header 0x40; holes (low-res, and retail high-res per cell) and header 0x50 exclude the cell. Matches the original client's placements for all six solarityclient native cases.
- [x] Place the effect's density (0 means 8) of doodads from its 16-slot weighted distribution, on the cell face under them, rejecting faces with normal z < 0.4; tint by the interpolated MCCV unless the doodad has flag 2; MCSH-shadowed doodads carry alpha 0.
- [x] Expand placements into at most four texture buckets of `min(density × 64, 4096)` vertices/indices, rotating about the up axis or (flag 1) about the face normal per cell face. Matches the original client's vertices for all six native cases.
- [x] Retail data: GroundEffectTexture (1308499) and GroundEffectDoodad (1308057) exported to `data/db2/12.1.0.69933/*.csv`; every Northshire (azeroth_32_48) placement stands on the rendered terrain.
- [x] Detail the chunks whose centre is within `groundEffectDist` (140) plus half a chunk diagonal of the camera; free them a chunk farther out. Draw two-sided, depth-writing, alpha-tested at 128, lit by ambient + direct × N·L with the terrain face normal, authored shadows darkening 30%, fading out over the last 15% of the distance, with the retail scene fog. Live Northshire fixture (`godot/tests/world_ground_detail_flow.gd`): the GroundDetail node changes ≥ 5000 pixels.
- [ ] Retail parity of density and look: retail GroundEffectTexture weights are percentages (e.g. 4/4/4/88), which the 12340 distribution reduces to its last doodad; GroundEffectDoodad size/rotation variation, wind animation and MCDD are not used. Not compared against a Retail capture.
- [ ] MCSH is sampled after the parser's edge fix (column 63 copies 62); the client samples the authored bits.
- [ ] Options control (groundEffectDensity / groundEffectDist) and a draw-distance setting.
