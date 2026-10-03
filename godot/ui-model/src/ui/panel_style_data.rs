//! Authored panel skins shared by the Bevy and Godot registry hosts: the default slate
//! panel and the Retail metal window borders composed from their atlas members.

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

/// `PortraitFrameTemplate` metal border (Retail `ButtonFrameTemplate` windows). Put it
/// on a frame [`METAL_FRAME_OUTSET`] larger than the window.
pub const METAL_FRAME_PANEL_STYLE: &str = "metal_frame";
/// `ButtonFrameTemplateNoPortrait` metal border (flat panels such as `LootFrame`). Put
/// it on a frame [`METAL_FRAME_NO_PORTRAIT_OUTSET`] larger than the window.
pub const METAL_FRAME_NO_PORTRAIT_PANEL_STYLE: &str = "metal_frame_no_portrait";

/// `NineSliceLayouts.PortraitFrameTemplate` corner offsets from the window edges
/// (display units): `[left, top, right, bottom]` outward. The styled frame sits at
/// `(-left, -top)` and is `left + right` wider and `top + bottom` taller than the window.
pub const METAL_FRAME_OUTSET: [f32; 4] = [13.0, 16.0, 4.0, 3.0];
/// `NineSliceLayouts.ButtonFrameTemplateNoPortrait` corner offsets, same convention.
pub const METAL_FRAME_NO_PORTRAIT_OUTSET: [f32; 4] = [8.0, 16.0, 4.0, 3.0];

/// The top-left corner that tells the two metal layouts apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetalTopLeft {
    /// `ui-frame-portraitmetal-cornertopleft-2x` (8504): `PortraitFrameTemplate`.
    Portrait,
    /// `ui-frame-metal-cornertopleft-2x` (8501): `ButtonFrameTemplateNoPortrait`.
    Plain,
}

impl MetalTopLeft {
    fn source(self) -> PixelRect {
        match self {
            Self::Portrait => (1, 153, 150, 150),
            Self::Plain => (1, 1, 150, 150),
        }
    }

    pub fn style_name(self) -> &'static str {
        match self {
            Self::Portrait => METAL_FRAME_PANEL_STYLE,
            Self::Plain => METAL_FRAME_NO_PORTRAIT_PANEL_STYLE,
        }
    }
}

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
pub const METAL_CORNERS: u32 = 2_406_979;
pub const METAL_SIDE_EDGES: u32 = 2_406_984;
pub const METAL_TOP_BOTTOM_EDGES: u32 = 2_406_987;

/// The composed sheet is a 3×3 grid in atlas pixels: columns 150 | 64 | 150 and rows
/// 150 | 32 | 64. The Retail layout's bottom corners are 32 display units wide while
/// the side edges are 75, so each bottom corner cell continues with bottom-edge art
/// out to the side column's width — the same pixels Retail's bottom edge draws there.
const METAL_COLUMNS: [u32; 3] = [150, 64, 150];
const METAL_ROWS: [u32; 3] = [150, 32, 64];
pub const METAL_SHEET: (u32, u32) = (364, 246);
/// Display edge sizes `[left, top, right, bottom]` (half the atlas pixels).
pub const METAL_EDGE_SIZES: [f32; 4] = [75.0, 75.0, 75.0, 32.0];

/// `_ui-frame-metal-edgebottom-2x` (8516): a 32×64 strip tiled along the bottom.
const EDGE_BOTTOM: PixelRect = (0, 153, 32, 64);

fn metal_frame_blits(top_left: MetalTopLeft) -> Vec<MetalBlit> {
    let blit = |fdid, source, dest| MetalBlit { fdid, source, dest };
    let bottom_y = METAL_ROWS[0] + METAL_ROWS[1];
    let mut blits = vec![
        blit(METAL_CORNERS, top_left.source(), (0, 0, 150, 150)),
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
pub fn metal_frame_uv_rects() -> [[f32; 4]; 9] {
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
pub fn compose_metal_sheet(
    top_left: MetalTopLeft,
    mut source: impl FnMut(u32) -> Result<(Vec<u8>, u32), String>,
) -> Result<Vec<u8>, String> {
    let (sheet_w, sheet_h) = METAL_SHEET;
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    let mut loaded: Vec<(u32, (Vec<u8>, u32))> = Vec::new();
    for blit in metal_frame_blits(top_left) {
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

/// The metal border `NineSlice` drawing the composed sheet `texture`.
pub fn metal_frame_style(texture: TextureSource) -> NineSlice {
    NineSlice {
        edge_size: METAL_EDGE_SIZES[0],
        edge_sizes: Some(METAL_EDGE_SIZES),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        texture: Some(texture),
        uv_rects: Some(metal_frame_uv_rects()),
        ..Default::default()
    }
}
