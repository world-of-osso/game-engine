/// Scale RGBA pixels by 2x using nearest-neighbor.
pub fn scale_2x(pixels: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let new_w = w * 2;
    let new_h = h * 2;
    let mut out = vec![0u8; (new_w * new_h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let si = ((y * w + x) * 4) as usize;
            let pixel = &pixels[si..si + 4];
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let di = (((y * 2 + dy) * new_w + x * 2 + dx) * 4) as usize;
                    out[di..di + 4].copy_from_slice(pixel);
                }
            }
        }
    }
    (out, new_w, new_h)
}

/// Blit an overlay onto a base image at (dst_x, dst_y) with alpha blending.
pub fn blit_region(
    base: &mut [u8],
    base_w: u32,
    overlay: &[u8],
    ov_w: u32,
    ov_h: u32,
    dst_x: u32,
    dst_y: u32,
) {
    let geometry = BlitGeometry {
        base_w,
        ov_w,
        dst_x,
        dst_y,
    };
    for row in 0..ov_h {
        for col in 0..ov_w {
            let Some((bi, oi)) = geometry.pixel_indices(base, overlay, row, col) else {
                continue;
            };
            blend_overlay_pixel(base, overlay, bi, oi);
        }
    }
}

struct BlitGeometry {
    base_w: u32,
    ov_w: u32,
    dst_x: u32,
    dst_y: u32,
}

impl BlitGeometry {
    fn pixel_indices(
        &self,
        base: &[u8],
        overlay: &[u8],
        row: u32,
        col: u32,
    ) -> Option<(usize, usize)> {
        let bx = self.dst_x + col;
        if bx >= self.base_w {
            return None;
        }
        let by = self.dst_y + row;
        let bi = ((by * self.base_w + bx) * 4) as usize;
        let oi = ((row * self.ov_w + col) * 4) as usize;
        ((bi + 3) < base.len() && (oi + 3) < overlay.len()).then_some((bi, oi))
    }
}

fn blend_overlay_pixel(base: &mut [u8], overlay: &[u8], bi: usize, oi: usize) {
    let alpha = overlay[oi + 3];
    match alpha {
        0 => {}
        255 => copy_opaque_overlay_pixel(base, overlay, bi, oi),
        _ => blend_translucent_overlay_pixel(base, overlay, bi, oi, alpha as u16),
    }
}

fn copy_opaque_overlay_pixel(base: &mut [u8], overlay: &[u8], bi: usize, oi: usize) {
    base[bi] = overlay[oi];
    base[bi + 1] = overlay[oi + 1];
    base[bi + 2] = overlay[oi + 2];
    base[bi + 3] = 255;
}

fn blend_translucent_overlay_pixel(
    base: &mut [u8],
    overlay: &[u8],
    bi: usize,
    oi: usize,
    alpha: u16,
) {
    let inv = 255 - alpha;
    base[bi] = blend_channel(alpha, inv, overlay[oi], base[bi]);
    base[bi + 1] = blend_channel(alpha, inv, overlay[oi + 1], base[bi + 1]);
    base[bi + 2] = blend_channel(alpha, inv, overlay[oi + 2], base[bi + 2]);
    base[bi + 3] = base[bi + 3].max(overlay[oi + 3]);
}

fn blend_channel(alpha: u16, inv: u16, overlay: u8, base: u8) -> u8 {
    ((alpha * overlay as u16 + inv * base as u16) / 255) as u8
}
