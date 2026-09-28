//! Decode authored BLP bytes to mip 0 RGBA8 pixels.
use crate::asset::blp_format::{fix_1bit_alpha, strip_mipmaps};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};

pub struct RgbaImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn decode_rgba(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut bytes = bytes.to_vec();
    strip_mipmaps(&mut bytes);
    let blp = load_blp_from_buf(&bytes).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let rgba = blp_to_image(&blp, 0)
        .map_err(|e| format!("Failed to convert BLP: {e}"))?
        .to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let mut pixels = rgba.into_raw();
    fix_1bit_alpha(&mut pixels);
    Ok(RgbaImage {
        pixels,
        width,
        height,
    })
}
