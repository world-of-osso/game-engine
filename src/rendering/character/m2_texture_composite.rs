use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::prelude::*;

use crate::asset;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextureCacheKey {
    base_path: std::path::PathBuf,
    overlays: Vec<asset::m2::TextureOverlay>,
    texture_2_fdid: Option<u32>,
    shader_id: u16,
    blend_mode: u16,
}

pub(crate) static COMPOSITED_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<TextureCacheKey, Result<Handle<Image>, String>>>,
> = OnceLock::new();

const M2_SHADER_ALPHA_MASK: u16 = 0x8000;

pub(crate) fn load_composited_texture(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = composited_texture_cache_key(base_path, batch);
    let cache =
        COMPOSITED_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }
    let handle = build_composited_texture_handle(base_path, batch, texture_dir, images)?;
    cache.lock().unwrap().insert(key, Ok(handle.clone()));
    Ok(handle)
}

fn composited_texture_cache_key(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
) -> TextureCacheKey {
    TextureCacheKey {
        base_path: base_path.to_path_buf(),
        overlays: batch.overlays.clone(),
        texture_2_fdid: batch.texture_2_fdid,
        shader_id: batch.shader_id,
        blend_mode: batch.blend_mode,
    }
}

fn build_composited_texture_handle(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let (mut pixels, w, h) = asset::blp::load_blp_rgba(base_path)
        .map_err(|e| format!("Failed to load BLP {}: {e}", base_path.display()))?;
    if let Some(texture_2_fdid) = batch.texture_2_fdid
        && !batch.use_env_map_2
    {
        composite_second_texture(
            &mut pixels,
            w,
            h,
            texture_2_fdid,
            batch.shader_id,
            texture_dir,
        );
    }
    for ov in &batch.overlays {
        composite_overlay(&mut pixels, w, ov, texture_dir);
    }
    let mut image = crate::rgba_image(pixels, w, h);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    Ok(images.add(image))
}

fn composite_second_texture(
    base_pixels: &mut [u8],
    base_width: u32,
    base_height: u32,
    overlay_fdid: u32,
    shader_id: u16,
    texture_dir: &Path,
) {
    let overlay_path = asset::asset_cache::texture(overlay_fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{overlay_fdid}.blp")));
    let Ok((overlay_pixels, overlay_width, overlay_height)) =
        asset::blp::load_blp_rgba(&overlay_path)
    else {
        eprintln!(
            "Failed to load secondary texture {}",
            overlay_path.display()
        );
        return;
    };

    for y in 0..base_height {
        for x in 0..base_width {
            let base_idx = ((y * base_width + x) * 4) as usize;
            let ox = x.rem_euclid(overlay_width);
            let oy = y.rem_euclid(overlay_height);
            let overlay_idx = ((oy * overlay_width + ox) * 4) as usize;
            let base = &mut base_pixels[base_idx..base_idx + 4];
            let overlay = &overlay_pixels[overlay_idx..overlay_idx + 4];
            apply_m2_multitexture_shader(base, overlay, shader_id);
        }
    }
}

fn apply_m2_multitexture_shader(base: &mut [u8], overlay: &[u8], shader_id: u16) {
    let base_rgb = [
        base[0] as f32 / 255.0,
        base[1] as f32 / 255.0,
        base[2] as f32 / 255.0,
    ];
    let base_a = base[3] as f32 / 255.0;
    let overlay_rgb = [
        overlay[0] as f32 / 255.0,
        overlay[1] as f32 / 255.0,
        overlay[2] as f32 / 255.0,
    ];
    let overlay_a = overlay[3] as f32 / 255.0;

    let (rgb, a) = shader_blend(base_rgb, base_a, overlay_rgb, overlay_a, shader_id);

    base[0] = (rgb[0] * 255.0).round() as u8;
    base[1] = (rgb[1] * 255.0).round() as u8;
    base[2] = (rgb[2] * 255.0).round() as u8;
    base[3] = (a * 255.0).round() as u8;
}

fn shader_blend(
    base_rgb: [f32; 3],
    base_a: f32,
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    shader_id: u16,
) -> ([f32; 3], f32) {
    match shader_id {
        M2_SHADER_ALPHA_MASK => (base_rgb, (base_a * overlay_a).clamp(0.0, 1.0)),
        0x4014 => (
            mul_2x_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a * 2.0).clamp(0.0, 1.0),
        ),
        0x0010 => (mul_rgb(base_rgb, overlay_rgb), base_a),
        0x0011 => (
            mul_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a).clamp(0.0, 1.0),
        ),
        0x4016 => (mul_2x_rgb(base_rgb, overlay_rgb), base_a),
        0x8015 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8001 => (shader_8001_rgb(base_rgb, base_a, overlay_rgb), 1.0),
        0x8002 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8003 => (
            add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, base_a),
            1.0,
        ),
        _ => (base_rgb, base_a),
    }
}

fn mul_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0]).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1]).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2]).clamp(0.0, 1.0),
    ]
}

fn mul_2x_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0] * 2.0).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1] * 2.0).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2] * 2.0).clamp(0.0, 1.0),
    ]
}

fn add_overlay_rgb(
    base_rgb: [f32; 3],
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    weight: f32,
) -> [f32; 3] {
    [
        (base_rgb[0] + overlay_rgb[0] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[1] + overlay_rgb[1] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[2] + overlay_rgb[2] * overlay_a * weight).clamp(0.0, 1.0),
    ]
}

fn shader_8001_rgb(base_rgb: [f32; 3], base_a: f32, overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * ((overlay_rgb[0] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[1] * ((overlay_rgb[1] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[2] * ((overlay_rgb[2] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
    ]
}

fn composite_overlay(
    pixels: &mut [u8],
    base_width: u32,
    ov: &asset::m2::TextureOverlay,
    texture_dir: &Path,
) {
    use asset::m2::OverlayScale;
    let ov_path = asset::asset_cache::texture(ov.fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{}.blp", ov.fdid)));
    match asset::blp::load_blp_rgba(&ov_path) {
        Ok((ov_pixels, ov_w, ov_h)) => match ov.scale {
            OverlayScale::None => {
                asset::blp::blit_region(pixels, base_width, &ov_pixels, ov_w, ov_h, ov.x, ov.y);
            }
            OverlayScale::Uniform2x => {
                let (scaled, sw, sh) = asset::blp::scale_2x(&ov_pixels, ov_w, ov_h);
                asset::blp::blit_region(pixels, base_width, &scaled, sw, sh, ov.x, ov.y);
            }
        },
        Err(e) => eprintln!("Failed to load overlay {}: {e}", ov_path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::apply_m2_multitexture_shader;

    #[test]
    fn shader_8015_uses_secondary_alpha_as_additive_mask() {
        let mut base = [128, 64, 32, 51];
        let overlay = [255, 128, 0, 128];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x8015);

        assert_eq!(base, [255, 128, 32, 255]);
    }

    #[test]
    fn shader_0011_modulates_rgb_and_alpha() {
        let mut base = [255, 255, 255, 255];
        let overlay = [128, 64, 32, 64];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x0011);

        assert_eq!(base, [128, 64, 32, 64]);
    }

    #[test]
    fn shader_4016_modulates_rgb_2x_and_keeps_base_alpha() {
        let mut base = [128, 128, 128, 51];
        let overlay = [128, 255, 64, 13];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x4016);

        assert_eq!(base, [129, 255, 64, 51]);
    }

    #[test]
    fn shader_8000_uses_secondary_alpha_as_mask_only() {
        let mut base = [128, 64, 32, 128];
        let overlay = [0, 255, 255, 64];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x8000);

        assert_eq!(base, [128, 64, 32, 32]);
    }
}
