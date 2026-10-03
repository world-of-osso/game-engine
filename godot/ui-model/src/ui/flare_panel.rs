//! FlareUI 1.3's bronze panel, the Forever skin's look for the chat frame, the damage
//! meter and tooltips: a `Backdrop` of `Interface\DialogFrame\UI-DialogBox-Background-Dark`
//! inside a 16-px `Interface\Tooltips\UI-Tooltip-Border` edge tinted `ns.BORDER_COLOR`,
//! insets 3 (Chat.lua:1583-1594, DamageMeter.lua:218-223, Core.lua:8,139-146,244-249).
//! Only Blizzard's two files are drawn; FlareUI's own media is not licensed for reuse.
//!
//! It is the named panel style [`FLARE_BRONZE_PANEL_STYLE`]: the host composes
//! [`compose_flare_bronze_sheet`] into a texture and registers [`flare_bronze_style`].

use shared::faction_reaction::Reaction;
use ui_toolkit::frame::NineSlice;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::TextureSource;

use crate::damage_meter_data::class_color;
use crate::merchant_data::quality_color;
use crate::tooltip_presentation::{TooltipBorder, parse_rgba, rgba_string};

pub const FLARE_BRONZE_PANEL_STYLE: &str = "flare_bronze";
/// `Interface\Tooltips\UI-Tooltip-Border`: eight 16-px cells, left, right, top and bottom
/// edges, then the four corners.
pub const BORDER_FDID: u32 = 137_057;
/// `Interface\DialogFrame\UI-DialogBox-Background-Dark`.
pub const BACKGROUND_FDID: u32 = 312_922;
/// `ns.BORDER_COLOR` #A67D45 (Core.lua:8).
pub const FLARE_BORDER_COLOR: [f32; 4] = [0.65, 0.49, 0.27, 1.0];
/// `chat.opacity` and `damagemeter.opacity` (Core.lua:142,246): `SetBackdropColor(1, 1, 1, 0.6)`.
pub const FLARE_PANEL_OPACITY: f32 = 0.6;
/// `edgeSize` 16 and `insets` 3 (Core.lua:144-145; DamageMeter.lua:222).
const EDGE: f32 = 16.0;
const INSET: f32 = 3.0;
/// Each backdrop piece draws its cell's inner 14×14 (Backdrop.lua:144-154).
const CELL: u32 = 14;
/// The composed sheet: a 3×3 grid of those cells.
pub const FLARE_SHEET: (u32, u32) = (3 * CELL, 3 * CELL);

/// Where a sheet cell comes from.
#[derive(Clone, Copy)]
enum Piece {
    /// `UI-Tooltip-Border` cell as drawn.
    Border(u32),
    /// `UI-Tooltip-Border` cell turned a quarter clockwise: the top and bottom edges lie on
    /// their side in the file (`TopEdge`/`BottomEdge` coords, Backdrop.lua:151-152).
    Turned(u32),
    Background,
}

/// TL, T, TR, L, C, R, BL, B, BR.
const PIECES: [Piece; 9] = [
    Piece::Border(4),
    Piece::Turned(2),
    Piece::Border(5),
    Piece::Border(0),
    Piece::Background,
    Piece::Border(1),
    Piece::Border(6),
    Piece::Turned(3),
    Piece::Border(7),
];

/// A decoded texture: `(pixels, width)`.
type Rgba = (Vec<u8>, u32);

/// RGBA sheet of the nine pieces; `source` returns `(pixels, width)` of a texture.
pub fn compose_flare_bronze_sheet(
    mut source: impl FnMut(u32) -> Result<Rgba, String>,
) -> Result<Vec<u8>, String> {
    let border = source(BORDER_FDID)?;
    let background = source(BACKGROUND_FDID)?;
    let (sheet_w, _) = FLARE_SHEET;
    let mut sheet = vec![0u8; (sheet_w * sheet_w * 4) as usize];
    for (part, piece) in PIECES.into_iter().enumerate() {
        let (cx, cy) = ((part as u32 % 3) * CELL, (part as u32 / 3) * CELL);
        for (x, y) in (0..CELL).flat_map(|y| (0..CELL).map(move |x| (x, y))) {
            let rgba = match piece {
                Piece::Border(cell) => pixel(&border, BORDER_FDID, cell * 16 + 1 + x, 1 + y)?,
                Piece::Turned(cell) => pixel(&border, BORDER_FDID, cell * 16 + 1 + y, CELL - x)?,
                Piece::Background => pixel(&background, BACKGROUND_FDID, x, y)?,
            };
            let at = (((cy + y) * sheet_w + cx + x) * 4) as usize;
            sheet[at..at + 4].copy_from_slice(&rgba);
        }
    }
    Ok(sheet)
}

fn pixel((pixels, width): &Rgba, fdid: u32, x: u32, y: u32) -> Result<[u8; 4], String> {
    let at = ((y * width + x) * 4) as usize;
    pixels
        .get(at..at + 4)
        .filter(|_| x < *width)
        .map(|rgba| [rgba[0], rgba[1], rgba[2], rgba[3]])
        .ok_or_else(|| format!("flare_bronze source {fdid} has no pixel ({x}, {y})"))
}

/// Normalised `[left, right, top, bottom]` of each sheet cell in TL..BR order.
fn uv_rects() -> [[f32; 4]; 9] {
    std::array::from_fn(|part| {
        let (col, row) = ((part % 3) as f32, (part / 3) as f32);
        [col / 3.0, (col + 1.0) / 3.0, row / 3.0, (row + 1.0) / 3.0]
    })
}

/// The bronze backdrop drawing the composed sheet `texture`.
pub fn flare_bronze_style(texture: TextureSource) -> NineSlice {
    NineSlice {
        edge_size: EDGE,
        bg_color: [1.0, 1.0, 1.0, FLARE_PANEL_OPACITY],
        border_color: FLARE_BORDER_COLOR,
        texture: Some(texture),
        uv_rects: Some(uv_rects()),
        center_inset: Some(INSET),
        ..Default::default()
    }
}

/// A `flare_bronze` skin frame `name` at `(x, y, width, height)` in its parent.
pub fn flare_panel(name: &str, (x, y, width, height): (f32, f32, f32, f32)) -> Element {
    rsx! {
        r#frame {
            name: {DynName(name.to_owned())},
            width,
            height,
            style: FLARE_BRONZE_PANEL_STYLE,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `tooltips.background` (Core.lua:333): `NineSlice:SetCenterColor` (Tooltips.lua:142-143).
pub const TOOLTIP_BACKGROUND: [f32; 4] = [0.05, 0.05, 0.06, 0.9];

/// A tooltip's `flare_bronze` panel in its [`tooltip_border_rgb`] colour.
pub fn flare_tooltip_panel(
    name: &str,
    (width, height): (f32, f32),
    border: TooltipBorder,
) -> Element {
    let [r, g, b] = tooltip_border_rgb(border);
    let border_color = rgba_string([r, g, b, 1.0]);
    let bg_color = rgba_string(TOOLTIP_BACKGROUND);
    rsx! {
        r#frame {
            name: {DynName(name.to_owned())},
            width,
            height,
            style: FLARE_BRONZE_PANEL_STYLE,
            nine_slice_border_color: {border_color.as_str()},
            nine_slice_bg_color: {bg_color.as_str()},
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

/// `DEFAULT_BORDER_COLOR` #CC9957 (Tooltips.lua:47).
pub const TOOLTIP_DEFAULT_BORDER: [f32; 3] = [0.80, 0.60, 0.34];
/// Class and quality colours are toned down by this (`MUTE`, Tooltips.lua:48) for items.
const MUTE: f32 = 0.85;

/// FlareUI's tooltip border colour (`GetUnitBorderColor`, `OnTooltipSetItem`;
/// Tooltips.lua:117-131,166-181, defaults `borderByClass`/`Reaction`/`Quality` on):
/// players in their class colour, other units in the muted reaction palette, items in
/// their quality colour times [`MUTE`], everything else #CC9957.
pub fn tooltip_border_rgb(border: TooltipBorder) -> [f32; 3] {
    match border {
        TooltipBorder::Default => TOOLTIP_DEFAULT_BORDER,
        TooltipBorder::Class(class_id) => class_color(class_id),
        // `REACTION_COLORS` (Tooltips.lua:37-46): hostile 1-2, neutral 4, friendly 5-8.
        TooltipBorder::Reaction(Reaction::Hostile) => [0.78, 0.28, 0.24],
        TooltipBorder::Reaction(Reaction::Neutral) => [0.80, 0.68, 0.30],
        TooltipBorder::Reaction(Reaction::Friendly) => [0.35, 0.65, 0.38],
        TooltipBorder::Quality(quality) => {
            let [r, g, b, _] = parse_rgba(quality_color(quality));
            [r * MUTE, g * MUTE, b * MUTE]
        }
    }
}

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
