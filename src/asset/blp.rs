use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image_blp::convert::blp_to_image;
use image_blp::parser::load_blp;
use image_blp::types::BlpContent;

pub fn load_blp_to_image(path: &Path) -> Result<Image, String> {
    let blp = load_blp(path).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let blp_img = blp_to_image(&blp, 0).map_err(|e| format!("Failed to convert BLP: {e}"))?;
    let rgba = blp_img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let mut pixels = rgba.into_raw();
    fix_1bit_alpha(&mut pixels);

    Ok(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Load a BLP as a GPU-compressed Image when possible (BC1/BC2/BC3).
/// Falls back to RGBA8 decompression for JPEG/Raw BLPs.
/// This skips CPU-side DXT decompression entirely for DXT BLPs.
pub fn load_blp_gpu_image(path: &Path) -> Result<Image, String> {
    let blp = load_blp(path).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let (w, h) = (blp.header.width, blp.header.height);
    match &blp.content {
        BlpContent::Dxt1(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc1RgbaUnormSrgb),
        BlpContent::Dxt3(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc2RgbaUnormSrgb),
        BlpContent::Dxt5(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc3RgbaUnormSrgb),
        _ => {
            // Non-DXT: fall back to CPU decode
            load_blp_to_image(path)
        }
    }
}

fn gpu_image_from_dxtn(
    dxtn: &image_blp::types::BlpDxtn,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> Result<Image, String> {
    let data = dxtn
        .images
        .first()
        .ok_or_else(|| "BLP DXT has no mipmap level 0".to_string())?;
    let (w, h) = dxtn_actual_dimensions(width, height, data.content.len(), format);
    Ok(Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data.content.clone(),
        format,
        RenderAssetUsages::default(),
    ))
}

/// Some BLPs have truncated mipmaps — header says 128×128 but mip0 data
/// only fits a smaller resolution. Infer actual dimensions from data size.
fn dxtn_actual_dimensions(w: u32, h: u32, data_len: usize, format: TextureFormat) -> (u32, u32) {
    let block_bytes = match format {
        TextureFormat::Bc1RgbaUnormSrgb => 8,
        _ => 16, // BC2, BC3
    };
    let expected = dxtn_size(w, h, block_bytes);
    if data_len >= expected { return (w, h); }
    let (mut mw, mut mh) = (w, h);
    while mw > 4 && mh > 4 {
        mw /= 2;
        mh /= 2;
        if data_len >= dxtn_size(mw, mh, block_bytes) { return (mw, mh); }
    }
    (4.max(mw), 4.max(mh))
}

fn dxtn_size(w: u32, h: u32, block_bytes: usize) -> usize {
    let bw = w.div_ceil(4) as usize;
    let bh = h.div_ceil(4) as usize;
    bw * bh * block_bytes
}

/// Load a BLP file and return raw RGBA pixels + dimensions.
pub fn load_blp_rgba(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let blp = load_blp(path).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let blp_img = blp_to_image(&blp, 0).map_err(|e| format!("Failed to convert BLP: {e}"))?;
    let rgba = blp_img.to_rgba8();
    let w = rgba.width();
    let h = rgba.height();
    let mut pixels = rgba.into_raw();
    fix_1bit_alpha(&mut pixels);
    Ok((pixels, w, h))
}

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
    for row in 0..ov_h {
        for col in 0..ov_w {
            let bx = dst_x + col;
            let by = dst_y + row;
            if bx >= base_w {
                continue;
            }
            let bi = ((by * base_w + bx) * 4) as usize;
            let oi = ((row * ov_w + col) * 4) as usize;
            if bi + 3 >= base.len() || oi + 3 >= overlay.len() {
                continue;
            }
            let alpha = overlay[oi + 3] as u16;
            if alpha == 0 {
                continue;
            }
            if alpha == 255 {
                base[bi] = overlay[oi];
                base[bi + 1] = overlay[oi + 1];
                base[bi + 2] = overlay[oi + 2];
                base[bi + 3] = 255;
            } else {
                let inv = 255 - alpha;
                base[bi] = ((alpha * overlay[oi] as u16 + inv * base[bi] as u16) / 255) as u8;
                base[bi + 1] =
                    ((alpha * overlay[oi + 1] as u16 + inv * base[bi + 1] as u16) / 255) as u8;
                base[bi + 2] =
                    ((alpha * overlay[oi + 2] as u16 + inv * base[bi + 2] as u16) / 255) as u8;
                base[bi + 3] = base[bi + 3].max(overlay[oi + 3]);
            }
        }
    }
}

fn fix_1bit_alpha(pixels: &mut [u8]) {
    let max_alpha = pixels.iter().skip(3).step_by(4).copied().max().unwrap_or(0);
    if max_alpha == 0 {
        // No alpha channel — set all pixels fully opaque.
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
    } else if max_alpha == 1 {
        // 1-bit alpha — expand 1 → 255.
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            if *alpha > 0 {
                *alpha = 255;
            }
        }
    }
}
