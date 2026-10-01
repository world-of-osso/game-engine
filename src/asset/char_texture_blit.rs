use super::{CharTextureData, CompositedModelTextures, TextureLayer, TextureSection};

pub(crate) const FULL_TEXTURE_SECTION_MASK: i64 = -1;
pub(crate) const HD_TEXTURE_WIDTH: u32 = 2048;
pub(crate) const HD_TEXTURE_HEIGHT: u32 = 1024;

pub(crate) struct BlitLayerInput<'a> {
    pub pixels: &'a mut [u8],
    pub canvas_w: u32,
    pub tex: &'a [u8],
    pub tex_w: u32,
    pub tex_h: u32,
    pub layer: &'a TextureLayer,
    pub layout_id: u32,
}

pub(crate) struct BlitScaledInput<'a> {
    pub pixels: &'a mut [u8],
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub tex: &'a [u8],
    pub tex_w: u32,
    pub tex_h: u32,
    pub dx: u32,
    pub dy: u32,
    pub target_w: u32,
    pub target_h: u32,
    pub layer: &'a TextureLayer,
}

pub(crate) fn blit_layer(data: &CharTextureData, input: BlitLayerInput<'_>) {
    let BlitLayerInput {
        pixels,
        canvas_w,
        tex,
        tex_w,
        tex_h,
        layer,
        layout_id,
    } = input;
    if layer.section_bitmask == FULL_TEXTURE_SECTION_MASK {
        if let Some(section) = data.full_texture_section(layer, layout_id) {
            blit_section(pixels, canvas_w, tex, tex_w, tex_h, &section, layer);
            return;
        }
        let canvas_h = pixels.len() as u32 / (canvas_w * 4);
        blit_scaled(BlitScaledInput {
            pixels,
            canvas_w,
            canvas_h,
            tex,
            tex_w,
            tex_h,
            dx: 0,
            dy: 0,
            target_w: canvas_w,
            target_h: canvas_h,
            layer,
        });
        return;
    }
    for bit in 0..32u32 {
        if layer.section_bitmask & (1i64 << bit) == 0 {
            continue;
        }
        let Some(section) = data.sections.get(&(layout_id, bit)) else {
            continue;
        };
        blit_section(pixels, canvas_w, tex, tex_w, tex_h, section, layer);
    }
}

pub(crate) fn runtime_texture_for_section(
    data: &CharTextureData,
    pixels: Vec<u8>,
    layout_id: u32,
    width: u32,
    height: u32,
    section_type: u32,
) -> Option<(Vec<u8>, u32, u32)> {
    let section = *data.sections.get(&(layout_id, section_type))?;
    if width == HD_TEXTURE_WIDTH && height == HD_TEXTURE_HEIGHT {
        let (scaled_pixels, scaled_w, scaled_h) =
            scale_to(&pixels, width, height, width / 2, height / 2);
        let section = scaled_section(section, 2);
        let cropped = crop_rgba(
            &scaled_pixels,
            scaled_w,
            scaled_h,
            section.x,
            section.y,
            section.width,
            section.height,
        );
        return Some((cropped, section.width, section.height));
    }

    let cropped = crop_rgba(
        &pixels,
        width,
        height,
        section.x,
        section.y,
        section.width,
        section.height,
    );
    Some((cropped, section.width, section.height))
}

pub(crate) fn runtime_textures_from_layout(
    data: &CharTextureData,
    pixels: Vec<u8>,
    layout_id: u32,
    width: u32,
    height: u32,
) -> CompositedModelTextures {
    if width == HD_TEXTURE_WIDTH && height == HD_TEXTURE_HEIGHT {
        let (body_pixels, body_w, body_h) = scale_to(&pixels, width, height, width / 2, height / 2);
        let head = runtime_texture_for_section(data, pixels.clone(), layout_id, width, height, 9);
        return CompositedModelTextures {
            body: (body_pixels, body_w, body_h),
            head,
            hair: None,
        };
    }

    CompositedModelTextures {
        body: (pixels, width, height),
        head: None,
        hair: None,
    }
}

pub(crate) fn blit_section(
    pixels: &mut [u8],
    canvas_w: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    section: &TextureSection,
    layer: &TextureLayer,
) {
    let (scaled, sw, sh) = scale_to(tex, tex_w, tex_h, section.width, section.height);
    for row in 0..sh.min(section.height) {
        for col in 0..sw.min(section.width) {
            let si = ((row * sw + col) * 4) as usize;
            let dx = section.x + col;
            let dy = section.y + row;
            let di = ((dy * canvas_w + dx) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= scaled.len() {
                continue;
            }
            blend_pixel(pixels, di, &scaled, si, layer.blend_mode);
        }
    }
}

pub(crate) fn blit_scaled(input: BlitScaledInput<'_>) {
    let BlitScaledInput {
        pixels,
        canvas_w,
        canvas_h,
        tex,
        tex_w,
        tex_h,
        dx,
        dy,
        target_w,
        target_h,
        layer,
    } = input;
    for row in 0..target_h.min(canvas_h - dy) {
        for col in 0..target_w.min(canvas_w - dx) {
            let sx = (col * tex_w / target_w).min(tex_w - 1);
            let sy = (row * tex_h / target_h).min(tex_h - 1);
            let si = ((sy * tex_w + sx) * 4) as usize;
            let px = dx + col;
            let py = dy + row;
            let di = ((py * canvas_w + px) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= tex.len() {
                continue;
            }
            blend_pixel(pixels, di, tex, si, layer.blend_mode);
        }
    }
}

/// 1 blit, 9 straight alpha and 15 inferred alpha all weight by source alpha.
fn uses_source_alpha(blend_mode: u32) -> bool {
    matches!(blend_mode, 1 | 9 | 15)
}

/// ChrModelTextureLayer.BlendMode values that tint the pixels already composited
/// below them (WMVx `CharacterTextureBuilder::BlendMode`: 4 multiply, 6 overlay,
/// 7 screen). Authored HD skin-color layers (target 30) use overlay.
fn is_tint_blend(blend_mode: u32) -> bool {
    matches!(blend_mode, 4 | 6 | 7)
}

fn tint_channel(blend_mode: u32, src: u8, dst: u8) -> u16 {
    let (s, d) = (u32::from(src), u32::from(dst));
    let tinted = match blend_mode {
        4 => s * d / 255,
        6 if d < 128 => 2 * s * d / 255,
        6 => 255 - 2 * (255 - s) * (255 - d) / 255,
        _ => 255 - (255 - s) * (255 - d) / 255,
    };
    tinted as u16
}

pub(crate) fn blend_pixel(dst: &mut [u8], di: usize, src: &[u8], si: usize, blend_mode: u32) {
    let alpha = src[si + 3] as u16;
    if alpha == 0 {
        return;
    }
    if is_tint_blend(blend_mode) {
        for channel in 0..3 {
            let tinted = tint_channel(blend_mode, src[si + channel], dst[di + channel]);
            let base = dst[di + channel] as u16;
            dst[di + channel] = ((alpha * tinted + (255 - alpha) * base) / 255) as u8;
        }
        return;
    }
    if !uses_source_alpha(blend_mode) || alpha == 255 {
        dst[di] = src[si];
        dst[di + 1] = src[si + 1];
        dst[di + 2] = src[si + 2];
        dst[di + 3] = 255;
    } else {
        let inv = 255 - alpha;
        dst[di] = ((alpha * src[si] as u16 + inv * dst[di] as u16) / 255) as u8;
        dst[di + 1] = ((alpha * src[si + 1] as u16 + inv * dst[di + 1] as u16) / 255) as u8;
        dst[di + 2] = ((alpha * src[si + 2] as u16 + inv * dst[di + 2] as u16) / 255) as u8;
        dst[di + 3] = dst[di + 3].max(src[si + 3]);
    }
}

pub(crate) fn scaled_section(section: TextureSection, divisor: u32) -> TextureSection {
    TextureSection {
        x: section.x / divisor,
        y: section.y / divisor,
        width: section.width / divisor,
        height: section.height / divisor,
    }
}

fn crop_rgba(src: &[u8], src_w: u32, src_h: u32, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for row in 0..h {
        for col in 0..w {
            let sx = x + col;
            let sy = y + row;
            if sx >= src_w || sy >= src_h {
                continue;
            }
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((row * w + col) * 4) as usize;
            if si + 3 < src.len() && di + 3 < out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    out
}

pub(crate) fn scale_to(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> (Vec<u8>, u32, u32) {
    if src_w == dst_w && src_h == dst_h {
        return (src.to_vec(), dst_w, dst_h);
    }
    if src_w * 2 <= dst_w && src_h * 2 <= dst_h && dst_w % src_w == 0 && dst_h % src_h == 0 {
        let (expanded, w, h) = paste_scale(src, src_w, src_h);
        return scale_to(&expanded, w, h, dst_w, dst_h);
    }
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = (x * src_w / dst_w).min(src_w - 1);
            let sy = (y * src_h / dst_h).min(src_h - 1);
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((y * dst_w + x) * 4) as usize;
            if si + 3 < src.len() && di + 3 < out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    (out, dst_w, dst_h)
}

/// Wow.exe PasteScale (solarityclient composer.rs `blend_scaled_rect`): a 2x expansion
/// where even texels copy the source and odd ones average their right/lower
/// neighbours, truncating. Larger power-of-two expansions repeat it.
fn paste_scale(src: &[u8], src_w: u32, src_h: u32) -> (Vec<u8>, u32, u32) {
    let (w, h) = (src_w as usize, src_h as usize);
    let mut out = vec![0u8; w * 2 * h * 2 * 4];
    let texel = |x: usize, y: usize, channel: usize| u16::from(src[(y * w + x) * 4 + channel]);
    for y in 0..h * 2 {
        let (sy, ny) = (y / 2, (y / 2 + 1).min(h - 1));
        for x in 0..w * 2 {
            let (sx, nx) = (x / 2, (x / 2 + 1).min(w - 1));
            for channel in 0..4 {
                let value = match (x & 1, y & 1) {
                    (0, 0) => texel(sx, sy, channel),
                    (1, 0) => (texel(sx, sy, channel) + texel(nx, sy, channel)) / 2,
                    (0, 1) => (texel(sx, sy, channel) + texel(sx, ny, channel)) / 2,
                    _ => {
                        (texel(sx, sy, channel)
                            + texel(nx, sy, channel)
                            + texel(sx, ny, channel)
                            + texel(nx, ny, channel))
                            / 4
                    }
                };
                out[(y * w * 2 + x) * 4 + channel] = value as u8;
            }
        }
    }
    (out, src_w * 2, src_h * 2)
}
