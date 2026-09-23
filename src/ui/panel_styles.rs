use bevy::prelude::*;

use crate::ui::frame::{NineSlice, ThreeSlice};
use crate::ui::plugin::UiState;
use crate::ui::screens::loading_component::{
    TEX_LOADING_BAR_CENTER, TEX_LOADING_BAR_LEFT, TEX_LOADING_BAR_RIGHT,
};
use crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE;
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
    ui.registry
        .register_panel_style(STATIC_POPUP_PANEL_STYLE, static_popup_border());
}

/// `Interface/DialogFrame/UIFrameDiamondMetalBorder`: atlas `UI-DiamondDialogBox-Border`
/// occupies texels 1..71 of the 128px sheet; corners are 16 texels.
fn static_popup_border() -> NineSlice {
    const COLUMNS: [f32; 4] = [1.0 / 128.0, 17.0 / 128.0, 55.0 / 128.0, 71.0 / 128.0];
    let mut uv_rects = [[0.0; 4]; 9];
    for (part, rect) in uv_rects.iter_mut().enumerate() {
        let (col, row) = (part % 3, part / 3);
        *rect = [
            COLUMNS[col],
            COLUMNS[col + 1],
            COLUMNS[row],
            COLUMNS[row + 1],
        ];
    }
    NineSlice {
        edge_size: 16.0,
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        texture: Some(TextureSource::FileDataId(6_795_680)),
        uv_rects: Some(uv_rects),
        ..Default::default()
    }
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
