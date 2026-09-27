use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::asset::AssetId;
use bevy::prelude::*;

use crate::asset;

#[path = "../../asset/m2_texture_composite_data.rs"]
pub(crate) mod m2_texture_composite_data;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextureCacheKey {
    base_path: std::path::PathBuf,
    overlays: Vec<asset::m2::TextureOverlay>,
    texture_2_fdid: Option<u32>,
    shader_id: u16,
    blend_mode: u16,
}

pub(crate) static COMPOSITED_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<TextureCacheKey, Result<AssetId<Image>, String>>>,
> = OnceLock::new();

pub(crate) fn load_composited_texture(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = composited_texture_cache_key(base_path, batch);
    let cache =
        COMPOSITED_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(cached) =
        crate::asset_lifetime::lookup_cached_result_asset_handle(cache, &key, images)
    {
        return cached;
    }
    let handle = build_composited_texture_handle(base_path, batch, texture_dir, images)?;
    crate::asset_lifetime::prune_unused_result_asset_handles(cache, images);
    cache.lock().unwrap().insert(key, Ok(handle.id()));
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

    m2_texture_composite_data::composite_second_texture_pixels(
        base_pixels,
        base_width,
        base_height,
        &overlay_pixels,
        overlay_width,
        overlay_height,
        shader_id,
    );
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
        Ok((ov_pixels, ov_w, ov_h)) => m2_texture_composite_data::composite_overlay_pixels(
            pixels,
            base_width,
            &ov_pixels,
            ov_w,
            ov_h,
            ov.x,
            ov.y,
            matches!(ov.scale, OverlayScale::Uniform2x),
        ),
        Err(e) => eprintln!("Failed to load overlay {}: {e}", ov_path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::m2_texture_composite_data::apply_m2_multitexture_shader;

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
