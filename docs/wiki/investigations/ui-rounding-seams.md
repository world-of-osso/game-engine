# UI Rounding Seams

**Symptom**: at UI scale 2/3 (the in-world default), multi-piece art showed a 1-device-pixel scene-coloured vertical seam where adjacent textures should meet. The auction house tabs showed it at every cap/middle join. The dumped rects of Tab1 (Buy) had a gap: left cap 577.5–612.0, middle from 613.5.

**Root cause**: Bevy 0.19 lays out with taffy 0.10.1, whose `round_layout` rounded a node's location parent-relative (`round(location.x)`) but its size from cumulative coordinates (`compute/mod.rs:230-232`). Under a parent at a fractional pixel, a child's rendered left edge is the sum of rounded relative offsets, but its right edge comes from rounded absolute coordinates, so abutting siblings disagree by one pixel. The authored layout (cap at -1, width 35; middle at 34) has no gap.

**Ruled out**: Bevy UI edge anti-aliasing (`UiAntiAlias::Off` left the capture unchanged), atlas bleeding (the edge texels of `uiframe-tab-*` in atlas 4707839 are alpha 255), and mipmaps (the BLP loader uploads level 0 only).

**Fix**: game-engine patches taffy to `../bevy-patches/taffy`, crates.io 0.10.1 with upstream's unreleased change `location = round(cumulative) - round(parent_cumulative)`. Commits: bevy-patches `20ed24d`, game-engine `f1a6b16d`.

**Proof**:
- `bevy-patches/taffy/tests/rounding_gaps.rs` lays out the Buy tab's three pieces at physical coordinates. RED: left cap ends at 22, middle starts at 23. GREEN with the fix.
- A live capture shows every tab edge contiguous (Tab1 613.5/621.0, Tab2 667.5/675.0, Tab3 721.5/742.5) and no seams.
- Evidence: `data/diagnostics/ahtabs-20260926/` (`buy.webp` before, `buy-taffy.webp` after).

## Turned textures (2026-10-04)

Symptom: the Forever XP bar's top and bottom border edges sat 1 px right of and below the corner pieces.

`TextureData.rotation` turns a texture about its centre. The `UI-Tooltip-Border` top and bottom edges are such textures (`inworld_unit_frames_flare.rs` `border_pieces`): the XP bar's is 16×569 at (292.5, −276.5), a half-pixel origin whenever the frame width minus the edge is odd. Layout rounded that rect to pixels (+0.5, +0.5), then Godot snapped the turned `TextureRect` (it rounds a control's transform origin at quarter turns) another +0.5 on each axis.

Fix: `layout.rs` keeps the unrounded rect of a turned texture (`is_turned`), and `projection.rs` puts its node on whole pixels and adds the remainder to the turned image, so the image is snapped once, as drawn. Borders with an even difference (player frame 240 wide) were never affected. Test: `ui::layout::tests::flare_border_edges_meet_their_corners`.
