use bevy::prelude::*;

use crate::ui::frame::NineSlice;
use crate::ui::plugin::UiState;
use crate::ui::widgets::texture::TextureSource;

/// Register built-in panel styles on startup.
pub fn register_panel_styles(mut ui: ResMut<UiState>) {
    ui.registry.register_panel_style(
        "default",
        NineSlice {
            edge_size: 12.0,
            uv_edge_size: Some(10.0),
            bg_color: [1.0, 1.0, 1.0, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            texture: Some(TextureSource::File(
                "data/textures/ui/panel_slate_gold_128.png".to_string(),
            )),
            ..Default::default()
        },
    );
}
