use bevy::prelude::*;

use crate::ui::frame::NineSlice;
use crate::ui::plugin::UiState;
use crate::ui::widgets::texture::TextureSource;

/// Register built-in panel styles on startup.
pub fn register_panel_styles(mut ui: ResMut<UiState>) {
    ui.registry.register_panel_style(
        "default",
        NineSlice {
            edge_size: 8.0,
            uv_edge_size: Some(8.0),
            bg_color: [1.0, 1.0, 1.0, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            texture: Some(TextureSource::File(
                "data/textures/ui/panel_slate_gold_512.png".to_string(),
            )),
            ..Default::default()
        },
    );
    ui.registry.register_panel_style(
        "inner_plain",
        NineSlice {
            edge_size: 8.0,
            uv_edge_size: Some(8.0),
            bg_color: [1.0, 1.0, 1.0, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            texture: Some(TextureSource::File(
                "data/textures/ui/panel_slate_gold_plain_128.png".to_string(),
            )),
            ..Default::default()
        },
    );
    // Apply to any Panel frames created before styles were registered.
    ui.registry.refresh_panel_styles();
}
