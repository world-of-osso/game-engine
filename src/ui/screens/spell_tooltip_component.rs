//! Spell tooltip in the `GameTooltip` layout (`Blizzard_SharedXML/Mainline/GameTooltip*`):
//! the spell name in white, pairs of left/right lines (cost | range, cast time |
//! cooldown) in white, then the rendered description in gold, wrapped by the host.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const SPELL_TOOLTIP: FrameName = FrameName("SpellTooltip");
/// `GameTooltip` text insets and line gap.
pub const PADDING: f32 = 10.0;
pub const LINE_GAP: f32 = 2.0;
/// `GameTooltipHeaderText` 14, `GameTooltipText` 12.
pub const HEADER_SIZE: f32 = 14.0;
pub const TEXT_SIZE: f32 = 12.0;
pub const HEADER_H: f32 = 17.0;
pub const LINE_H: f32 = 15.0;
/// `TOOLTIP_DEFAULT_BACKGROUND_COLOR` 0.09,0.09,0.19 and `TOOLTIP_DEFAULT_COLOR` border.
const BACKGROUND: &str = "0.09,0.09,0.19,0.9";
const BORDER: &str = "0.6,0.6,0.6,1.0";
const WHITE: &str = "1.0,1.0,1.0,1.0";
/// `NORMAL_FONT_COLOR`.
const GOLD: &str = "1.0,0.82,0.0,1.0";
/// `RED_FONT_COLOR`.
const RED: &str = "1.0,0.125,0.125,1.0";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellTooltipState {
    pub visible: bool,
    /// Top-left in screen pixels.
    pub origin: [f32; 2],
    pub width: f32,
    pub name: String,
    /// Left and right text of each detail line.
    pub details: Vec<(String, String)>,
    /// Description lines, already wrapped to `width - 2 * PADDING`.
    pub description: Vec<String>,
    /// Red requirement line ("Level 10"), for spells not learned yet.
    pub requirement: Option<String>,
}

impl SpellTooltipState {
    pub fn height(&self) -> f32 {
        let lines = self.details.len() + self.requirement.iter().count() + self.description.len();
        2.0 * PADDING + HEADER_H + lines as f32 * (LINE_H + LINE_GAP)
    }
}

struct DynName(String);

fn text(
    name: String,
    text: &str,
    rect: [f32; 4],
    size: f32,
    color: &str,
    justify: &str,
) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size: size,
            font_color: color,
            justify_h: justify,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

pub fn spell_tooltip_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<SpellTooltipState>()
        .expect("SpellTooltipState must be in SharedContext");
    let inner = state.width - 2.0 * PADDING;
    let mut y = PADDING;
    let mut lines = text(
        "SpellTooltipTextLeft1".into(),
        &state.name,
        [PADDING, y, inner, HEADER_H],
        HEADER_SIZE,
        WHITE,
        "LEFT",
    );
    y += HEADER_H + LINE_GAP;
    let mut row = 2;
    for (left, right) in &state.details {
        lines.extend(text(
            format!("SpellTooltipTextLeft{row}"),
            left,
            [PADDING, y, inner, LINE_H],
            TEXT_SIZE,
            WHITE,
            "LEFT",
        ));
        lines.extend(text(
            format!("SpellTooltipTextRight{row}"),
            right,
            [PADDING, y, inner, LINE_H],
            TEXT_SIZE,
            WHITE,
            "RIGHT",
        ));
        y += LINE_H + LINE_GAP;
        row += 1;
    }
    if let Some(requirement) = &state.requirement {
        lines.extend(text(
            format!("SpellTooltipTextLeft{row}"),
            requirement,
            [PADDING, y, inner, LINE_H],
            TEXT_SIZE,
            RED,
            "LEFT",
        ));
        y += LINE_H + LINE_GAP;
        row += 1;
    }
    for line in &state.description {
        lines.extend(text(
            format!("SpellTooltipTextLeft{row}"),
            line,
            [PADDING, y, inner, LINE_H],
            TEXT_SIZE,
            GOLD,
            "LEFT",
        ));
        y += LINE_H + LINE_GAP;
        row += 1;
    }
    let [x, top] = state.origin;
    let height = state.height();
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: SPELL_TOOLTIP,
            width: {state.width},
            height,
            hidden: hide,
            background_color: BORDER,
            strata: FrameStrata::Tooltip,
            pos_type: "absolute",
            left: x,
            top,
            r#frame {
                name: "SpellTooltipBackground",
                width: {state.width - 2.0},
                height: {height - 2.0},
                background_color: BACKGROUND,
                pos_type: "absolute",
                pos_x: 1.0,
                pos_y: 1.0,
            }
            {lines}
        }
    }
}
