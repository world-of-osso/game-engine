# Target Circle Style Investigation (2026-03-30)

## Summary

Added a target circle style picker to the InWorldSelectionDebug screen. Allows switching between procedural and BLP-textured selection circles under the targeted wolf.

## WoW Selection Circle

WoW's unit selection circle is **not** an M2 model or a dedicated BLP texture. The client renders it procedurally as a ground-projected ring, tinted by unit reaction (red/green/yellow).

However, WoW does ship white ring textures that are used for various glow/ring effects:

| FDID | Path | Format | Notes |
|------|------|--------|-------|
| 167208 | `spells/whiteringthin128.blp` | DXT1 128×128 | Thin ring, white-on-black, no alpha |
| 167207 | `spells/whiteringfat128.blp` | DXT1 128×128 | Fat ring, same encoding |
| 651522 | `spells/whitering_glow.blp` | DXT5 256×256 | Soft glow ring, has real alpha |
| 623667 | `spells/whitering_double_soft.blp` | DXT5 | Double ring with soft edges |
| 166706 | `spells/reticle_128.blp` | DXT1 128×128 | Crosshair reticle |

Spell area targeting indicators (Holy, Fire, Arcane, etc.) are at `spells/targetarea_*.blp` — these are DXT5 with proper alpha and are used for AoE placement, not unit selection.

## Blending Modes

- **DXT1 textures** (no alpha channel): `fix_1bit_alpha` sets all alpha to 255. Ring shape is in RGB luminance only. Must use **additive blending** (`AlphaMode::Add`) — black contributes nothing, bright pixels glow.
- **DXT5 textures** (real alpha): Use standard **alpha blending** (`AlphaMode::Blend`). Alpha channel defines transparency.

Detection: `is_fully_opaque()` checks if every pixel has alpha=255 after BLP decode. If so, additive; otherwise alpha blend.

## Material Setup

Both types use the same `StandardMaterial`:
- `base_color`: tint color (reaction color for rings, school color for spell areas)
- `base_color_texture`: the BLP texture (tint multiplies RGB)
- `emissive` + `emissive_texture`: same texture for glow effect
- `unlit: true`, `cull_mode: None`

## Files Changed

- `src/target.rs` — `TargetCircleStyle` resource, `available_circle_styles()`, additive vs alpha blend detection
- `src/ui/screens/inworld_selection_debug_component.rs` — Circle style picker panel (right side)
- `src/inworld_selection_debug_screen.rs` — Wired `SelectCircleStyle` action to `TargetCircleStyle` resource

## BLP Textures Extracted

All saved to `data/textures/` by FDID:
167091, 167207, 167208, 294443, 623667, 651522, 166706, 533708, 1001600, 1001601, 1001690, 1001693, 1001694, 1001695, 1001697, 1011990
