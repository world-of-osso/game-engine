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
use ui_toolkit::widget_def::{Attr, Element, WidgetChild};
use ui_toolkit::widgets::font_string::{FontColor, GameFont, JustifyH};
use ui_toolkit::widgets::texture::TextureSource;

use crate::damage_meter_data::class_color;
use crate::merchant_data::quality_color;
use crate::tooltip_presentation::{TooltipBorder, parse_rgba};

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

/// Header palette and font: DamageMeter.lua:60-61,548,705; Core.lua:194.
pub const FLARE_ACTIVE_TEXT: [f32; 4] = [0.80, 0.60, 0.34, 1.0];
pub const FLARE_INACTIVE_TEXT: [f32; 4] = [0.56, 0.51, 0.46, 1.0];
pub const FLARE_ICON_COLOR: &str = "0.61,0.48,0.29,1";
pub const FLARE_HEADER_HEIGHT: f32 = 24.0;
pub const FLARE_FONT_SIZE: f32 = 12.0;
/// Chat.lua:531; Core.lua:161,166,171,176 scales the 22px icons to 0.6.
/// Keep the hit box separate from the glyph, shared with the meter.
pub const FLARE_HEADER_BUTTON_SIZE: f32 = 22.0;
pub const FLARE_HEADER_ICON_SIZE: f32 = 13.2;
pub const FLARE_HEADER_ICON_INSET: f32 = 4.4;

/// Blizzard glyph crops `(FileDataID, [left, right, top, bottom])`, without cell padding
/// and button frames. FlareUI draws its own Media art on Blizzard's chat buttons
/// (Chat.lua:54-61,609-611); that art is not reusable, so each glyph is cut from the
/// Blizzard art of the button it restyles.
/// Pixel measurements and provenance: docs/specs/forever-chat-meter-chrome.md.
///
/// `ChatFrameMenuButton` (Chat.lua:611): the speech bubble of its NormalTexture
/// `Interface\ChatFrame\UI-ChatIcon-Chat-Up` (FloatingChatFrame.xml:670).
pub const FLARE_MENU_ART: (u32, [f32; 4]) =
    (130_949, [8.0 / 32.0, 22.0 / 32.0, 9.0 / 32.0, 23.0 / 32.0]);
/// The meter's settings cog without its dropdown frame.
pub const FLARE_GEAR_ART: (u32, [f32; 4]) = (
    7_518_377,
    [67.0 / 128.0, 79.0 / 128.0, 35.0 / 64.0, 47.0 / 64.0],
);
/// `QuickJoinToastButton` (Chat.lua:609): the figure of its `FriendsButton` atlas
/// `quickjoin-button-friendslist-up` (QuickJoinToast.xml:42).
pub const FLARE_SOCIAL_ART: (u32, [f32; 4]) = (
    1_537_274,
    [346.0 / 512.0, 360.0 / 512.0, 7.0 / 64.0, 21.0 / 64.0],
);
pub const FLARE_VOLUME_ART: (u32, [f32; 4]) = (
    5_390_329,
    [388.0 / 512.0, 399.0 / 512.0, 31.0 / 256.0, 45.0 / 256.0],
);

/// Header and measured thin solid separator, siblings in the panel's parent.
/// Measurements and all derived rectangles: docs/specs/forever-chat-meter-chrome.md.
pub fn flare_header(root: &str, (x, y, width, _): (f32, f32, f32, f32)) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("{root}Header"))},
            width,
            height: FLARE_HEADER_HEIGHT,
            pos_type: "absolute",
            left: x,
            top: y,
        }
        r#frame {
            name: {DynName(format!("{root}Separator"))},
            width: {width - 2.0 * INSET},
            height: 1.0,
            background_color: "0.65,0.49,0.27,1",
            pos_type: "absolute",
            left: {x + INSET},
            top: {y + FLARE_HEADER_HEIGHT},
        }
    }
}

/// Shadowed reference text, using FlareUI's font values, not its artwork.
pub fn flare_text(
    name: &str,
    label: &str,
    rect: [f32; 4],
    color: [f32; 4],
    justify: JustifyH,
) -> Element {
    flare_text_sized(name, label, rect, color, justify, FLARE_FONT_SIZE)
}

/// [`flare_text`] at `font_size`.
pub fn flare_text_sized(
    name: &str,
    label: &str,
    [x, y, width, height]: [f32; 4],
    [r, g, b, a]: [f32; 4],
    justify: JustifyH,
    font_size: f32,
) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_owned())},
            width,
            height,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: {FontColor::new(r, g, b, a)},
            shadow_color: "0,0,0,1",
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Existing Blizzard glyph, bronze tinted and centred inside its hit box.
/// Only supported actions get a hit target.
pub fn flare_icon(
    name: &str,
    (fdid, [left, right, top, bottom]): (u32, [f32; 4]),
    [x, y, width, height]: [f32; 4],
    action: Option<&str>,
) -> Element {
    let tex_coords = format!("{left},{right},{top},{bottom}");
    let mut icon = rsx! {
        r#frame {
            name: {DynName(name.to_owned())},
            width,
            height,
            pos_type: "absolute",
            left: x,
            top: y,
            texture {
                name: {DynName(format!("{name}Icon"))},
                width: FLARE_HEADER_ICON_SIZE,
                height: FLARE_HEADER_ICON_SIZE,
                texture_fdid: fdid,
                tex_coords: {tex_coords.as_str()},
                vertex_color: FLARE_ICON_COLOR,
                pos_type: "absolute",
                left: FLARE_HEADER_ICON_INSET,
                top: FLARE_HEADER_ICON_INSET,
            }
        }
    };
    if let (Some(action), Some(WidgetChild::Widget(frame))) = (action, icon.first_mut()) {
        frame
            .attrs
            .push(Attr::new_dynamic("onclick", action.to_owned()));
        frame
            .attrs
            .push(Attr::new_dynamic("mouse_enabled", "true".to_owned()));
    }
    icon
}

/// `tooltips.background` (Core.lua:333): `NineSlice:SetCenterColor` (Tooltips.lua:142-143).
pub const TOOLTIP_BACKGROUND: [f32; 4] = [0.05, 0.05, 0.06, 0.9];

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
