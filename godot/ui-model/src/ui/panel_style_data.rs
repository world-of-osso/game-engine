//! Authored panel skins shared by the Bevy and Godot registry hosts: the default slate
//! panel and the Retail metal window borders composed from their atlas members.

use ui_toolkit::atlas::{ActiveSkin, AtlasRegion, AtlasSource, resolve_region, thread_skin};
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
/// on a frame [`MetalTopLeft::outset`] larger than the window.
pub const METAL_FRAME_PANEL_STYLE: &str = "metal_frame";
/// `ButtonFrameTemplateNoPortrait` metal border (flat panels such as `LootFrame`).
pub const METAL_FRAME_NO_PORTRAIT_PANEL_STYLE: &str = "metal_frame_no_portrait";

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

    /// `NineSliceLayouts.PortraitFrameTemplate` / `.ButtonFrameTemplateNoPortrait` corner
    /// offsets from the window edges (display units), `[left, top, right, bottom]`
    /// outward: the styled frame sits at `(-left, -top)` and is `left + right` wider and
    /// `top + bottom` taller than the window. Forever's c60 art is larger than Retail's,
    /// so Camelot moves the right corners 2 in and the bottom corners to y = -8
    /// (`Blizzard_SharedXML/Camelot/NineSliceLayoutOverrides.lua:2-25`).
    pub fn outset(self) -> [f32; 4] {
        let left = match self {
            Self::Portrait => 13.0,
            Self::Plain => 8.0,
        };
        match thread_skin() {
            ActiveSkin::Modern => [left, 16.0, 4.0, 3.0],
            ActiveSkin::Forever => [left, 16.0, 2.0, 8.0],
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

/// The composed sheet is a 3×3 grid in the active skin's atlas pixels, each member at
/// its own size so neighbouring pieces share one scale: columns `EdgeLeft` | `EdgeTop` |
/// `EdgeRight` wide, rows `EdgeTop` | `EdgeLeft` | `EdgeBottom` tall (Retail 150|64|150 ×
/// 150|32|64, Forever c60 190|256|190 × 190|256|200). A bottom corner narrower than its
/// side column (Retail: 64 in 150) continues with bottom-edge art out to the column's
/// width — the same pixels Retail's bottom edge draws there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetalGeometry {
    pub columns: [u32; 3],
    pub rows: [u32; 3],
}

impl MetalGeometry {
    /// The geometry of the active skin's metal members.
    pub fn active() -> Result<Self, String> {
        let size = |atlas: &str| {
            let skin = thread_skin();
            resolve_region(atlas, skin)
                .map(|region| [region.width as u32, region.height as u32])
                .ok_or_else(|| format!("metal frame atlas {atlas} missing for {skin:?}"))
        };
        let [left, side_h] = size(EDGE_LEFT)?;
        let [right, _] = size(EDGE_RIGHT)?;
        let [top_w, top_h] = size(EDGE_TOP)?;
        let [_, bottom_h] = size(EDGE_BOTTOM)?;
        Ok(Self {
            columns: [left, top_w, right],
            rows: [top_h, side_h, bottom_h],
        })
    }

    pub fn sheet_size(self) -> (u32, u32) {
        (self.columns.iter().sum(), self.rows.iter().sum())
    }

    /// Display edge sizes `[left, top, right, bottom]` (half the atlas pixels).
    pub fn edge_sizes(self) -> [f32; 4] {
        [self.columns[0], self.rows[0], self.columns[2], self.rows[2]].map(|px| px as f32 / 2.0)
    }
}

const EDGE_TOP: &str = "_UI-Frame-Metal-EdgeTop";
const EDGE_BOTTOM: &str = "_UI-Frame-Metal-EdgeBottom";
const EDGE_LEFT: &str = "!UI-Frame-Metal-EdgeLeft";
const EDGE_RIGHT: &str = "!UI-Frame-Metal-EdgeRight";
const CORNER_TOP_RIGHT: &str = "UI-Frame-Metal-CornerTopRight";
const CORNER_BOTTOM_LEFT: &str = "UI-Frame-Metal-CornerBottomLeft";
const CORNER_BOTTOM_RIGHT: &str = "UI-Frame-Metal-CornerBottomRight";

// Retail and Forever AddOns/Blizzard_SharedXML/Mainline/NineSliceLayouts.lua:
// portrait TL :20, plain TL :46, TR :21, BL :22, BR :23, T/B/L/R :24-27.
fn metal_frame_blits(
    top_left: MetalTopLeft,
    geometry: MetalGeometry,
) -> Result<Vec<MetalBlit>, String> {
    let skin = thread_skin();
    let width = |atlas: &str| {
        resolve_region(atlas, skin)
            .map(|region| region.width as u32)
            .ok_or_else(|| format!("metal frame atlas {atlas} missing for {skin:?}"))
    };
    let blit = |atlas, dest| MetalBlit {
        atlas,
        dest,
        width_fraction: 1.0,
    };
    let [left, middle, right] = geometry.columns;
    let [top, side, bottom] = geometry.rows;
    let (sheet_w, _) = geometry.sheet_size();
    let bottom_y = top + side;
    let bottom_left = width(CORNER_BOTTOM_LEFT)?;
    let bottom_right = width(CORNER_BOTTOM_RIGHT)?;
    let tile = width(EDGE_BOTTOM)?;
    let mut blits = vec![
        blit(top_left.atlas(), (0, 0, left, top)),
        blit(EDGE_TOP, (left, 0, middle, top)),
        blit(CORNER_TOP_RIGHT, (left + middle, 0, right, top)),
        blit(EDGE_LEFT, (0, top, left, side)),
        blit(EDGE_RIGHT, (left + middle, top, right, side)),
        blit(CORNER_BOTTOM_LEFT, (0, bottom_y, bottom_left, bottom)),
        blit(
            CORNER_BOTTOM_RIGHT,
            (sheet_w - bottom_right, bottom_y, bottom_right, bottom),
        ),
    ];
    // Bottom edge from the end of the left corner to the start of the right corner.
    let mut x = bottom_left;
    while x < sheet_w - bottom_right {
        let span = (sheet_w - bottom_right - x).min(tile);
        blits.push(MetalBlit {
            atlas: EDGE_BOTTOM,
            dest: (x, bottom_y, span, bottom),
            width_fraction: span as f32 / tile as f32,
        });
        x += span;
    }
    Ok(blits)
}

/// Normalised `[left, right, top, bottom]` of each grid cell in TL,T,TR,L,C,R,BL,B,BR order.
pub fn metal_frame_uv_rects(geometry: MetalGeometry) -> [[f32; 4]; 9] {
    let (sheet_w, sheet_h) = geometry.sheet_size();
    let starts = |sizes: [u32; 3]| {
        [
            0,
            sizes[0],
            sizes[0] + sizes[1],
            sizes[0] + sizes[1] + sizes[2],
        ]
    };
    let (xs, ys) = (starts(geometry.columns), starts(geometry.rows));
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

/// Sheets `compose_metal_sheet` reads under the active skin, for hosts that copy textures
/// out of local CASC on demand. An unresolved member is reported by the compose itself.
pub fn metal_sheet_fdids(top_left: MetalTopLeft) -> Vec<u32> {
    let skin = thread_skin();
    let mut fdids = Vec::new();
    let blits = MetalGeometry::active().and_then(|geometry| metal_frame_blits(top_left, geometry));
    for blit in blits.into_iter().flatten() {
        if let Some(AtlasRegion {
            source: AtlasSource::FileDataId(fdid),
            ..
        }) = resolve_region(blit.atlas, skin)
            && !fdids.contains(&fdid)
        {
            fdids.push(fdid);
        }
    }
    fdids
}

/// RGBA sheet of `geometry.sheet_size()` from the blits; `source` returns
/// `(pixels, width)` of a texture.
pub fn compose_metal_sheet(
    top_left: MetalTopLeft,
    geometry: MetalGeometry,
    mut source: impl FnMut(u32) -> Result<(Vec<u8>, u32), String>,
) -> Result<Vec<u8>, String> {
    let (sheet_w, sheet_h) = geometry.sheet_size();
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    let mut loaded = std::collections::HashMap::new();
    let skin = thread_skin();
    for blit in metal_frame_blits(top_left, geometry)? {
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
        copy_metal_member(&mut sheet, sheet_w, (pixels, *width), region, blit)?;
    }
    Ok(sheet)
}

/// Copy the resolved member into its sheet cell (only a final partial bottom-edge tile
/// takes the leading part of its member).
fn copy_metal_member(
    sheet: &mut [u8],
    sheet_w: u32,
    (pixels, width): (&[u8], u32),
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
            let to = (((dy + y) * sheet_w + dx + x) * 4) as usize;
            let pixel = pixels
                .get(from..from + 4)
                .ok_or_else(|| format!("metal frame atlas {} source too small", blit.atlas))?;
            sheet[to..to + 4].copy_from_slice(pixel);
        }
    }
    Ok(())
}

/// The metal border `NineSlice` drawing the sheet `texture` composed at `geometry`.
pub fn metal_frame_style(texture: TextureSource, geometry: MetalGeometry) -> NineSlice {
    let edge_sizes = geometry.edge_sizes();
    NineSlice {
        edge_size: edge_sizes[0],
        edge_sizes: Some(edge_sizes),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        texture: Some(texture),
        uv_rects: Some(metal_frame_uv_rects(geometry)),
        ..Default::default()
    }
}
