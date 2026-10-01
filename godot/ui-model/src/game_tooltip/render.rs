//! The Retail `GameTooltip` look (`SharedTooltipArtTemplate`, SharedTooltipTemplates.xml:9-24):
//! the `TooltipDefaultLayout` nine-slice (NineSliceLayouts.lua:257-267) with its centre tinted
//! `TOOLTIP_DEFAULT_BACKGROUND_COLOR` (GlobalColor 79, 0.09 0.09 0.188), the first line in
//! `GameTooltipHeaderText` (FRIZQT 14), the rest in `GameTooltipText` (`Tooltip_Med`, FRIZQT
//! 12), 10 px insets, 2 px between lines, a right text at least 20 px after its left text
//! (wow-ui-sim `DOUBLE_LINE_GAP`), and the width of the widest line.

use ui_toolkit::rsx;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::merchant_frame_component::{MoneyAlign, money, money_width};
use crate::tooltip_presentation::{
    COLLECTED_MARK, ItemMark, TooltipLineState, TooltipPresentation, UNCOLLECTED_MARK, rgba_string,
};

pub const HEADER_SIZE: f32 = 14.0;
pub const TEXT_SIZE: f32 = 12.0;
pub const INSET: f32 = 10.0;
pub const LINE_GAP: f32 = 2.0;
const DOUBLE_LINE_GAP: f32 = 20.0;
const MARK_SIZE: f32 = 12.0;
const MARK_GAP: f32 = 2.0;
/// Coins start this far after a `SELL_PRICE` label (`GameTooltip_OnTooltipAddMoney`).
const MONEY_GAP: f32 = 4.0;
/// `TOOLTIP_DEFAULT_COLOR` white border, `TOOLTIP_DEFAULT_BACKGROUND_COLOR` centre.
const BORDER_COLOR: &str = "1.0,1.0,1.0,1.0";
const CENTER_COLOR: &str = "0.09,0.09,0.188,1.0";

/// `TooltipDefaultLayout` pieces: UiTextureAtlas 1825 (4185447, 32×64) corners and the
/// horizontal edges, 1827 (4185474, 32×16) the vertical edges, 1826 (4185455) the centre.
const fn piece(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt { fdid, atlas, rect }
}
const CORNERS: (f32, f32) = (32.0, 64.0);
const TOP_LEFT: AtlasArt = piece(4_185_447, CORNERS, (1.0, 8.0, 45.0, 52.0));
const TOP_RIGHT: AtlasArt = piece(4_185_447, CORNERS, (1.0, 8.0, 54.0, 61.0));
const BOTTOM_LEFT: AtlasArt = piece(4_185_447, CORNERS, (19.0, 26.0, 19.0, 26.0));
const BOTTOM_RIGHT: AtlasArt = piece(4_185_447, CORNERS, (1.0, 8.0, 36.0, 43.0));
const EDGE_TOP: AtlasArt = piece(4_185_447, CORNERS, (0.0, 16.0, 10.0, 17.0));
const EDGE_BOTTOM: AtlasArt = piece(4_185_447, CORNERS, (0.0, 16.0, 1.0, 8.0));
const EDGE_LEFT: AtlasArt = piece(4_185_474, (32.0, 16.0), (1.0, 8.0, 0.0, 16.0));
const EDGE_RIGHT: AtlasArt = piece(4_185_474, (32.0, 16.0), (10.0, 17.0, 0.0, 16.0));
const CENTER: AtlasArt = piece(4_185_455, (64.0, 64.0), (0.0, 64.0, 0.0, 64.0));
/// Corner pieces are 7×7; the centre reaches 4 into them (`x = -4, y = 4, x1 = 4, y1 = -4`).
const CORNER: f32 = 7.0;
const CENTER_INSET: f32 = CORNER - 4.0;

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn text_width(text: &str, size: f32) -> f32 {
    measure_text(text, GameFont::FrizQuadrata, size).map_or(0.0, |(width, _)| width.ceil())
}

fn line_width(line: &TooltipLineState) -> f32 {
    let indent = if line.item_mark.is_some() {
        MARK_SIZE + MARK_GAP
    } else {
        0.0
    };
    let left = text_width(&line.left_text, TEXT_SIZE);
    let right = match line.right_text.is_empty() {
        true => 0.0,
        false => DOUBLE_LINE_GAP + text_width(&line.right_text, TEXT_SIZE),
    };
    let coins = line
        .money
        .map_or(0.0, |copper| MONEY_GAP + money_width(copper));
    indent + left + right + coins
}

/// `[width, height]` of the tooltip: the widest line plus the insets, the lines stacked.
pub fn tooltip_size(tooltip: &TooltipPresentation) -> [f32; 2] {
    let widest = tooltip
        .lines
        .iter()
        .map(line_width)
        .fold(text_width(&tooltip.title, HEADER_SIZE), f32::max);
    let lines = tooltip.lines.len() as f32;
    [
        widest + 2.0 * INSET,
        2.0 * INSET + HEADER_SIZE + lines * (LINE_GAP + TEXT_SIZE),
    ]
}

/// Top of line `index` (0 is the first line after the title).
fn line_top(index: usize) -> f32 {
    INSET + HEADER_SIZE + LINE_GAP + index as f32 * (TEXT_SIZE + LINE_GAP)
}

/// One tooltip: `{prefix}Frame` with its nine-slice, `{prefix}Title` and
/// `{prefix}Line{i}Left`/`Right`/`Mark`/`Money*`.
pub fn retail_tooltip(state: &TooltipPresentation, prefix: &str) -> Element {
    let [width, height] = tooltip_size(state);
    let hidden = !state.visible;
    let border = nine_slice(prefix, width, height);
    let title = text(
        format!("{prefix}Title"),
        &state.title,
        [INSET, INSET, width - 2.0 * INSET, HEADER_SIZE],
        (HEADER_SIZE, state.title_color, "LEFT"),
    );
    let lines: Element = state
        .lines
        .iter()
        .enumerate()
        .flat_map(|(index, line)| line_elements(prefix, index, line, width))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(format!("{prefix}Frame"))},
            width: {width},
            height: {height},
            hidden: {hidden},
            strata: "TOOLTIP",
            pos_type: "absolute",
            anchor: "screen",
            pos_x: {state.x},
            pos_y: {state.y},
            {border}
            {title}
            {lines}
        }
    }
}

fn line_elements(prefix: &str, index: usize, line: &TooltipLineState, width: f32) -> Element {
    let name = format!("{prefix}Line{index}");
    let top = line_top(index);
    let inner = width - 2.0 * INSET;
    let mut elements = mark(&name, line.item_mark, top);
    let indent = if line.item_mark.is_some() {
        MARK_SIZE + MARK_GAP
    } else {
        0.0
    };
    elements.extend(text(
        format!("{name}Left"),
        &line.left_text,
        [INSET + indent, top, inner - indent, TEXT_SIZE],
        (TEXT_SIZE, line.left_color, "LEFT"),
    ));
    elements.extend(text(
        format!("{name}Right"),
        &line.right_text,
        [INSET, top, inner, TEXT_SIZE],
        (TEXT_SIZE, line.right_color, "RIGHT"),
    ));
    if let Some(copper) = line.money {
        let x = INSET + indent + text_width(&line.left_text, TEXT_SIZE) + MONEY_GAP;
        elements.extend(money(
            &format!("{name}Money"),
            copper,
            (x, top + TEXT_SIZE + 1.0),
            MoneyAlign::Left,
            false,
        ));
    }
    elements
}

fn text(name: String, text: &str, [x, y, w, h]: [f32; 4], style: (f32, [f32; 4], &str)) -> Element {
    let (size, color, justify) = style;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: {w.max(1.0)},
            height: {h + 2.0},
            text: {text},
            font: "FrizQuadrata",
            font_size: {size},
            font_color: {rgba_string(color)},
            justify_h: {justify},
            pos_type: "absolute",
            pos_x: {x},
            pos_y: {y},
        }
    }
}

fn mark(name: &str, mark: Option<ItemMark>, top: f32) -> Element {
    let art = match mark {
        Some(ItemMark::Collected) => &COLLECTED_MARK,
        Some(ItemMark::Uncollected) => &UNCOLLECTED_MARK,
        Some(ItemMark::Unmarked) | None => return Element::new(),
    };
    atlas_piece(
        format!("{name}Mark"),
        art,
        [
            INSET,
            top + (TEXT_SIZE - MARK_SIZE) / 2.0,
            MARK_SIZE,
            MARK_SIZE,
        ],
        "1.0,1.0,1.0,1.0",
    )
}

/// `TooltipDefaultLayout` over a `width`×`height` tooltip: corners at their size, edges
/// stretched between them, the tinted centre inset 3.
fn nine_slice(prefix: &str, width: f32, height: f32) -> Element {
    let (w, h, c) = (width, height, CORNER);
    let pieces = [
        (
            "Center",
            &CENTER,
            [
                CENTER_INSET,
                CENTER_INSET,
                w - 2.0 * CENTER_INSET,
                h - 2.0 * CENTER_INSET,
            ],
            CENTER_COLOR,
        ),
        ("TopLeftCorner", &TOP_LEFT, [0.0, 0.0, c, c], BORDER_COLOR),
        (
            "TopRightCorner",
            &TOP_RIGHT,
            [w - c, 0.0, c, c],
            BORDER_COLOR,
        ),
        (
            "BottomLeftCorner",
            &BOTTOM_LEFT,
            [0.0, h - c, c, c],
            BORDER_COLOR,
        ),
        (
            "BottomRightCorner",
            &BOTTOM_RIGHT,
            [w - c, h - c, c, c],
            BORDER_COLOR,
        ),
        ("TopEdge", &EDGE_TOP, [c, 0.0, w - 2.0 * c, c], BORDER_COLOR),
        (
            "BottomEdge",
            &EDGE_BOTTOM,
            [c, h - c, w - 2.0 * c, c],
            BORDER_COLOR,
        ),
        (
            "LeftEdge",
            &EDGE_LEFT,
            [0.0, c, c, h - 2.0 * c],
            BORDER_COLOR,
        ),
        (
            "RightEdge",
            &EDGE_RIGHT,
            [w - c, c, c, h - 2.0 * c],
            BORDER_COLOR,
        ),
    ];
    pieces
        .into_iter()
        .flat_map(|(part, art, rect, color)| {
            atlas_piece(format!("{prefix}NineSlice{part}"), art, rect, color)
        })
        .collect()
}

fn atlas_piece(name: String, art: &AtlasArt, [x, y, w, h]: [f32; 4], color: &str) -> Element {
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name: {DynName(name)},
            width: {w.max(0.0)},
            height: {h.max(0.0)},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: color,
            pos_type: "absolute",
            pos_x: {x},
            pos_y: {y},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_fits_the_widest_line_and_stacks_the_lines() {
        super::super::set_test_data_root();
        let tooltip = TooltipPresentation {
            visible: true,
            title: "Zoom In".into(),
            ..TooltipPresentation::hidden()
        };
        let [width, height] = tooltip_size(&tooltip);
        assert_eq!(width, text_width("Zoom In", HEADER_SIZE) + 20.0);
        assert_eq!(height, 20.0 + 14.0);
        let pair = TooltipPresentation {
            lines: vec![
                TooltipLineState::pair("20 Rage", "Melee Range"),
                TooltipLineState::new("x"),
            ],
            ..tooltip
        };
        let [width, height] = tooltip_size(&pair);
        let expected =
            text_width("20 Rage", TEXT_SIZE) + 20.0 + text_width("Melee Range", TEXT_SIZE);
        assert_eq!(
            width,
            expected.max(text_width("Zoom In", HEADER_SIZE)) + 20.0
        );
        assert_eq!(height, 20.0 + 14.0 + 2.0 * 14.0);
    }
}
