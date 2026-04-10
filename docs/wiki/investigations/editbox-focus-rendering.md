# Editbox Focus Rendering

The char-create editbox needs a visible focused/unfocused state change (dark fill → warm fill + gold border). Focus state propagation works correctly, but the nine-slice renderer has a structural limitation that prevents clean fill changes without engine-level changes.

## Finding

Six approaches were attempted; all failed for the same underlying reason:

The nine-slice center part covers only the interior, inset by `edge_size` on all sides. The border parts (T, B, L, R, corners) have transparent fill in their inner portion. There is no mechanism to fill the gap between the center tile and the visible border line. The source textures (`Common-Input-Border-M.blp`) decode to near-black pixels `RGBA(0,0,0,136)` — any color tint on near-black produces near-black.

Attempts that exposed the gap:
- Background color quad: correct concept but produces square corners that bleed through transparent corner textures.
- Inset background quad by `edge_size`: leaves an unfilled strip between the inset edge and the visible border line.
- White center texture + `bg_color` tint: center fill doesn't extend under border parts, leaving a visible gap.

## Root Cause

Nine-slice fill architecture: center tile does not extend to the border tile inner edge. WoW solves this with `backdropColor`, a solid fill rendered behind the entire frame before the nine-slice. Our `background_color` field approximates this but cannot match corner shape without masking.

## Possible Solutions

1. **Rounded background quad**: Render `background_color` with rounded corners matching nine-slice corner radius. Requires shader or geometry work.
2. **Dual nine-slice layers**: Second nine-slice underneath using solid-color textures; original border layer on top. Requires engine support.
3. **Corrected baked textures**: Composite fill into border textures but preserve original alpha on corner outer pixels (attempt 5 used `max(ba, fill_a)` which flattened corner transparency — should preserve original alpha instead).
4. **Reduce layout `edge_size` with larger UV `edge_size`**: Center extends further in layout space; corners are squished to `edge_size` px.

## Sources

- [editbox-focus-texture-swap-2026-04-06.md](../../editbox-focus-texture-swap-2026-04-06.md) — all six approaches, failure modes, and proposed solutions

## See Also

- [[target-circle-rendering]] — separate alpha/blend mode decisions for UI rendering
