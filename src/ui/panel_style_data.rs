//! Authored default panel skin shared by the Bevy and Godot registry hosts.

use ui_toolkit::frame::NineSlice;
use ui_toolkit::widgets::texture::TextureSource;

pub fn default_panel_style() -> NineSlice {
    NineSlice {
        edge_size: 8.0,
        uv_edge_size: Some(8.0),
        bg_color: [1.0; 4],
        border_color: [1.0; 4],
        texture: Some(TextureSource::File(
            "data/textures/ui/panel_slate_gold_512.ktx2".to_owned(),
        )),
        ..Default::default()
    }
}
