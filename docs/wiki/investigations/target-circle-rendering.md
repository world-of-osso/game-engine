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

## Spell ground-target placement

VERIFIED: `data/community-listfile.csv` maps FDIDs 1001600/1001601/1001690/1001693/1001694/1001695/1001697 to the Fire, every-base, Arcane, Frost, Holy, Nature and Shadow `spells/targetarea_*.blp` files. They are AoE placement art; 166706 `spells/reticle_128.blp` is a crosshair and is not the target-area decal. All seven BLPs exist in `data/textures/`; `scripts/player_bundle.py` stages that full directory.

INFERRED: `SpellMisc.SchoolMask` selects the matching area art. The engine's spell catalog maps the single Fire/Holy/Nature/Frost/Shadow/Arcane masks and uses the documented every-base texture for Physical and composite/unlisted masks.

VERIFIED: In local DB2 build 12.1.5.70448, Flamestrike 2120 has school mask 4, an 8 yd destination-area radius and `SpellTargetRestrictions.Targets=64`; DnD 43265 has school mask 32 and the same destination flag and radius. Heroic Leap 6544 and Infernal Strike 189110 have 3 yd destination radii. Consecration 26573 has school mask 2, range ID1 (Self Only), an effect 179 row with target 87, and no `SpellTargetRestrictions` row. The explicit-destination restriction separates cursor spells from the caster-centered area-trigger spell.

INFERRED: The decal radius uses positive `SpellEffect` radii on destination-area targets, plus the area-capable effects that use `DEST_DEST`; unrelated ranges do not size the decal. VERIFIED: Eagle Eye 6197 has a 100 yd effect radius but its targets are not destination-area targets, so it remains a cursor spell without a fabricated 100 yd decal. VERIFIED: A destination spell without a positive area radius keeps its cursor and has no area decal.

## Resolution

A `TargetCircleStyle` resource was added with a picker in `InWorldSelectionDebug` for switching between procedural and BLP-textured styles. The blend mode detection (`is_fully_opaque()`) applies automatically based on decoded alpha.

## Transform updates

Commit `3ebbd348` retains target-circle translation and scale evaluation every update, while copying the existing rotation and applying the resulting `Transform` only when it differs. A stationary circle no longer creates a changed-transform notification; target movement still updates its translation. Proof covers those boundaries only; no CPU or rotation-specific claim.

## Sources

- [target-circle-styles-2026-03-30.md](../../target-circle-styles-2026-03-30.md) — approach comparison, BLP inventory, material setup
- `data/diagnostics/unchanged-writes-20260909/target-circle/` — conditional-transform RED/GREEN and verification evidence

## See Also

- [[torch-halo-blend-modes]] — M2 material blend mode reference (WMVx values)
