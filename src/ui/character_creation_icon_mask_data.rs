//! Bevy-free clipping of character-creation icons by the authored portrait mask.

use image::{GrayImage, Luma, RgbaImage};

/// `TempPortraitAlphaMask`: round portrait coverage in its alpha channel.
pub const PORTRAIT_MASK_FDID: u32 = 130_924;

/// The mask's alpha channel; its colour channels are not coverage.
pub fn mask_alpha(mask: &RgbaImage) -> GrayImage {
    GrayImage::from_fn(mask.width(), mask.height(), |x, y| {
        Luma([mask.get_pixel(x, y)[3]])
    })
}

/// Multiply icon alpha by the mask resized to the icon; colours are untouched.
pub fn compose_masked_icon(mut source: RgbaImage, mask_alpha: &GrayImage) -> RgbaImage {
    let alpha = image::imageops::resize(
        mask_alpha,
        source.width(),
        source.height(),
        image::imageops::FilterType::Triangle,
    );
    for (pixel, mask) in source.pixels_mut().zip(alpha.pixels()) {
        pixel[3] = (u16::from(pixel[3]) * u16::from(mask[0]) / 255) as u8;
    }
    source
}
