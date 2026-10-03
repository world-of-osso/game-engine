//! Authored panel skins shared by the Bevy and Godot registry hosts: the default slate
//! panel and the Retail metal window borders composed from their atlas members.

use ui_toolkit::atlas::{AtlasRegion, AtlasSource, active_skin, resolve_region};
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
    fn atlas(self) -> &'static str {
        match self {
            Self::Portrait => "UI-Frame-PortraitMetal-CornerTopLeft",
            Self::Plain => "UI-Frame-Metal-CornerTopLeft",
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
#[derive(Clone, Copy, Debug)]
struct MetalBlit {
    atlas: &'static str,
    dest: PixelRect,
    /// The final bottom-edge tile uses only the leading fraction of its member.
    width_fraction: f32,
}

/// The composed sheet is a 3×3 grid in atlas pixels: columns 150 | 64 | 150 and rows
/// 150 | 32 | 64. The Retail layout's bottom corners are 32 display units wide while
/// the side edges are 75, so each bottom corner cell continues with bottom-edge art
/// out to the side column's width — the same pixels Retail's bottom edge draws there.
const METAL_COLUMNS: [u32; 3] = [150, 64, 150];
const METAL_ROWS: [u32; 3] = [150, 32, 64];
pub const METAL_SHEET: (u32, u32) = (364, 246);
/// Display edge sizes `[left, top, right, bottom]` (half the atlas pixels).
pub const METAL_EDGE_SIZES: [f32; 4] = [75.0, 75.0, 75.0, 32.0];

/// Keep the existing 32-pixel bottom-edge tiling and composed-sheet geometry.
const BOTTOM_TILE_WIDTH: u32 = 32;

// Retail and Forever AddOns/Blizzard_SharedXML/Mainline/NineSliceLayouts.lua:
// portrait TL :20, plain TL :46, TR :21, BL :22, BR :23, T/B/L/R :24-27.
fn metal_frame_blits(top_left: MetalTopLeft) -> Vec<MetalBlit> {
    let blit = |atlas, dest| MetalBlit {
        atlas,
        dest,
        width_fraction: 1.0,
    };
    let bottom_y = METAL_ROWS[0] + METAL_ROWS[1];
    let mut blits = vec![
        blit(top_left.atlas(), (0, 0, 150, 150)),
        blit("_UI-Frame-Metal-EdgeTop", (150, 0, 64, 150)),
        blit("UI-Frame-Metal-CornerTopRight", (214, 0, 150, 150)),
        blit("!UI-Frame-Metal-EdgeLeft", (0, 150, 150, 32)),
        blit("!UI-Frame-Metal-EdgeRight", (214, 150, 150, 32)),
        blit("UI-Frame-Metal-CornerBottomLeft", (0, bottom_y, 64, 64)),
        blit("UI-Frame-Metal-CornerBottomRight", (300, bottom_y, 64, 64)),
    ];
    // Bottom edge from the end of the left corner to the start of the right corner.
    let mut x = 64;
    while x < 300 {
        let width = (300 - x).min(BOTTOM_TILE_WIDTH);
        blits.push(MetalBlit {
            atlas: "_UI-Frame-Metal-EdgeBottom",
            dest: (x, bottom_y, width, 64),
            width_fraction: width as f32 / BOTTOM_TILE_WIDTH as f32,
        });
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
    let mut loaded = std::collections::HashMap::new();
    let skin = active_skin();
    for blit in metal_frame_blits(top_left) {
        let region = resolve_region(blit.atlas, skin)
            .ok_or_else(|| format!("metal frame atlas {} missing for {skin:?}", blit.atlas))?;
        let AtlasSource::FileDataId(fdid) = region.source else {
            return Err(format!(
                "metal frame atlas {} is not a BLP sheet",
                blit.atlas
            ));
        };
        let (pixels, width) = match loaded.entry(fdid) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => entry.insert(source(fdid)?),
        };
        copy_metal_member(&mut sheet, pixels, *width, region, blit)?;
    }
    Ok(sheet)
}

/// Sample the resolved member into the unchanged sheet cell. Identical source/destination
/// sizes copy identical pixels; differently sized skin art uses the same fixed window geometry.
fn copy_metal_member(
    sheet: &mut [u8],
    pixels: &[u8],
    width: u32,
    region: AtlasRegion,
    blit: MetalBlit,
) -> Result<(), String> {
    let height = pixels.len() as u32 / (width * 4);
    let rect = region.rect_pixels(width, height);
    let [sx, sy] = rect.min.map(|value| value.round() as u32);
    let sw = ((rect.max[0] - rect.min[0]) * blit.width_fraction).round() as u32;
    let sh = (rect.max[1] - rect.min[1]).round() as u32;
    let (dx, dy, dw, dh) = blit.dest;
    for y in 0..dh {
        for x in 0..dw {
            let source_x = sx + x * sw / dw;
            let source_y = sy + y * sh / dh;
            let from = ((source_y * width + source_x) * 4) as usize;
            let to = (((dy + y) * METAL_SHEET.0 + dx + x) * 4) as usize;
            let pixel = pixels
                .get(from..from + 4)
                .ok_or_else(|| format!("metal frame atlas {} source too small", blit.atlas))?;
            sheet[to..to + 4].copy_from_slice(pixel);
        }
    }
    Ok(())
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
