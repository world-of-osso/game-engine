use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const GHOST_HINT_FRAME: FrameName = FrameName("GhostHintFrame");
pub const GHOST_HINT_TEXT: FrameName = FrameName("GhostHintFrameText");

const FRAME_W: f32 = 512.0;
/// Below `UIErrorsFrame` (top 122, 60 tall).
const FRAME_TOP: f32 = 190.0;
const FRAME_H: f32 = 24.0;
const FONT_SIZE: f32 = 16.0;
/// Retail `NORMAL_FONT_COLOR` gold.
const HINT_COLOR: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);

/// Corpse-run hint shown while a ghost is out of corpse range.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GhostHintState {
    /// Rounded distance to the corpse in yards; `None` hides the hint.
    pub corpse_yards: Option<u32>,
}

pub fn ghost_hint_text(yards: u32) -> String {
    format!("Return to your corpse to resurrect ({yards} yds)")
}

pub fn ghost_hint_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GhostHintState>()
        .expect("GhostHintState must be in SharedContext");
    let hide = state.corpse_yards.is_none();
    let text = state.corpse_yards.map(ghost_hint_text).unwrap_or_default();
    rsx! {
        r#frame {
            name: GHOST_HINT_FRAME,
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: FRAME_TOP,
            fontstring {
                name: GHOST_HINT_TEXT,
                width: FRAME_W,
                height: FRAME_H,
                text: text.as_str(),
                font: GameFont::FrizQuadrata,
                font_size: FONT_SIZE,
                font_color: HINT_COLOR,
                justify_h: "CENTER",
                strata: FrameStrata::Dialog,
            }
        }
    }
}
