use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::hud_layout;
use crate::ui::screens::inworld_unit_frames_component::CAST_DOCK_W;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_flare::{
    FLARE_INSET, flare_border,
};

/// The bar fills the cast area.
pub const BAR_W: f32 = CAST_DOCK_W - 8.0;
pub const BAR_H: f32 = 20.0;
const BAR_INSET: f32 = 4.0;
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
/// How a skin draws the player cast bar: bar size, the holder's inset around it, the
/// holder's background and the fill colours.
struct CastBarStyle {
    bar: (f32, f32),
    inset: f32,
    holder_background: &'static str,
    /// The bronze border round the holder.
    border: bool,
    spark: bool,
    icon: bool,
    cast: &'static str,
    channel: &'static str,
    uninterruptible: &'static str,
}

const MODERN_STYLE: CastBarStyle = CastBarStyle {
    bar: (BAR_W, BAR_H),
    inset: BAR_INSET,
    holder_background: BORDER_BG,
    border: false,
    spark: true,
    icon: false,
    cast: FILL_CAST,
    channel: FILL_CHANNEL,
    uninterruptible: FILL_UNINTERRUPTIBLE,
};

/// FlareUI's standalone player cast bar: `playerCastbar` 292×26 (Core.lua:281) in a
/// transparent holder INSET 4 larger under the bronze border, `PLAYER_CAST_COLOR`
/// #5C8FC7, `PLAYER_CAST_CHANNEL` #80BFE0 and `CAST_NOINTERRUPT` grey
/// (UnitFrames.lua:75-79,651-661,2225-2234). Icon height equals bar height,
/// flush against its left edge (UnitFrames.lua:53,568-569).
const FOREVER_STYLE: CastBarStyle = CastBarStyle {
    bar: (292.0, 26.0),
    inset: FLARE_INSET,
    holder_background: "0.0,0.0,0.0,0.0",
    border: true,
    spark: false,
    icon: true,
    cast: "0.36,0.56,0.78,1.0",
    channel: "0.50,0.75,0.88,1.0",
    uninterruptible: "0.55,0.55,0.55,1.0",
};

fn cast_bar_style(skin: ActiveSkin) -> &'static CastBarStyle {
    match skin {
        ActiveSkin::Modern => &MODERN_STYLE,
        ActiveSkin::Forever => &FOREVER_STYLE,
    }
}

const SPELL_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const TIMER_COLOR: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct CastingBarState {
    pub visible: bool,
    pub spell_name: String,
    /// The catalog's spell art; absent art gets no substitute texture.
    pub icon_fdid: Option<u32>,
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
            icon_fdid: None,
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
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let style = cast_bar_style(skin);
    let hide = !state.visible;
    let (bar_w, bar_h) = style.bar;
    let icon_width = if style.icon { bar_h } else { 0.0 };
    let holder = (
        bar_w + icon_width + 2.0 * style.inset,
        bar_h + 2.0 * style.inset,
    );
    let icon = if style.icon {
        state
            .icon_fdid
            .map(|fdid| spell_icon(fdid, bar_h, style.inset))
            .unwrap_or_default()
    } else {
        Element::default()
    };
    let fill_w = bar_w * state.progress.clamp(0.0, 1.0);
    let at = hud_layout(ctx).cast_bar.place(holder);
    let border = if style.border {
        flare_border("PlayerCastingBarFrame", holder)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: "PlayerCastingBarFrame",
            width: {holder.0},
            height: {holder.1},
            background_color: style.holder_background,
            hidden: hide,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {bar_background(style, fill_w, bar_fill_color(state, style), state)}
            {icon}
            {border}
        }
    }
}

fn bar_fill_color(state: &CastingBarState, style: &CastBarStyle) -> &'static str {
    if state.is_interrupted {
        FILL_INTERRUPTED
    } else if !state.is_interruptible {
        style.uninterruptible
    } else if state.is_channel {
        style.channel
    } else {
        style.cast
    }
}

fn bar_background(
    style: &CastBarStyle,
    fill_w: f32,
    color: &str,
    state: &CastingBarState,
) -> Element {
    let (bar_w, bar_h) = style.bar;
    if style.icon {
        return forever_bar_background(style, fill_w, color, state);
    }
    let spark = if style.spark {
        spark(fill_w - SPARK_W / 2.0, bar_h)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: "CastingBarBackground",
            width: {bar_w},
            height: {bar_h},
            background_color: BAR_BG,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            {fill_bar(fill_w, bar_h, color)}
            {spark}
            {spell_name_text(&state.spell_name, bar_w)}
            {timer_text(&state.timer_text)}
        }
    }
}

fn spell_icon(fdid: u32, size: f32, inset: f32) -> Element {
    rsx! {
        texture {
            name: "CastingBarIcon",
            width: size,
            height: size,
            texture_fdid: fdid,
            pos_type: "absolute",
            pos_x: inset,
            pos_y: inset,
        }
    }
}

fn forever_bar_background(
    style: &CastBarStyle,
    fill_w: f32,
    color: &str,
    state: &CastingBarState,
) -> Element {
    let (width, height) = style.bar;
    // UnitFrames.lua:49,588-596: 4px text inset and 4px before the timer.
    const TEXT_INSET: f32 = 4.0;
    let timer_x = width - TEXT_INSET - TIMER_W;
    let name_width = timer_x - 2.0 * TEXT_INSET;
    rsx! {
        r#frame {
            name: "CastingBarBackground",
            width,
            height,
            background_color: BAR_BG,
            pos_type: "absolute",
            pos_x: {style.inset + height},
            pos_y: style.inset,
            {fill_bar(fill_w, height, color)}
            {forever_cast_label("CastingBarSpellName", &state.spell_name, (TEXT_INSET, name_width, height), "LEFT")}
            {forever_cast_label("CastingBarTimer", &state.timer_text, (timer_x, TIMER_W, height), "RIGHT")}
        }
    }
}

fn forever_cast_label(
    name: &str,
    text: &str,
    (x, width, height): (f32, f32, f32),
    justify: &str,
) -> Element {
    rsx! {
        fontstring {
            name: {crate::ui::screens::inworld_unit_frames_component::dyn_name(name.to_string())},
            width,
            height,
            text,
            font: "FrizQuadrata",
            font_size: 12.0,
            font_color: SPELL_NAME_COLOR,
            outline: "OUTLINE",
            justify_h: justify,
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
        }
    }
}

fn fill_bar(fill_w: f32, height: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarFill",
            width: {fill_w},
            height: {height},
            background_color: color,
            pos_type: "absolute",
            pos_x: 0.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spark(x: f32, bar_h: f32) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarSpark",
            width: {SPARK_W},
            height: {bar_h + 6.0},
            background_color: SPARK_COLOR,
            pos_type: "absolute",
            pos_x: x,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spell_name_text(name: &str, bar_w: f32) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarSpellName",
            width: {bar_w - TIMER_W},
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
