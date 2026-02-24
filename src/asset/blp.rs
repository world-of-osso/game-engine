use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image_blp::convert::blp_to_image;
use image_blp::parser::load_blp;

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

fn fix_1bit_alpha(pixels: &mut [u8]) {
    let max_alpha = pixels.iter().skip(3).step_by(4).copied().max().unwrap_or(0);
    if max_alpha > 1 {
        return;
    }
    for alpha in pixels.iter_mut().skip(3).step_by(4) {
        if *alpha > 0 {
            *alpha = 255;
        }
    }
}
