#[path = "casting_bar_feedback.rs"]
mod feedback;
pub use feedback::{CastFeedback, apply_casting_bar_feedback_postsetup};

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::{Placement, hud_layout};
use crate::ui::screens::inworld_unit_frames_component::CAST_DOCK_W;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_flare::{
    FLARE_INSET, FLARE_TARGET, flare_border_frame, flare_layer,
};
use crate::ui::screens::inworld_unit_frames_component::{
    DynName, InWorldUnitFramesState, UNIT_FRAME_H, UNIT_FRAME_W, dyn_name,
    modern_target_cast_offset,
};
use crate::unit_frame_style::styled_frame;

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
const SPARK_COLOR: &str = "1.0,1.0,1.0,0.8";
/// How a skin draws the player cast bar: bar size, the holder's inset around it, the
/// holder's background and the fill colours.
struct CastBarStyle {
    root: &'static str,
    prefix: &'static str,
    /// Retail target icons sit outside the track, without a holder inset.
    retail_target: bool,
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
    root: "PlayerCastingBarFrame",
    prefix: "CastingBar",
    retail_target: false,
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
    root: "PlayerCastingBarFrame",
    prefix: "CastingBar",
    retail_target: false,
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

// TargetSpellBarTemplate overrides SmallCastingBarFrameTemplate's height:
// Blizzard_UnitFrame/Mainline/TargetFrame.xml:529-531.
const MODERN_TARGET_STYLE: CastBarStyle = CastBarStyle {
    root: "TargetFrameSpellBar",
    prefix: "TargetCastingBar",
    retail_target: true,
    bar: (150.0, 10.0),
    inset: 0.0,
    holder_background: "0.0,0.0,0.0,0.0",
    border: false,
    spark: true,
    icon: true,
    cast: FILL_CAST,
    channel: FILL_CHANNEL,
    uninterruptible: FILL_UNINTERRUPTIBLE,
};

// FlareUI Core.lua:289-291, UnitFrames.lua:51-53,541-581,74-76:
// 240-wide holder, 4 inset, 16 icon, 216 track, bronze casts/blue channels.
const FOREVER_TARGET_STYLE: CastBarStyle = CastBarStyle {
    root: "TargetFrameSpellBar",
    prefix: "TargetCastingBar",
    retail_target: false,
    bar: (216.0, 16.0),
    inset: FLARE_INSET,
    holder_background: "0.0,0.0,0.0,0.0",
    border: true,
    spark: false,
    icon: true,
    cast: "0.80,0.60,0.36,1.0",
    channel: "0.30,0.65,0.90,1.0",
    uninterruptible: "0.55,0.55,0.55,1.0",
};

impl CastBarStyle {
    fn name(&self, suffix: &str) -> DynName {
        dyn_name(format!("{}{suffix}", self.prefix))
    }

    fn holder_size(&self) -> (f32, f32) {
        let icon_width = if self.icon && !self.retail_target {
            self.bar.1
        } else {
            0.0
        };
        (
            self.bar.0 + icon_width + 2.0 * self.inset,
            self.bar.1 + 2.0 * self.inset,
        )
    }
}

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
    /// `FadeOutAnim` / `HoldFadeOutAnim` on the whole bar.
    pub alpha: f32,
    /// Player-only animation clock; target bars retain their existing presentation.
    pub player_feedback: Option<CastFeedback>,
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
            alpha: 1.0,
            player_feedback: None,
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
    let at = hud_layout(ctx).cast_bar.place(style.holder_size());
    cast_bar_frame(state, style, &at)
}

pub(super) fn target_cast_bar_frame(
    ctx: &SharedContext,
    units: &InWorldUnitFramesState,
) -> Element {
    if !units.show_target_frame || units.target.is_none() {
        return Element::default();
    }
    let Some(state) = units.target_cast.as_ref().filter(|state| state.visible) else {
        return Element::default();
    };
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let (style, parent_size, offset) = match skin {
        ActiveSkin::Modern => (
            &MODERN_TARGET_STYLE,
            (UNIT_FRAME_W, UNIT_FRAME_H),
            modern_target_cast_offset(units),
        ),
        // `LayoutCastBar` BOTTOM: the backdrop starts at the frame's bottom edge
        // (UnitFrames.lua:558-574).
        ActiveSkin::Forever => (
            &FOREVER_TARGET_STYLE,
            FLARE_TARGET.size,
            (0.0, FLARE_TARGET.size.1),
        ),
    };
    // A child of the target frame: it takes the target's size and text settings.
    let layout = hud_layout(ctx);
    let scale = layout.target_style.scale;
    let anchor = layout.target.offset_from_top_left(
        (parent_size.0 * scale, parent_size.1 * scale),
        (offset.0 * scale, offset.1 * scale),
    );
    let bar = cast_bar_frame(state, style, &anchor.place(style.holder_size()));
    styled_frame(bar, &layout.target_style, &anchor)
}

fn cast_bar_frame(state: &CastingBarState, style: &CastBarStyle, at: &Placement) -> Element {
    let hide = !state.visible;
    let (bar_w, _) = style.bar;
    let holder = style.holder_size();
    let icon = if style.icon {
        state
            .icon_fdid
            .map(|fdid| spell_icon(fdid, style))
            .unwrap_or_default()
    } else {
        Element::default()
    };
    let fill_w = bar_w * state.progress.clamp(0.0, 1.0);
    let (shake_x, shake_y) = state
        .player_feedback
        .map_or((0.0, 0.0), CastFeedback::shake_offset);
    let border = if style.border {
        flare_border_frame(style.root, holder)
    } else {
        Element::default()
    };
    let labels = if style.icon && !style.retail_target {
        forever_cast_labels(style, state)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: {dyn_name(style.root.to_string())},
            width: {holder.0},
            height: {holder.1},
            background_color: style.holder_background,
            hidden: hide,
            alpha: {state.alpha},
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left + shake_x},
            margin_top: {at.margin_top + shake_y},
            {bar_background(style, fill_w, bar_fill_color(state, style), state)}
            {icon}
            {retail_target_art(style, state)}
            {border}
            {labels}
        }
    }
}

// SmallCastingBarFrameTemplate: icon 20x20 at (-22,0), shield 29x33 at
// (-27,-4), border outside the 150x10 track (CastingBarFrame.xml:415-436).
fn retail_target_art(style: &CastBarStyle, state: &CastingBarState) -> Element {
    if !style.retail_target {
        return Element::default();
    }
    let hide_shield = state.is_interruptible;
    rsx! {
        texture {
            name: {style.name("Border")}, width: {style.bar.0 + 2.0}, height: {style.bar.1 + 4.0},
            texture_atlas: "ui-castingbar-frame",
            pos_type: "absolute", pos_x: -1.0, pos_y: -2.0,
        }
        texture {
            name: {style.name("Shield")}, width: 29.0, height: 33.0,
            hidden: hide_shield, texture_atlas: "ui-castingbar-shield",
            pos_type: "absolute", pos_x: -27.0, pos_y: -4.0,
        }
    }
}

fn retail_target_spell_name(style: &CastBarStyle, text: &str) -> Element {
    // CastingBarFrame.xml:449-454; remaining timer is the existing client's addition.
    rsx! { fontstring {
        name: {style.name("SpellName")}, width: {style.bar.0}, height: 16.0,
        text, font_size: 10.0, font_color: SPELL_NAME_COLOR, justify_h: "CENTER",
        pos_type: "absolute", pos_x: 0.0, pos_y: 8.0,
    } }
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
    if state.player_feedback.is_some() {
        return player_bar_background(style, fill_w, color, state);
    }
    if style.icon && !style.retail_target {
        return forever_bar_background(style, fill_w, color);
    }
    let spark = if style.spark {
        spark(fill_w, style)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: {style.name("Background")},
            width: {bar_w},
            height: {bar_h},
            background_color: BAR_BG,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            {fill_bar(style, fill_w, bar_h, color)}
            {spark}
            {spell_name_text(style, &state.spell_name)}
            {timer_text(style, &state.timer_text)}
        }
    }
}

fn player_bar_background(
    style: &CastBarStyle,
    fill_w: f32,
    color: &str,
    state: &CastingBarState,
) -> Element {
    let (width, height) = style.bar;
    let forever = style.icon;
    let x = if forever {
        style.inset + height
    } else {
        style.inset
    };
    let background = if forever {
        rsx! { r#frame { width, height, background_color: BAR_BG } }
    } else {
        rsx! { texture {
            name: {style.name("BackgroundArt")}, width, height,
            texture_atlas: "ui-castingbar-background",
        } }
    };
    let labels = if forever {
        Element::default()
    } else {
        spell_name_text(style, &state.spell_name)
            .into_iter()
            .chain(timer_text(style, &state.timer_text))
            .collect()
    };
    rsx! { r#frame {
        name: {style.name("Background")}, width, height,
        pos_type: "absolute", pos_x: x, pos_y: style.inset,
        {background}
        {player_fill(style, state, fill_w, color)}
        {feedback::feedback_art(style, state)}
        {labels}
        {feedback::player_spark(style, state, fill_w)}
    } }
}

fn player_fill(style: &CastBarStyle, state: &CastingBarState, width: f32, color: &str) -> Element {
    if style.icon {
        return fill_bar(style, width, style.bar.1, color);
    }
    let full = matches!(state.player_feedback, Some(CastFeedback::Finished(_)));
    let atlas = if state.is_interrupted {
        "ui-castingbar-interrupted"
    } else if !state.is_interruptible {
        "ui-castingbar-uninterruptable"
    } else if state.is_channel {
        if full {
            "ui-castingbar-full-channel"
        } else {
            "ui-castingbar-filling-channel"
        }
    } else if full {
        "ui-castingbar-full-standard"
    } else {
        "ui-castingbar-filling-standard"
    };
    // Crop the fill; do not compress a whole atlas into the partial width.
    let right = state.progress.clamp(0.0, 1.0);
    let coords = format!("0.0,{right},0.0,1.0");
    rsx! { texture {
        name: {style.name("Fill")}, width, height: {style.bar.1},
        texture_atlas: atlas, tex_coords: {coords.as_str()},
        pos_type: "absolute", pos_x: 0.0, pos_y: 0.0,
    } }
}

fn spell_icon(fdid: u32, style: &CastBarStyle) -> Element {
    let (size, x, y) = if style.retail_target {
        (20.0, -22.0, 0.0)
    } else {
        (style.bar.1, style.inset, style.inset)
    };
    rsx! {
        texture {
            name: {style.name("Icon")},
            width: size,
            height: size,
            texture_fdid: fdid,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

fn forever_bar_background(style: &CastBarStyle, fill_w: f32, color: &str) -> Element {
    let (width, height) = style.bar;
    rsx! {
        r#frame {
            name: {style.name("Background")},
            width,
            height,
            background_color: BAR_BG,
            pos_type: "absolute",
            pos_x: {style.inset + height},
            pos_y: style.inset,
            {fill_bar(style, fill_w, height, color)}
        }
    }
}

/// `CastingBarOverlay`: spell name and remaining time over the bar, above its border.
fn forever_cast_labels(style: &CastBarStyle, state: &CastingBarState) -> Element {
    let (width, height) = style.bar;
    // UnitFrames.lua:49,588-596: 4px text inset and 4px before the timer.
    const TEXT_INSET: f32 = 4.0;
    let timer_x = width - TEXT_INSET - TIMER_W;
    let name_width = timer_x - 2.0 * TEXT_INSET;
    let labels = rsx! {
        {forever_cast_label(&format!("{}SpellName", style.prefix), &state.spell_name, (TEXT_INSET, name_width, height), "LEFT")}
        {forever_cast_label(&format!("{}Timer", style.prefix), &state.timer_text, (timer_x, TIMER_W, height), "RIGHT")}
    };
    flare_layer(
        format!("{}Overlay", style.prefix),
        (style.inset + height, style.inset, width, height),
        labels,
    )
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

fn fill_bar(style: &CastBarStyle, fill_w: f32, height: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: {style.name("Fill")},
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

fn spark(fill_w: f32, style: &CastBarStyle) -> Element {
    let width = if style.retail_target { 6.0 } else { SPARK_W };
    let x = fill_w - width / 2.0;
    let bar_h = style.bar.1;
    rsx! {
        r#frame {
            name: {style.name("Spark")},
            width,
            height: {bar_h + 6.0},
            background_color: SPARK_COLOR,
            pos_type: "absolute",
            pos_x: x,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spell_name_text(style: &CastBarStyle, name: &str) -> Element {
    if style.retail_target {
        return retail_target_spell_name(style, name);
    }
    let bar_w = style.bar.0;
    rsx! {
        fontstring {
            name: {style.name("SpellName")},
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

fn timer_text(style: &CastBarStyle, timer: &str) -> Element {
    rsx! {
        fontstring {
            name: {style.name("Timer")},
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
