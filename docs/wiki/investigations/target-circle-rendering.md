# Target Circle Rendering

WoW's unit selection circle is rendered procedurally by the client — not as an M2 model or dedicated texture. WoW does ship white ring BLP textures for glow/ring effects, and these can substitute for a procedural ring. Additive vs alpha-blend selection depends on whether the BLP has a real alpha channel.

## Finding

### WoW Reference Behavior

WoW renders selection circles as procedural ground-projected rings tinted by unit reaction (friendly/hostile/neutral). No dedicated selection-circle M2 or BLP is used for the base ring.

### Available BLP Ring Textures

| FDID | Path | Format | Use |
|------|------|--------|-----|
| 167208 | `spells/whiteringthin128.blp` | DXT1 128×128 | Thin ring, no alpha |
| 167207 | `spells/whiteringfat128.blp` | DXT1 128×128 | Fat ring, no alpha |
| 651522 | `spells/whitering_glow.blp` | DXT5 256×256 | Soft glow ring, real alpha |
| 623667 | `spells/whitering_double_soft.blp` | DXT5 | Double ring, real alpha |

Spell area indicators (`spells/targetarea_*.blp`) are DXT5 AoE placement textures, not unit selection rings.

### Blend Mode Selection

- **DXT1** (no alpha, `fix_1bit_alpha` sets all alpha to 255): use `AlphaMode::Add`. Black pixels contribute nothing; bright pixels glow.
- **DXT5** (real alpha): use `AlphaMode::Blend`.

Detection: `is_fully_opaque()` after BLP decode — if all pixels have alpha=255, use additive; otherwise alpha blend.

### Material Setup

Both types use `StandardMaterial` with `base_color` as tint, `base_color_texture` + `emissive` + `emissive_texture` for glow, `unlit: true`, `cull_mode: None`.

## Resolution

A `TargetCircleStyle` resource was added with a picker in `InWorldSelectionDebug` for switching between procedural and BLP-textured styles. The blend mode detection (`is_fully_opaque()`) applies automatically based on decoded alpha.

## Sources

- [target-circle-styles-2026-03-30.md](../../target-circle-styles-2026-03-30.md) — approach comparison, BLP inventory, material setup

## See Also

- [[torch-halo-blend-modes]] — M2 material blend mode reference (WMVx values)
