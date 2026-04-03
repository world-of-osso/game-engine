use super::{CharTextureData, CompositedModelTextures, TextureLayer, TextureSection};

pub(super) const FULL_TEXTURE_SECTION_MASK: i64 = -1;
pub(super) const HD_TEXTURE_WIDTH: u32 = 2048;
pub(super) const HD_TEXTURE_HEIGHT: u32 = 1024;

pub(super) struct BlitLayerInput<'a> {
    pub pixels: &'a mut [u8],
    pub canvas_w: u32,
    pub tex: &'a [u8],
    pub tex_w: u32,
    pub tex_h: u32,
    pub layer: &'a TextureLayer,
    pub layout_id: u32,
}

pub(super) struct BlitScaledInput<'a> {
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

pub(super) fn blit_layer(data: &CharTextureData, input: BlitLayerInput<'_>) {
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

pub(super) fn runtime_texture_for_section(
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

pub(super) fn runtime_textures_from_layout(
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

pub(super) fn blit_section(
    pixels: &mut [u8],
    canvas_w: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    section: &TextureSection,
    layer: &TextureLayer,
) {
    let (scaled, sw, sh) = scale_to(tex, tex_w, tex_h, section.width, section.height);
    let use_src_alpha = uses_source_alpha(layer.blend_mode);
    for row in 0..sh.min(section.height) {
        for col in 0..sw.min(section.width) {
            let si = ((row * sw + col) * 4) as usize;
            let dx = section.x + col;
            let dy = section.y + row;
            let di = ((dy * canvas_w + dx) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= scaled.len() {
                continue;
            }
            blend_pixel(pixels, di, &scaled, si, use_src_alpha);
        }
    }
}

pub(super) fn blit_scaled(input: BlitScaledInput<'_>) {
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
    let use_src_alpha = uses_source_alpha(layer.blend_mode);
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
            blend_pixel(pixels, di, tex, si, use_src_alpha);
        }
    }
}

fn uses_source_alpha(blend_mode: u32) -> bool {
    matches!(blend_mode, 1 | 15)
}

pub(super) fn blend_pixel(dst: &mut [u8], di: usize, src: &[u8], si: usize, use_src_alpha: bool) {
    let alpha = src[si + 3] as u16;
    if alpha == 0 {
        return;
    }
    if !use_src_alpha || alpha == 255 {
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

pub(super) fn scaled_section(section: TextureSection, divisor: u32) -> TextureSection {
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

pub(super) fn scale_to(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> (Vec<u8>, u32, u32) {
    if src_w == dst_w && src_h == dst_h {
        return (src.to_vec(), dst_w, dst_h);
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

#[cfg(test)]
pub(super) fn load_test_data() -> CharTextureData {
    use std::path::Path;

    crate::char_texture_cache::import_char_texture_cache(Path::new("data"))
        .expect("import char texture cache");
    CharTextureData::load(Path::new("data"))
}

#[cfg(test)]
#[path = "../../tests/unit/char_texture_tests.rs"]
mod tests;
