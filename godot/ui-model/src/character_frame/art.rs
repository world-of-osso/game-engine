//! CharacterFrame art: the inset and stats-pane backgrounds, the race backdrop behind the
//! model, the paperdoll inner border and the slot frames (CF.xml texture templates).

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{Column, FRAME_NAME, INSET, INSET_RIGHT, MODEL, PaperDollButton, SLOT, STATS};
use crate::quest_art::DynName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;

pub(super) const WHITE: &str = "1.0,1.0,1.0,1.0";
pub(super) const FULL: &str = "0,1,0,1";
const INSET_BG: u32 = 374_154; // Interface\FrameGeneral\UI-Background-Marble
pub(super) const WHITE_ICON_FRAME: u32 = 651_080; // Interface\Common\WhiteIconFrame
/// `Interface\CharacterFrame\Char-Paperdoll-Parts` and its tiled border strips.
const PAPERDOLL_PARTS: u32 = 410_248;
const PAPERDOLL_HORIZONTAL: u32 = 410_247;
const PAPERDOLL_VERTICAL: u32 = 410_249;

/// `character-panel-background` (27190), atlas 2845 `5882640` 1024×512.
const PANEL_BACKGROUND: AtlasArt = art(5_882_640, (1024.0, 512.0), (1.0, 451.0, 1.0, 421.0));
/// UiTextureAtlas 838 `1400895` 1024×1024 and 839 `1400896` 1024×512.
const INFO_ATLAS: (u32, (f32, f32)) = (1_400_895, (1024.0, 1024.0));
const INFO_ATLAS2: (u32, (f32, f32)) = (1_400_896, (1024.0, 512.0));
/// `UI-Character-Info-Title` (6033) 196×40.
pub(super) const CATEGORY_TITLE: AtlasArt = info((1.0, 197.0, 715.0, 755.0));
/// `UI-Character-Info-Line-Bounce` (6028) 157×19.
pub(super) const LINE_BOUNCE: AtlasArt = info((1.0, 158.0, 788.0, 807.0));
/// `UI-Character-Info-ItemLevel-Bounce` (6030) 162×29.
pub(super) const ITEM_LEVEL_BOUNCE: AtlasArt = info((1.0, 163.0, 757.0, 786.0));

const fn art(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt { fdid, atlas, rect }
}

const fn info(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(INFO_ATLAS.0, INFO_ATLAS.1, rect)
}

const fn info2(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(INFO_ATLAS2.0, INFO_ATLAS2.1, rect)
}

/// `UI-Character-Info-<CLASS>-BG` (6022-6040) 197×355 by `ChrClasses` id; Evoker has none.
pub fn class_background(class_id: u8) -> Option<AtlasArt> {
    let row = |left: f32, top: f32| (left, left + 197.0, top, top + 355.0);
    Some(match class_id {
        1 => info(row(797.0, 1.0)),
        2 => info(row(200.0, 1.0)),
        3 => info2(row(598.0, 1.0)),
        4 => info(row(399.0, 1.0)),
        5 => info(row(200.0, 358.0)),
        6 => info2(row(1.0, 1.0)),
        7 => info(row(399.0, 358.0)),
        8 => info(row(1.0, 1.0)),
        9 => info(row(598.0, 1.0)),
        10 => info(row(1.0, 358.0)),
        11 => info2(row(399.0, 1.0)),
        12 => info2(row(200.0, 1.0)),
        _ => return None,
    })
}

/// `Interface\DressUpFrame\DressUpBackground-<ChrRaces.ClientFileString>1`; the four
/// quarters are consecutive FileDataIDs (community listfile).
pub fn race_background(race_id: u8) -> Option<u32> {
    Some(match race_id {
        1 => 131_093,
        2 => 131_101,
        3 => 131_089,
        4 => 131_097,
        5 => 131_105,
        6 => 131_109,
        7 => 455_998,
        8 => 456_006,
        9 => 456_002,
        10 => 131_081,
        11 => 131_085,
        22 => 456_010,
        24..=26 => 603_300,
        27 => 1_821_879,
        28 => 1_776_814,
        29 => 1_776_826,
        30 => 1_776_818,
        31 => 2_763_598,
        32 => 2_734_964,
        34 => 2_054_328,
        35 => 3_185_304,
        36 => 2_055_420,
        37 => 3_185_300,
        52 | 70 => 4_709_136,
        _ => return None,
    })
}

/// `SetPaperDollBackground`'s per-race `BackgroundOverlay` alpha (PDF.lua:2648-2662).
pub fn race_overlay_alpha(race_id: u8) -> f32 {
    match race_id {
        10 => 0.8,
        2 | 4 | 8 | 9 => 0.6,
        5 => 0.3,
        22 => 0.5,
        _ => 0.7,
    }
}

pub(super) fn texture(
    name: String,
    fdid: u32,
    rect: (f32, f32, f32, f32),
    coords: &str,
    color: &str,
) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: coords,
            vertex_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

pub(super) fn atlas(
    name: String,
    art: &AtlasArt,
    rect: (f32, f32, f32, f32),
    color: &str,
) -> Element {
    texture(name, art.fdid, rect, &art.tex_coords(1.0), color)
}

/// `Inset.Background` `character-panel-background`; `InsetRight` (InsetFrameTemplate)
/// with the class background of the stats pane.
pub(super) fn inset_backgrounds(class_id: u8) -> Element {
    let (left, top, right, bottom) = INSET;
    let mut children = atlas(
        format!("{FRAME_NAME}Background"),
        &PANEL_BACKGROUND,
        (left, top, right - left, bottom - top),
        WHITE,
    );
    let (left, top, right, bottom) = INSET_RIGHT;
    children.extend(texture(
        format!("{FRAME_NAME}InsetRightBg"),
        INSET_BG,
        (left, top, right - left, bottom - top),
        FULL,
        WHITE,
    ));
    if let Some(art) = class_background(class_id) {
        let (w, h) = art.size();
        children.extend(atlas(
            "CharacterStatsPaneClassBackground".into(),
            &art,
            (STATS.0, STATS.1, w, h),
            WHITE,
        ));
    }
    children
}

pub(super) const MODEL_BACKGROUND_NAMES: [&str; 4] = [
    "CharacterModelFrameBackgroundTopLeft",
    "CharacterModelFrameBackgroundTopRight",
    "CharacterModelFrameBackgroundBotLeft",
    "CharacterModelFrameBackgroundBotRight",
];

/// The race backdrop quarters and their black `BackgroundOverlay` (PDF.xml:626-664).
pub(super) fn race_backdrop(race_id: u8) -> Element {
    let Some(first) = race_background(race_id) else {
        return Element::default();
    };
    let (x, y, ..) = MODEL;
    // (size, offset, tex coords) of TopLeft, TopRight, BotLeft, BotRight.
    let quarters = [
        (
            (212.0, 245.0),
            (0.0, 0.0),
            "0.171875,1,0.0392156862745098,1",
        ),
        (
            (19.0, 245.0),
            (212.0, 0.0),
            "0,0.296875,0.0392156862745098,1",
        ),
        ((212.0, 128.0), (0.0, 245.0), "0.171875,1,0,1"),
        ((19.0, 128.0), (212.0, 245.0), "0,0.296875,0,1"),
    ];
    let mut children: Element = quarters
        .into_iter()
        .zip(MODEL_BACKGROUND_NAMES)
        .zip(first..)
        .flat_map(|((((w, h), (dx, dy), coords), name), fdid)| {
            texture(name.into(), fdid, (x + dx, y + dy, w, h), coords, WHITE)
        })
        .collect();
    // TopLeft to BotRight, its bottom 52 up.
    let overlay = format!("0.0,0.0,0.0,{}", race_overlay_alpha(race_id));
    children.extend(rsx! {
        r#frame {
            name: "CharacterModelFrameBackgroundOverlay",
            width: 231.0,
            height: {245.0 + 128.0 - 52.0},
            background_color: {overlay.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    });
    children
}

/// `PaperDollInnerBorder*` corners (`Char-Corner-*`, relative to the Inset).
fn inner_corners() -> [(&'static str, (f32, f32), &'static str); 4] {
    let (left, top, right, bottom) = INSET;
    let (x_left, x_right) = (left + 46.0, right - 47.0 - 7.0);
    let (y_top, y_bottom) = (top + 4.0, bottom - 31.0 - 7.0);
    [
        (
            "TopLeft",
            (x_left, y_top),
            "0.40625,0.43359375,0.8046875,0.859375",
        ),
        (
            "TopRight",
            (x_right, y_top),
            "0.40625,0.43359375,0.734375,0.7890625",
        ),
        (
            "BottomLeft",
            (x_left, y_bottom),
            "0.40625,0.43359375,0.6640625,0.71875",
        ),
        (
            "BottomRight",
            (x_right, y_bottom),
            "0.40625,0.43359375,0.59375,0.6484375",
        ),
    ]
}

/// The paperdoll inner border: corners, the tiled edges between them and `Bottom2` 27
/// above the Inset bottom (PDF.xml:667-716).
pub(super) fn inner_border() -> Element {
    let corners = inner_corners();
    let mut children: Element = corners
        .iter()
        .flat_map(|(name, (x, y), coords)| {
            let name = format!("PaperDollInnerBorder{name}");
            texture(name, PAPERDOLL_PARTS, (*x, *y, 7.0, 7.0), coords, WHITE)
        })
        .collect();
    let (tl, tr, bl) = (corners[0].1, corners[1].1, corners[2].1);
    let (side_h, top_w) = (bl.1 - tl.1 - 7.0, tr.0 - tl.0 - 7.0);
    let (left, _, right, bottom) = INSET;
    let edges = [
        (
            "Left",
            PAPERDOLL_VERTICAL,
            (tl.0 - 1.0, tl.1 + 7.0, 5.0, side_h),
            "0.0625,0.375,0,1",
        ),
        (
            "Right",
            PAPERDOLL_VERTICAL,
            (tr.0 + 3.0, tr.1 + 7.0, 5.0, side_h),
            "0.5,0.8125,0,1",
        ),
        (
            "Top",
            PAPERDOLL_HORIZONTAL,
            (tl.0 + 7.0, tl.1 - 1.0, top_w, 5.0),
            "0,1,0.5,0.8125",
        ),
        (
            "Bottom",
            PAPERDOLL_HORIZONTAL,
            (tl.0 + 7.0, bl.1 + 3.0, top_w, 5.0),
            "0,1,0.0625,0.375",
        ),
        (
            "Bottom2",
            PAPERDOLL_HORIZONTAL,
            (left, bottom - 32.0, right - left, 5.0),
            "0,1,0.0625,0.375",
        ),
    ];
    for (name, fdid, rect, coords) in edges {
        children.extend(texture(
            format!("PaperDollInnerBorder{name}"),
            fdid,
            rect,
            coords,
            WHITE,
        ));
    }
    children
}

/// `Char-LeftSlot` 49×44 TOPLEFT -4, `Char-RightSlot` 50×44 TOPRIGHT +4, `Char-BottomSlot`
/// 42×53 TOPLEFT -4,+8, plus the weapon pair's outer caps `Char-Slot-Bottom-Left/Right`.
pub(super) fn slot_frame_art(button: &PaperDollButton) -> Element {
    let name = button.name;
    let (rect, coords) = match button.column {
        Column::Left => (
            (-4.0, 0.0, 49.0, 44.0),
            "0.20703125,0.3984375,0.59375,0.9375",
        ),
        Column::Right => (
            (SLOT + 4.0 - 50.0, 0.0, 50.0, 44.0),
            "0.00390625,0.19921875,0.59375,0.9375",
        ),
        Column::Bottom => (
            (-4.0, -8.0, 42.0, 53.0),
            "0.671875,0.8359375,0.0078125,0.421875",
        ),
    };
    let mut children = texture(format!("{name}Frame"), PAPERDOLL_PARTS, rect, coords, WHITE);
    let cap = match (button.column, button.row) {
        (Column::Bottom, 0) => Some((
            (-10.0, -8.0, 6.0, 54.0),
            "0.70703125,0.73046875,0.4375,0.859375",
        )),
        (Column::Bottom, _) => Some((
            (38.0, -8.0, 7.0, 54.0),
            "0.671875,0.69921875,0.4375,0.859375",
        )),
        _ => None,
    };
    if let Some((rect, coords)) = cap {
        children.extend(texture(
            format!("{name}FrameCap"),
            PAPERDOLL_PARTS,
            rect,
            coords,
            WHITE,
        ));
    }
    children
}
