//! The pet action button autocast Shine (Retail `AutoCastOverlayTemplate`,
//! Blizzard_UIPanelTemplates/Mainline/AutoCastTemplates.xml): the
//! `UI-HUD-ActionBar-PetAutoCast-Ants` texture, TOPLEFT (-5, 5) / BOTTOMRIGHT (5, -5) of the
//! overlay, turning `degrees="-360"` over `duration="4"` (clockwise), seen through the
//! `UI-HUD-ActionBar-PetAutoCast-Mask` inset 4 px into the overlay. The toolkit has no mask
//! textures, so each frame is composed here.

use image::{GrayImage, Rgba, RgbaImage};

/// The Shine's rotation `Anim` duration.
pub const SHINE_PERIOD_SECS: f32 = 4.0;
/// The Shine rect side: the 31 px small-button overlay (`SmallActionButtonMixin_OnLoad`)
/// plus 5 px on each side.
pub const SHINE_SIZE: f32 = 41.0;
/// The mask rect inside the Shine: the overlay inset 4 px, itself 5 px into the Shine.
const MASK_OFFSET: f32 = 9.0;
const MASK_SIZE: f32 = 23.0;

/// The Shine `seconds` into its animation as a `size`² RGBA8 image: `ants` turned clockwise
/// by `seconds / SHINE_PERIOD_SECS` of a turn about the Shine's centre, its alpha times the
/// mask alpha (`mask_alpha` stretched over the mask rect, zero outside it).
pub fn compose_autocast_shine(
    ants: &RgbaImage,
    mask_alpha: &GrayImage,
    size: u32,
    seconds: f32,
) -> RgbaImage {
    let turn = (seconds % SHINE_PERIOD_SECS) / SHINE_PERIOD_SECS * std::f32::consts::TAU;
    let (sin, cos) = turn.sin_cos();
    let scale = SHINE_SIZE / size as f32;
    let centre = SHINE_SIZE / 2.0;
    RgbaImage::from_fn(size, size, |x, y| {
        // Shine coordinates of this pixel's centre (y down).
        let (px, py) = ((x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale);
        let mask = sample_mask(mask_alpha, px, py);
        if mask == 0.0 {
            return Rgba([0, 0, 0, 0]);
        }
        // Un-rotate: the clockwise turn on screen maps (dx, dy) to
        // (dx cos - dy sin, dx sin + dy cos), so sample at its inverse.
        let (dx, dy) = (px - centre, py - centre);
        let (sx, sy) = (dx * cos + dy * sin + centre, -dx * sin + dy * cos + centre);
        let Rgba([r, g, b, a]) = sample_bilinear(ants, sx / SHINE_SIZE, sy / SHINE_SIZE);
        Rgba([r, g, b, (f32::from(a) * mask).round() as u8])
    })
}

/// Mask alpha (0-1) at Shine point (`x`, `y`): nearest texel of the mask rect.
fn sample_mask(mask: &GrayImage, x: f32, y: f32) -> f32 {
    let (u, v) = ((x - MASK_OFFSET) / MASK_SIZE, (y - MASK_OFFSET) / MASK_SIZE);
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return 0.0;
    }
    let texel = mask.get_pixel(
        (u * mask.width() as f32) as u32,
        (v * mask.height() as f32) as u32,
    );
    f32::from(texel[0]) / 255.0
}

/// Bilinear sample at normalized (`u`, `v`); transparent outside the image.
fn sample_bilinear(image: &RgbaImage, u: f32, v: f32) -> Rgba<u8> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    let (x, y) = (u * w - 0.5, v * h - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let texel = |tx: f32, ty: f32| -> [f32; 4] {
        if tx < 0.0 || ty < 0.0 || tx >= w || ty >= h {
            return [0.0; 4];
        }
        image.get_pixel(tx as u32, ty as u32).0.map(f32::from)
    };
    let corners = [
        (texel(x0, y0), (1.0 - fx) * (1.0 - fy)),
        (texel(x0 + 1.0, y0), fx * (1.0 - fy)),
        (texel(x0, y0 + 1.0), (1.0 - fx) * fy),
        (texel(x0 + 1.0, y0 + 1.0), fx * fy),
    ];
    let mut out = [0.0f32; 4];
    for (texel, weight) in corners {
        for (channel, value) in out.iter_mut().zip(texel) {
            *channel += value * weight;
        }
    }
    Rgba(out.map(|channel| channel.round().clamp(0.0, 255.0) as u8))
}
