#[path = "panel_style_data.rs"]
mod data;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::ui::frame::NineSlice;
use crate::ui::plugin::UiState;
use crate::ui::screens::loading_component::loading_bar_shell;
use crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE;
use crate::ui::widgets::texture::TextureSource;

pub use data::{
    METAL_FRAME_NO_PORTRAIT_OUTSET, METAL_FRAME_NO_PORTRAIT_PANEL_STYLE, METAL_FRAME_OUTSET,
    METAL_FRAME_PANEL_STYLE,
};
use data::{METAL_SHEET, MetalTopLeft, compose_metal_sheet};

/// Register built-in panel styles on startup.
pub fn register_panel_styles(mut ui: ResMut<UiState>, mut images: ResMut<Assets<Image>>) {
    register_nine_slice_styles(&mut ui);
    register_metal_frame_style(&mut ui, &mut images, MetalTopLeft::Portrait);
    register_metal_frame_style(&mut ui, &mut images, MetalTopLeft::Plain);
    register_three_slice_styles(&mut ui);
    // Apply to any frames created before styles were registered.
    ui.registry.refresh_panel_styles();
}

fn register_nine_slice_styles(ui: &mut UiState) {
    ui.registry
        .register_panel_style("default", data::default_panel_style());
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

fn load_blp_pixels(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("texture {fdid} not available"))?;
    let (pixels, width, _) = crate::asset::blp::load_blp_rgba(&path)?;
    Ok((pixels, width))
}

fn metal_frame_style(sheet: Handle<Image>) -> NineSlice {
    data::metal_frame_style(TextureSource::Dynamic(sheet))
}

fn register_metal_frame_style(
    ui: &mut UiState,
    images: &mut Assets<Image>,
    top_left: MetalTopLeft,
) {
    let name = top_left.style_name();
    let pixels = match compose_metal_sheet(top_left, load_blp_pixels) {
        Ok(pixels) => pixels,
        Err(err) => {
            error!("{name} panel style not registered: {err}");
            return;
        }
    };
    let (width, height) = METAL_SHEET;
    let sheet = images.add(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    ui.registry
        .register_panel_style(name, metal_frame_style(sheet));
}

fn register_three_slice_styles(ui: &mut UiState) {
    ui.registry
        .register_three_slice_style("loading_bar_shell", loading_bar_shell());
}

#[cfg(test)]
#[path = "panel_styles_tests.rs"]
mod tests;
