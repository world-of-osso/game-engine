use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::ui::frame::{NineSlice, ThreeSlice};
use crate::ui::plugin::UiState;
use crate::ui::screens::loading_component::{
    TEX_LOADING_BAR_CENTER, TEX_LOADING_BAR_LEFT, TEX_LOADING_BAR_RIGHT,
};
use crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE;
use crate::ui::widgets::texture::TextureSource;

/// `PortraitFrameTemplate` metal border (Retail `ButtonFrameTemplate` windows). Put it
/// on a frame [`METAL_FRAME_OUTSET`] larger than the window.
pub const METAL_FRAME_PANEL_STYLE: &str = "metal_frame";

/// Register built-in panel styles on startup.
pub fn register_panel_styles(mut ui: ResMut<UiState>, mut images: ResMut<Assets<Image>>) {
    register_nine_slice_styles(&mut ui);
    register_metal_frame_style(&mut ui, &mut images);
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

/// `NineSliceLayouts.PortraitFrameTemplate` corner offsets from the window edges
/// (display units): `[left, top, right, bottom]` outward. The styled frame sits at
/// `(-left, -top)` and is `left + right` wider and `top + bottom` taller than the window.
pub const METAL_FRAME_OUTSET: [f32; 4] = [13.0, 16.0, 4.0, 3.0];

/// A pixel rect `(x, y, width, height)`.
type PixelRect = (u32, u32, u32, u32);

/// One copy into the composed metal sheet: source texture, source rect, destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetalBlit {
    fdid: u32,
    source: PixelRect,
    dest: PixelRect,
}

/// `UiTextureAtlas` 1390 (corners), 1394 (`!` left/right edges), 1395 (`_` top/bottom
/// edges); all members are `-2x`, drawn at half their pixel size.
const METAL_CORNERS: u32 = 2_406_979;
const METAL_SIDE_EDGES: u32 = 2_406_984;
const METAL_TOP_BOTTOM_EDGES: u32 = 2_406_987;

/// The composed sheet is a 3×3 grid in atlas pixels: columns 150 | 64 | 150 and rows
/// 150 | 32 | 64. The Retail layout's bottom corners are 32 display units wide while
/// the side edges are 75, so each bottom corner cell continues with bottom-edge art
/// out to the side column's width — the same pixels Retail's bottom edge draws there.
const METAL_COLUMNS: [u32; 3] = [150, 64, 150];
const METAL_ROWS: [u32; 3] = [150, 32, 64];
const METAL_SHEET: (u32, u32) = (364, 246);
/// Display edge sizes `[left, top, right, bottom]` (half the atlas pixels).
const METAL_EDGE_SIZES: [f32; 4] = [75.0, 75.0, 75.0, 32.0];

/// `_ui-frame-metal-edgebottom-2x` (8516): a 32×64 strip tiled along the bottom.
const EDGE_BOTTOM: PixelRect = (0, 153, 32, 64);

fn metal_frame_blits() -> Vec<MetalBlit> {
    let blit = |fdid, source, dest| MetalBlit { fdid, source, dest };
    let bottom_y = METAL_ROWS[0] + METAL_ROWS[1];
    let mut blits = vec![
        // ui-frame-portraitmetal-cornertopleft-2x (8504)
        blit(METAL_CORNERS, (1, 153, 150, 150), (0, 0, 150, 150)),
        // _ui-frame-metal-edgetop-2x (8517)
        blit(METAL_TOP_BOTTOM_EDGES, (0, 1, 64, 150), (150, 0, 64, 150)),
        // ui-frame-metal-cornertopright-2x (8502)
        blit(METAL_CORNERS, (153, 1, 150, 150), (214, 0, 150, 150)),
        // !ui-frame-metal-edgeleft-2x (8514) / edgeright (8515)
        blit(METAL_SIDE_EDGES, (1, 0, 150, 32), (0, 150, 150, 32)),
        blit(METAL_SIDE_EDGES, (153, 0, 150, 32), (214, 150, 150, 32)),
        // ui-frame-metal-cornerbottomleft-2x (8499) / bottomright (8500)
        blit(METAL_CORNERS, (153, 153, 64, 64), (0, bottom_y, 64, 64)),
        blit(METAL_CORNERS, (219, 153, 64, 64), (300, bottom_y, 64, 64)),
    ];
    // Bottom edge from the end of the left corner to the start of the right corner.
    let mut x = 64;
    while x < 300 {
        let width = (300 - x).min(EDGE_BOTTOM.2);
        blits.push(blit(
            METAL_TOP_BOTTOM_EDGES,
            (EDGE_BOTTOM.0, EDGE_BOTTOM.1, width, EDGE_BOTTOM.3),
            (x, bottom_y, width, 64),
        ));
        x += width;
    }
    blits
}

/// Normalised `[left, right, top, bottom]` of each grid cell in TL,T,TR,L,C,R,BL,B,BR order.
fn metal_frame_uv_rects() -> [[f32; 4]; 9] {
    let (sheet_w, sheet_h) = METAL_SHEET;
    let starts = |sizes: [u32; 3]| {
        [
            0,
            sizes[0],
            sizes[0] + sizes[1],
            sizes[0] + sizes[1] + sizes[2],
        ]
    };
    let (xs, ys) = (starts(METAL_COLUMNS), starts(METAL_ROWS));
    std::array::from_fn(|part| {
        let (col, row) = (part % 3, part / 3);
        [
            xs[col] as f32 / sheet_w as f32,
            xs[col + 1] as f32 / sheet_w as f32,
            ys[row] as f32 / sheet_h as f32,
            ys[row + 1] as f32 / sheet_h as f32,
        ]
    })
}

/// RGBA sheet from the blits; `source` returns `(pixels, width)` of a texture.
fn compose_metal_sheet(
    mut source: impl FnMut(u32) -> Result<(Vec<u8>, u32), String>,
) -> Result<Vec<u8>, String> {
    let (sheet_w, sheet_h) = METAL_SHEET;
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    let mut loaded: Vec<(u32, (Vec<u8>, u32))> = Vec::new();
    for blit in metal_frame_blits() {
        if !loaded.iter().any(|(fdid, _)| *fdid == blit.fdid) {
            loaded.push((blit.fdid, source(blit.fdid)?));
        }
        let (pixels, width) = &loaded
            .iter()
            .find(|(fdid, _)| *fdid == blit.fdid)
            .unwrap()
            .1;
        let (sx, sy, w, h) = blit.source;
        let (dx, dy, _, _) = blit.dest;
        for row in 0..h {
            let from = (((sy + row) * width + sx) * 4) as usize;
            let to = (((dy + row) * sheet_w + dx) * 4) as usize;
            let len = (w * 4) as usize;
            let src = pixels
                .get(from..from + len)
                .ok_or_else(|| format!("metal frame source {} too small", blit.fdid))?;
            sheet[to..to + len].copy_from_slice(src);
        }
    }
    Ok(sheet)
}

fn load_blp_pixels(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("texture {fdid} not available"))?;
    let (pixels, width, _) = crate::asset::blp::load_blp_rgba(&path)?;
    Ok((pixels, width))
}

fn metal_frame_style(sheet: Handle<Image>) -> NineSlice {
    NineSlice {
        edge_size: METAL_EDGE_SIZES[0],
        edge_sizes: Some(METAL_EDGE_SIZES),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        texture: Some(TextureSource::Dynamic(sheet)),
        uv_rects: Some(metal_frame_uv_rects()),
        ..Default::default()
    }
}

fn register_metal_frame_style(ui: &mut UiState, images: &mut Assets<Image>) {
    let pixels = match compose_metal_sheet(load_blp_pixels) {
        Ok(pixels) => pixels,
        Err(err) => {
            error!("{METAL_FRAME_PANEL_STYLE} panel style not registered: {err}");
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
        .register_panel_style(METAL_FRAME_PANEL_STYLE, metal_frame_style(sheet));
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

#[cfg(test)]
#[path = "panel_styles_tests.rs"]
mod tests;
