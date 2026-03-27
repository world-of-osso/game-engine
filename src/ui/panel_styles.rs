use bevy::prelude::*;

use crate::ui::frame::{NineSlice, ThreeSlice};
use crate::ui::plugin::UiState;
use crate::ui::screens::loading_component::{
    TEX_LOADING_BAR_CENTER, TEX_LOADING_BAR_LEFT, TEX_LOADING_BAR_RIGHT,
};
use crate::ui::widgets::texture::TextureSource;

/// Register built-in panel styles on startup.
pub fn register_panel_styles(mut ui: ResMut<UiState>) {
    register_nine_slice_styles(&mut ui);
    register_three_slice_styles(&mut ui);
    // Apply to any frames created before styles were registered.
    ui.registry.refresh_panel_styles();
}

fn register_nine_slice_styles(ui: &mut UiState) {
    ui.registry.register_panel_style(
        "default",
        NineSlice {
            edge_size: 8.0,
            uv_edge_size: Some(8.0),
            bg_color: [1.0, 1.0, 1.0, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            texture: Some(TextureSource::File(
                "data/textures/ui/panel_slate_gold_512.ktx2".to_string(),
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
                "data/textures/ui/panel_slate_gold_plain_128.ktx2".to_string(),
            )),
            ..Default::default()
        },
    );
}

fn register_three_slice_styles(ui: &mut UiState) {
    ui.registry.register_three_slice_style(
        "loading_bar_shell",
        ThreeSlice {
            cap_width: 25.0,
            left: TextureSource::File(TEX_LOADING_BAR_LEFT.to_string()),
            center: TextureSource::File(TEX_LOADING_BAR_CENTER.to_string()),
            right: TextureSource::File(TEX_LOADING_BAR_RIGHT.to_string()),
            color: [1.0, 1.0, 1.0, 1.0],
        },
    );
}
