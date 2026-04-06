# Editbox Focus Visual Investigation

## Goal

Char create editbox should visibly change when focused — darker fill unfocused, brighter warm fill + gold border when selected.

## What works

- **Focus state propagates**: `name_input_focused` flows through `CharCreateUiState` → RSX → Screen rebuild. Verified by debug log and test `editbox_nine_slice_textures_change_on_focus`.
- **Diff engine updates nine_slice**: `apply_def` replaces `frame.nine_slice` with new values every rebuild. Confirmed by `[DEBUG] apply_def` log showing path switch from `dark` to `focused`.
- **Renderer updates colors**: Test `update_part_applies_new_texture_from_swapped_nine_slice` confirms sprite color changes when `frame.nine_slice` is swapped.
- **Focus click handler fixed**: All `cc.name_input` usages replaced with live `registry.get_by_name(CREATE_NAME_INPUT.0)` lookups since the editbox only exists in Customize mode (stale ID was always `None`).

## Approaches tried and why they failed

### 1. `bg_color` tint on original M texture
The original `Common-Input-Border-M.blp` decodes to near-black pixels `RGBA(0,0,0,136)`. Multiplying near-black by any tint gives near-black. No visible difference between focused/unfocused.

### 2. `border_color` tint on border parts
Border textures are also dark. Tint change from gray to gold is too subtle to notice on dark source pixels.

### 3. `background_color` solid quad behind nine_slice
Added renderer support (`is_renderable` allows nine_slice + background_color). Problem: the quad is a full rectangle with sharp corners that bleeds through the rounded/transparent corner textures. Can't clip to match corner shape without knowing the texture content.

### 4. Inset background_color quad by edge_size
Gap between the inset quad edge and the border textures — the border textures' visible line is thinner than `edge_size`, leaving an unfilled strip.

### 5. Baked tinted textures (all 9 parts)
Composited fill color under original border art for all 9 parts. Two problems:
- Corner textures became fully opaque (square corners) because fill alpha was applied to transparent corner pixels
- Colors didn't match the original WoW look

### 6. White center texture with bg_color tint
Replaced M texture with solid white 8x8, so `bg_color` tint becomes the exact fill color. Problem: the center fill doesn't extend under the border parts — the border textures have transparent inner areas, creating a visible gap between fill and border.

## Core problem

The nine_slice center part only covers the interior (inset by `edge_size` on all sides). The border parts (T, B, L, R, TL, TR, BL, BR) have transparent fill in their inner portion. There's no mechanism to fill the gap between center and visible border line.

WoW solves this with `backdropColor` — a solid fill rendered behind the entire nine_slice at the frame level. Our `background_color` approach (attempt 3) does this but produces square corners.

## Possible solutions

1. **Rounded background quad**: Render the `background_color` quad with rounded corner masking (shader or geometry) matching the nine_slice corner radius. Complex.

2. **Separate fill nine_slice**: Add a second nine_slice layer underneath with the same edge_size but using solid-color textures. The border layer renders on top. Requires engine support for dual nine_slice.

3. **Fix border textures**: Bake the fill into the border textures but preserve alpha in corner outer pixels. The compositing in attempt 5 used `max(ba, fill_a)` for alpha — should instead preserve original alpha so corners stay transparent where the border texture is transparent.

4. **Reduce edge_size + uv_edge_size**: Use smaller `edge_size` (e.g. 3) for layout so center extends further, but use `uv_edge_size: 8` to sample the full 8px of border texture. Corners would be squished to 3px layout space though.

## Files

- `src/ui/screens/char_create_component/char_create_widgets.rs` — RSX with focus-conditional nine_slice
- `src/scenes/char_create/mod.rs` — focus state propagation
- `src/scenes/char_create/input.rs` — click/focus handlers
- `ui-toolkit/src/render_nine_slice.rs` — nine_slice sprite rendering
- `ui-toolkit/src/render.rs` — `is_renderable` allows background_color on nine_slice frames
- `ui-toolkit/src/render_texture.rs` — texture loading with `missing_file_textures` blacklist
- `data/textures/editbox-white-fill.ktx2` — white 8x8 center texture for bg_color tinting
