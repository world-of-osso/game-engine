//! Retail `AutoCastOverlayTemplate` Shine (Blizzard_UIPanelTemplates/Mainline/
//! AutoCastTemplates.xml): the ants turn -360° every 4 s, seen only through the square ring
//! of `UI-HUD-ActionBar-PetAutoCast-Mask`.

use game_engine_core::pet_autocast_shine_data::{SHINE_PERIOD_SECS, compose_autocast_shine};
use image::{GrayImage, Luma, Rgba, RgbaImage};

/// Ants lit only in a block straddling the top centre of their square.
fn ants_lit_at_top() -> RgbaImage {
    RgbaImage::from_fn(40, 40, |x, y| {
        if (16..24).contains(&x) && y < 10 {
            Rgba([255, 220, 90, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
}

/// A 32×32 ring mask 3 px wide, as the Retail mask's border.
fn ring_mask() -> GrayImage {
    GrayImage::from_fn(32, 32, |x, y| {
        let edge = x < 3 || y < 3 || x >= 29 || y >= 29;
        Luma([if edge { 255 } else { 0 }])
    })
}

/// Alpha at a point in the 41×41 Shine rect (its own pixels, `size` per side).
fn alpha_at(shine: &RgbaImage, x: f32, y: f32) -> u8 {
    let scale = shine.width() as f32 / 41.0;
    shine.get_pixel((x * scale) as u32, (y * scale) as u32)[3]
}

#[test]
fn ants_show_only_through_the_ring_and_turn_clockwise() {
    let (ants, mask) = (ants_lit_at_top(), ring_mask());
    let start = compose_autocast_shine(&ants, &mask, 82, 0.0);
    assert_eq!((start.width(), start.height()), (82, 82));
    // The mask covers the overlay inset 4 px: 23×23 at (9, 9) of the Shine. Its top edge
    // under the lit ants shows them; the overlay centre never does.
    assert!(alpha_at(&start, 20.5, 9.5) > 200, "top of the ring dark");
    assert_eq!(alpha_at(&start, 20.5, 20.5), 0, "ants through the hole");
    assert_eq!(alpha_at(&start, 20.5, 2.0), 0, "ants outside the mask");
    assert_eq!(
        alpha_at(&start, 31.5, 20.5),
        0,
        "right of the ring lit at start"
    );

    // A quarter period later the ants have turned 90° clockwise: lit on the right edge.
    let quarter = compose_autocast_shine(&ants, &mask, 82, SHINE_PERIOD_SECS / 4.0);
    assert!(
        alpha_at(&quarter, 31.5, 20.5) > 200,
        "right of the ring dark"
    );
    assert_eq!(alpha_at(&quarter, 20.5, 9.5), 0, "top still lit");
    // A full period is the start again.
    let full = compose_autocast_shine(&ants, &mask, 82, SHINE_PERIOD_SECS);
    assert_eq!(full, start);
}
