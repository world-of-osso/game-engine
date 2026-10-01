//! Original tooltip line records, typography, collection marks, money and authored screen.

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::merchant_frame_component::{MoneyAlign, money};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

pub const TOOLTIP_W: f32 = 260.0;
pub const TOOLTIP_MIN_H: f32 = 34.0;
pub const TOOLTIP_INSET: f32 = 8.0;
pub const TOOLTIP_TITLE_H: f32 = 16.0;
pub const TOOLTIP_LINE_H: f32 = 14.0;
/// Collection mark drawn at the start of an item line, then the name.
pub const TOOLTIP_MARK_SIZE: f32 = 12.0;
pub const TOOLTIP_MARK_GAP: f32 = 2.0;

/// Retail `READY_CHECK_READY_TEXTURE` / `READY_CHECK_NOT_READY_TEXTURE`
/// (ReadyCheck.lua:2-4): atlases `UI-LFG-ReadyMark` / `UI-LFG-DeclineMark`,
/// 40×40 members of `interface/lfgframe/uilfgprompts.blp` (5171843, 2048×2048).
pub const COLLECTED_MARK: AtlasArt = AtlasArt {
    fdid: 5_171_843,
    atlas: (2048.0, 2048.0),
    rect: (1745.0, 1945.0, 259.0, 459.0),
};
pub const UNCOLLECTED_MARK: AtlasArt = AtlasArt {
    fdid: 5_171_843,
    atlas: (2048.0, 2048.0),
    rect: (1801.0, 2001.0, 1.0, 201.0),
};

pub const TOOLTIP_BG: &str = "0.03,0.02,0.01,0.96";
pub const TOOLTIP_BORDER: &str = "1px solid 0.66,0.54,0.22,0.95";
pub const TOOLTIP_TEXT_COLOR: [f32; 4] = [0.92, 0.89, 0.82, 1.0];
pub const TOOLTIP_LABEL_COLOR: [f32; 4] = [0.72, 0.72, 0.72, 1.0];
pub const TOOLTIP_BUFF_COLOR: [f32; 4] = [1.0, 0.82, 0.32, 1.0];
pub const TOOLTIP_SPELL_COLOR: [f32; 4] = [0.98, 0.88, 0.54, 1.0];
pub const TOOLTIP_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const TOOLTIP_DESCRIPTION_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
/// `GRAY_FONT_COLOR`.
pub const GRAY_FONT_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
pub const TOOLTIP_FONT_SIZE: f32 = 10.0;
pub const TOOLTIP_TEXT_W: f32 = TOOLTIP_W - 2.0 * TOOLTIP_INSET;

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Collection mark of a listed item: a green check for a collected appearance, a
/// red cross for an uncollected one, nothing for an item without an appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemMark {
    Unmarked,
    Collected,
    Uncollected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TooltipLineState {
    pub left_text: String,
    pub right_text: String,
    pub left_color: [f32; 4],
    pub right_color: [f32; 4],
    /// Item lines keep a mark column before the text; `None` for other lines.
    pub item_mark: Option<ItemMark>,
    /// Coins after the left text (`SetTooltipMoney`).
    pub money: Option<u64>,
}

impl TooltipLineState {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            left_text: text.into(),
            right_text: String::new(),
            left_color: TOOLTIP_TEXT_COLOR,
            right_color: TOOLTIP_TEXT_COLOR,
            item_mark: None,
            money: None,
        }
    }

    /// `SetTooltipMoney(tooltip, copper, nil, label)`: a white label and coins.
    pub fn money(label: impl Into<String>, copper: u64) -> Self {
        Self {
            money: Some(copper),
            ..Self::colored(label, TOOLTIP_WHITE)
        }
    }

    pub fn colored(text: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            left_color: color,
            ..Self::new(text)
        }
    }

    pub fn pair(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left_text: left.into(),
            right_text: right.into(),
            left_color: TOOLTIP_WHITE,
            right_color: TOOLTIP_WHITE,
            item_mark: None,
            money: None,
        }
    }

    pub fn key_value(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            left_text: label.into(),
            right_text: value.into(),
            left_color: TOOLTIP_LABEL_COLOR,
            right_color: TOOLTIP_TEXT_COLOR,
            item_mark: None,
            money: None,
        }
    }
}

/// Portable content and screen-pixel position for the original authored tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipPresentation {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    pub title: String,
    pub title_color: [f32; 4],
    pub lines: Vec<TooltipLineState>,
}

impl Default for TooltipPresentation {
    fn default() -> Self {
        Self::hidden()
    }
}

impl TooltipPresentation {
    pub fn hidden() -> Self {
        Self {
            visible: false,
            x: 0.0,
            y: 0.0,
            title: String::new(),
            title_color: TOOLTIP_TEXT_COLOR,
            lines: Vec::new(),
        }
    }

    pub fn height(&self) -> f32 {
        tooltip_height(self.lines.len())
    }
}

pub fn tooltip_height(line_count: usize) -> f32 {
    let lines_h = line_count as f32 * TOOLTIP_LINE_H;
    (2.0 * TOOLTIP_INSET + TOOLTIP_TITLE_H + lines_h).max(TOOLTIP_MIN_H)
}

/// Original idTip-style item line; the host appends it before positioning.
pub fn item_id_line(item_id: u32) -> TooltipLineState {
    TooltipLineState::colored(format!("Item ID: {item_id}"), GRAY_FONT_COLOR)
}

pub fn append_item_id(state: &mut TooltipPresentation, item_id: u32) {
    state.lines.push(item_id_line(item_id));
}

/// Description paragraphs word-wrapped to the tooltip text width. A paragraph that
/// opens with a `|cAARRGGBB` colour escape takes that colour; colour escapes and `|r`
/// are not shown. Blank source lines are kept as paragraph breaks.
pub fn description_lines(text: &str, color: [f32; 4]) -> Vec<TooltipLineState> {
    let mut lines = Vec::new();
    for paragraph in text.lines().map(str::trim) {
        let color = leading_color_escape(paragraph).unwrap_or(color);
        let plain = strip_color_escapes(paragraph);
        let wrapped = wrap_paragraph(&plain);
        if wrapped.is_empty() {
            lines.push(TooltipLineState::colored(String::new(), color));
        }
        lines.extend(
            wrapped
                .into_iter()
                .map(|line| TooltipLineState::colored(line, color)),
        );
    }
    while lines.last().is_some_and(|line| line.left_text.is_empty()) {
        lines.pop();
    }
    lines
}

/// Colour of a `|cAARRGGBB` escape at the start of `text`.
fn leading_color_escape(text: &str) -> Option<[f32; 4]> {
    let hex = text
        .strip_prefix("|c")
        .or_else(|| text.strip_prefix("|C"))?
        .get(..8)?;
    let channel = |at: usize| {
        u8::from_str_radix(&hex[at..at + 2], 16)
            .ok()
            .map(|v| f32::from(v) / 255.0)
    };
    Some([channel(2)?, channel(4)?, channel(6)?, channel(0)?])
}

fn strip_color_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('|') {
        out.push_str(&rest[..pos]);
        let escape = &rest[pos..];
        let skip = if leading_color_escape(escape).is_some() {
            10
        } else if escape.starts_with("|r") || escape.starts_with("|R") {
            2
        } else {
            out.push('|');
            1
        };
        rest = &escape[skip..];
    }
    out.push_str(rest);
    out
}

/// Greedy word wrap to the measured text width of a tooltip line.
fn wrap_paragraph(paragraph: &str) -> Vec<String> {
    let fits = |line: &str| {
        measure_text(line, GameFont::FrizQuadrata, TOOLTIP_FONT_SIZE)
            .expect("FrizQuadrata text measurement")
            .0
            <= TOOLTIP_TEXT_W
    };
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        if !line.is_empty() && !fits(&format!("{line} {word}")) {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

pub fn parse_rgba(input: &str) -> [f32; 4] {
    let values: Vec<f32> = input
        .split(',')
        .filter_map(|part| part.parse().ok())
        .collect();
    match values.as_slice() {
        [r, g, b, _a] => [*r, *g, *b, 1.0],
        [r, g, b] => [*r, *g, *b, 1.0],
        _ => TOOLTIP_TEXT_COLOR,
    }
}

pub fn tooltip_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TooltipPresentation>()
        .expect("TooltipPresentation must be in SharedContext");
    let hidden = !state.visible;
    let height = state.height();
    let title = tooltip_title(state);
    let lines = tooltip_lines(&state.lines);
    rsx! {
        r#frame {
            name: "TooltipFrame",
            width: {TOOLTIP_W},
            height: {height},
            hidden: {hidden},
            strata: "TOOLTIP",
            background_color: TOOLTIP_BG,
            border: TOOLTIP_BORDER,
            pos_type: "absolute",
            anchor: "screen",
            pos_x: {state.x},
            pos_y: {state.y},
            {title}
            {lines}
        }
    }
}

fn tooltip_title(state: &TooltipPresentation) -> Element {
    rsx! {
        fontstring {
            name: "TooltipTitle",
            width: {TOOLTIP_TEXT_W},
            height: {TOOLTIP_TITLE_H},
            text: {state.title.as_str()},
            font: "FrizQuadrata",
            font_size: 12.0,
            font_color: {rgba_string(state.title_color)},
            justify_h: "LEFT",
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {TOOLTIP_INSET},
        }
    }
}

fn tooltip_lines(lines: &[TooltipLineState]) -> Element {
    lines
        .iter()
        .enumerate()
        .flat_map(|(index, line)| tooltip_line(index, line))
        .collect()
}

fn tooltip_line(index: usize, line: &TooltipLineState) -> Element {
    let y = TOOLTIP_INSET + TOOLTIP_TITLE_H + index as f32 * TOOLTIP_LINE_H;
    let indent = if line.item_mark.is_some() {
        TOOLTIP_MARK_SIZE + TOOLTIP_MARK_GAP
    } else {
        0.0
    };
    let mut elements = tooltip_line_mark(index, line.item_mark, y);
    elements.extend(tooltip_line_text(index, line, y, indent));
    if let Some(copper) = line.money {
        let label_w = measure_text(&line.left_text, GameFont::FrizQuadrata, TOOLTIP_FONT_SIZE)
            .map_or(0.0, |(width, _)| width.ceil());
        elements.extend(money(
            &format!("TooltipLine{index}Money"),
            copper,
            (TOOLTIP_INSET + indent + label_w + 4.0, y + TOOLTIP_LINE_H),
            MoneyAlign::Left,
            false,
        ));
    }
    elements
}

fn tooltip_line_mark(index: usize, mark: Option<ItemMark>, y: f32) -> Element {
    let art = match mark {
        Some(ItemMark::Collected) => &COLLECTED_MARK,
        Some(ItemMark::Uncollected) => &UNCOLLECTED_MARK,
        Some(ItemMark::Unmarked) | None => return Element::new(),
    };
    let coords = art.tex_coords(1.0);
    let top = y + (TOOLTIP_LINE_H - TOOLTIP_MARK_SIZE) / 2.0;
    rsx! {
        texture {
            name: {DynName(format!("TooltipLine{index}Mark"))},
            width: {TOOLTIP_MARK_SIZE},
            height: {TOOLTIP_MARK_SIZE},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {top},
        }
    }
}

fn tooltip_line_text(index: usize, line: &TooltipLineState, y: f32, indent: f32) -> Element {
    let left_x = TOOLTIP_INSET + indent;
    let left_w = TOOLTIP_TEXT_W - indent;
    rsx! {
        fontstring {
            name: {DynName(format!("TooltipLine{index}Left"))},
            width: {left_w},
            height: {TOOLTIP_LINE_H},
            text: {line.left_text.as_str()},
            font: "FrizQuadrata",
            font_size: {TOOLTIP_FONT_SIZE},
            font_color: {rgba_string(line.left_color)},
            justify_h: "LEFT",
            pos_type: "absolute",
            pos_x: {left_x},
            pos_y: {y},
        }
        fontstring {
            name: {DynName(format!("TooltipLine{index}Right"))},
            width: {TOOLTIP_TEXT_W},
            height: {TOOLTIP_LINE_H},
            text: {line.right_text.as_str()},
            font: "FrizQuadrata",
            font_size: {TOOLTIP_FONT_SIZE},
            font_color: {rgba_string(line.right_color)},
            justify_h: "RIGHT",
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {y},
        }
    }
}

fn rgba_string(color: [f32; 4]) -> String {
    format!("{},{},{},{}", color[0], color[1], color[2], color[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;

    #[test]
    fn a_sell_price_line_draws_its_coins_after_the_label() {
        #[cfg(godot_host)]
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .expect("tooltip test data root");
        let state = TooltipPresentation {
            visible: true,
            title: "Linen Cloth".into(),
            lines: vec![TooltipLineState::money("Sell Price:", 260)],
            ..TooltipPresentation::hidden()
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(tooltip_frame_screen).sync(&shared, &mut registry);
        let text = |name: &str| match registry
            .get(registry.get_by_name(name).expect(name))
            .and_then(|frame| frame.widget_data.clone())
        {
            Some(ui_toolkit::frame::WidgetData::FontString(fs)) => fs.text,
            _ => panic!("{name} is not a FontString"),
        };
        assert_eq!(text("TooltipLine0Left"), "Sell Price:");
        // 2 silver 60 copper.
        assert_eq!(text("TooltipLine0MoneyAmount0"), "2");
        assert_eq!(text("TooltipLine0MoneyAmount1"), "60");
        assert!(registry.get_by_name("TooltipLine0MoneyCoin1").is_some());
    }
}
