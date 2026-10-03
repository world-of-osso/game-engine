use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::hud_layout;
use crate::ui::screens::inworld_unit_frames_component::CAST_DOCK_W;

/// The bar fills the cast area.
pub const BAR_W: f32 = CAST_DOCK_W - 8.0;
pub const BAR_H: f32 = 20.0;
const BORDER_W: f32 = BAR_W + 8.0;
const BORDER_H: f32 = BAR_H + 8.0;
const SPARK_W: f32 = 8.0;
const TEXT_H: f32 = 14.0;
const TIMER_W: f32 = 40.0;

const BORDER_BG: &str = "0.0,0.0,0.0,0.8";
const BAR_BG: &str = "0.15,0.15,0.15,0.9";
const FILL_CAST: &str = "1.0,0.7,0.0,1.0";
const FILL_CHANNEL: &str = "0.0,0.64,0.0,1.0";
const FILL_UNINTERRUPTIBLE: &str = "0.63,0.63,0.63,1.0";
const FILL_INTERRUPTED: &str = "1.0,0.0,0.0,1.0";
/// Retail GlobalStrings `INTERRUPTED`.
pub const INTERRUPTED_TEXT: &str = "Interrupted";
const SPARK_COLOR: &str = "1.0,1.0,1.0,0.8";
const SPELL_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const TIMER_COLOR: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct CastingBarState {
    pub visible: bool,
    pub spell_name: String,
    pub timer_text: String,
    /// Fill fraction 0.0..=1.0.
    pub progress: f32,
    pub is_channel: bool,
    pub is_interruptible: bool,
    pub is_interrupted: bool,
}

impl Default for CastingBarState {
    fn default() -> Self {
        Self {
            visible: false,
            spell_name: String::new(),
            timer_text: String::new(),
            progress: 0.0,
            is_channel: false,
            is_interruptible: true,
            is_interrupted: false,
        }
    }
}

impl CastingBarState {
    /// Full red bar with "Interrupted", as Retail `CastingBarFrame` shows it.
    pub fn interrupted() -> Self {
        Self {
            visible: true,
            spell_name: INTERRUPTED_TEXT.into(),
            progress: 1.0,
            is_interrupted: true,
            ..Self::default()
        }
    }
}

pub fn casting_bar_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CastingBarState>()
        .expect("CastingBarState must be in SharedContext");
    let hide = !state.visible;
    let fill_w = BAR_W * state.progress.clamp(0.0, 1.0);
    let fill_color = bar_fill_color(state);
    let spark_x = fill_w - SPARK_W / 2.0;
    let at = hud_layout(ctx).cast_bar.place((BORDER_W, BORDER_H));
    rsx! {
        r#frame {
            name: "PlayerCastingBarFrame",
            width: {BORDER_W},
            height: {BORDER_H},
            background_color: BORDER_BG,
            hidden: hide,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {bar_background(fill_w, fill_color, spark_x, &state.spell_name, &state.timer_text)}
        }
    }
}

fn bar_fill_color(state: &CastingBarState) -> &'static str {
    if state.is_interrupted {
        FILL_INTERRUPTED
    } else if !state.is_interruptible {
        FILL_UNINTERRUPTIBLE
    } else if state.is_channel {
        FILL_CHANNEL
    } else {
        FILL_CAST
    }
}

fn bar_background(fill_w: f32, color: &str, spark_x: f32, name: &str, timer: &str) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarBackground",
            width: {BAR_W},
            height: {BAR_H},
            background_color: BAR_BG,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            {fill_bar(fill_w, color)}
            {spark(spark_x)}
            {spell_name_text(name)}
            {timer_text(timer)}
        }
    }
}

fn fill_bar(fill_w: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarFill",
            width: {fill_w},
            height: {BAR_H},
            background_color: color,
            pos_type: "absolute",
            pos_x: 0.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spark(x: f32) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarSpark",
            width: {SPARK_W},
            height: {BAR_H + 6.0},
            background_color: SPARK_COLOR,
            pos_type: "absolute",
            pos_x: x,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spell_name_text(name: &str) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarSpellName",
            width: {BAR_W - TIMER_W},
            height: {TEXT_H},
            text: name,
            font_size: 10.0,
            font_color: SPELL_NAME_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

fn timer_text(timer: &str) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarTimer",
            width: {TIMER_W},
            height: {TEXT_H},
            text: timer,
            font_size: 9.0,
            font_color: TIMER_COLOR,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 2.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}
