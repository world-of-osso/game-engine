//! Retail Blizzard_ActionBar/Shared/ActionButtonComponentTemplate.xml:67-87:
//! centered 45px Frame with a centered 66px Flipbook, 5 columns × 6 rows, 30 frames/s.
//! UiTextureAtlasMember 32008, atlas3260, local Retail12.1.0.69933.
use crate::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::quest_art::{DynName, atlas_texture};
use ui_toolkit::{rsx, widget_def::Element};

pub const TEXTURE: u32 = 6_992_992;

/// Pixel crop in the authored 2048×1024 sheet; fractional cell widths are intentional.
pub fn frame_region(elapsed: f64) -> [f32; 4] {
    let frame = ((elapsed.max(0.0) * 30.0).floor() as u64 % 30) as u32;
    let width = 333.0 / 5.0;
    let height = 400.0 / 6.0;
    [
        845.0 + (frame % 5) as f32 * width,
        403.0 + (frame / 5) as f32 * height,
        width,
        height,
    ]
}

pub fn highlight(name: String, cell_size: f32, hidden: bool) -> Element {
    let [x, y, w, h] = frame_region(0.0);
    let art = AtlasArt {
        fdid: TEXTURE,
        atlas: (2048.0, 1024.0),
        rect: (x, x + w, y, y + h),
    };
    let size = cell_size * 66.0 / 45.0;
    let inset = (cell_size - size) / 2.0;
    let image = atlas_texture(format!("{name}Flipbook"), &art, (inset, inset, size, size));
    rsx! {
        r#frame {
            name: {DynName(name)}, width: cell_size, height: cell_size, hidden,
            draw_layer: "OVERLAY", pos_type: "absolute", pos_x: 0.0, pos_y: 0.0,
            {image}
        }
    }
}
