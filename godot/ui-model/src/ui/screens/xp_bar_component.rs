//! Retail experience bar (docs/specs/xp-bar.md): `MainStatusTrackingBarContainer` with its
//! `ExpStatusBarTemplate` bar. Modern draws Blizzard's atlases; Forever is FlareUI's reskin
//! of the same bar (`Modules/XPBar.lua`): flat fills, a dark track and a bronze
//! `UI-Tooltip-Border`, none of FlareUI's own media.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::hud_layout;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_flare::{
    BAR_BACKGROUND, flare_border_tinted, flare_layer,
};

pub const XP_BAR: &str = "ExperienceBar";

/// The newest `PlayerXpUpdate` and whether the pointer is over the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XpBarState {
    pub xp: u32,
    /// 0 at the level cap.
    pub next_level_xp: u32,
    pub rested_xp: u32,
    pub hovered: bool,
}

/// A skin's container and the bar inside it at (1, 1).
struct XpBarStyle {
    container: (f32, f32),
    bar: (f32, f32),
    font_size: f32,
}

/// Container 571×17 (StatusTrackingBar.xml:4); bar = container − 6 at BOTTOMLEFT (1, 5)
/// (StatusTrackingManagerOverrides.lua:43-51); `TextStatusBarText` FRIZQT 10.
const MODERN_STYLE: XpBarStyle = XpBarStyle {
    container: (571.0, 17.0),
    bar: (565.0, 11.0),
    font_size: 10.0,
};

/// The Forever client's gamepad container, 1192 / 2 × 17, the width the reference
/// screenshot shows (Camelot/StatusTrackingBarConstants.lua:1-3,12); bar = container − 3
/// at BOTTOMLEFT (1, 2) (wowforever StatusTrackingManagerOverrides.lua:46-53, XPBar.lua:36-38).
/// Font: the action bar hotkey font, Friz Quadrata 12 OUTLINE (XPBar.lua:84-90).
const FOREVER_STYLE: XpBarStyle = XpBarStyle {
    container: (596.0, 17.0),
    bar: (593.0, 14.0),
    font_size: 12.0,
};

const BAR_INSET: f32 = 1.0;
/// `ExhaustionTick` 10×14, centred `yOffset` 2 above the bar's centre (ExpBar.xml:20-25).
const TICK: (f32, f32) = (10.0, 14.0);
const TICK_RAISE: f32 = 2.0;
/// `COLOR_XP`, `COLOR_RESTED`, `RESTED_ALPHA` (XPBar.lua:34,42-43).
const FLARE_XP: &str = "0.58,0.0,0.55,1.0";
const FLARE_RESTED: &str = "0.0,0.39,0.88,1.0";
const FLARE_PREDICTION: &str = "0.0,0.39,0.88,0.4";
/// `BORDER_COLOR`, `BORDER_SIZE`, `BORDER_OUTSET` (XPBar.lua:30-32).
const FLARE_BORDER_COLOR: &str = "0.8,0.6,0.34,1.0";
const FLARE_BORDER_EDGE: f32 = 16.0;
const FLARE_BORDER_OUTSET: f32 = 4.0;

/// Widths of the bar's parts for one `PlayerXpUpdate`.
struct Extents {
    fill: f32,
    /// `ExhaustionLevelFillBar`: from the bar's left to the end of the rested pool.
    prediction: Option<f32>,
    /// `ExhaustionTick`'s centre, from the bar's left.
    tick: Option<f32>,
}

/// `ExhaustionTickMixin:UpdateTickPosition` (ExpBar.lua:135-157): no rested pool hides
/// both; the pip also hides within 1% of either edge (`hideAtBarEdge`) and the overlay
/// when the pool ends past the bar.
fn extents(state: &XpBarState, bar_width: f32) -> Extents {
    let next = state.next_level_xp as f32;
    if state.next_level_xp == 0 {
        return Extents {
            fill: 0.0,
            prediction: None,
            tick: None,
        };
    }
    let fill = bar_width * (state.xp as f32 / next).clamp(0.0, 1.0);
    if state.rested_xp == 0 {
        return Extents {
            fill,
            prediction: None,
            tick: None,
        };
    }
    let ratio = (state.xp as f32 + state.rested_xp as f32) / next;
    let end = ratio * bar_width;
    let inside = end <= bar_width;
    Extents {
        fill,
        prediction: inside.then_some(end),
        tick: (inside && (0.01..=0.99).contains(&ratio)).then_some(end),
    }
}

pub fn xp_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<XpBarState>()
        .expect("XpBarState must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let style = match skin {
        ActiveSkin::Modern => &MODERN_STYLE,
        ActiveSkin::Forever => &FOREVER_STYLE,
    };
    let extents = extents(state, style.bar.0);
    let (width, height) = style.container;
    let at = hud_layout(ctx).xp_bar.place(style.container);
    // Retail shows no experience bar at the level cap (`CanShowExperienceBar`,
    // StatusTrackingManagerOverrides.lua:30-31).
    let capped = state.next_level_xp == 0;
    let parts = match skin {
        ActiveSkin::Modern => modern_parts(style, state, &extents),
        ActiveSkin::Forever => forever_parts(style, state, &extents),
    };
    rsx! {
        r#frame {
            name: "ExperienceBar",
            width,
            height,
            hidden: capped,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {parts}
            {hover_text(style, state)}
        }
    }
}

/// Background, rested overlay under the fill, the frame art over the container and the
/// pip (StatusTrackingBarTemplate.xml:20, ExpBar.xml:11-36, StatusTrackingBar.xml:7).
fn modern_parts(style: &XpBarStyle, state: &XpBarState, extents: &Extents) -> Element {
    let (bar_w, bar_h) = style.bar;
    let (width, height) = style.container;
    // ExpBarOverrides.lua:2,6,14-15.
    let fill_atlas = if state.rested_xp > 0 {
        "UI-HUD-ExperienceBar-Fill-Rested"
    } else {
        "UI-HUD-ExperienceBar-Fill-Experience"
    };
    let no_prediction = extents.prediction.is_none();
    let no_tick = extents.tick.is_none();
    let tick_x = BAR_INSET + extents.tick.unwrap_or(0.0) - TICK.0 / 2.0;
    let tick_y = BAR_INSET + bar_h / 2.0 - TICK_RAISE - TICK.1 / 2.0;
    rsx! {
        texture {
            name: "ExperienceBarBackground",
            width: bar_w,
            height: bar_h,
            texture_atlas: "UI-HUD-ExperienceBar-Background",
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        texture {
            name: "ExperienceBarPrediction",
            width: {extents.prediction.unwrap_or(0.0)},
            height: bar_h,
            hidden: no_prediction,
            texture_atlas: "UI-HUD-ExperienceBar-Fill-Prediction",
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        texture {
            name: "ExperienceBarFill",
            width: {extents.fill},
            height: bar_h,
            texture_atlas: fill_atlas,
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        texture {
            name: "ExperienceBarFrame",
            width,
            height,
            texture_atlas: "UI-HUD-ExperienceBar-Frame",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
        texture {
            name: "ExperienceBarTick",
            width: {TICK.0},
            height: {TICK.1},
            hidden: no_tick,
            texture_atlas: "UI-HUD-ExperienceBar-Frame-Pip",
            pos_type: "absolute",
            pos_x: tick_x,
            pos_y: tick_y,
        }
    }
}

/// FlareUI's reskin: dark track, the rested pool at 40% under a flat fill in Blizzard's XP
/// purple or rested blue, then the bronze border `BORDER_OUTSET` outside the fill
/// (XPBar.lua:97-103,120-129,141-150). Frame art and dividers are hidden there.
fn forever_parts(style: &XpBarStyle, state: &XpBarState, extents: &Extents) -> Element {
    let (bar_w, bar_h) = style.bar;
    let fill_color = if state.rested_xp > 0 {
        FLARE_RESTED
    } else {
        FLARE_XP
    };
    let no_prediction = extents.prediction.is_none();
    let border_origin = BAR_INSET - FLARE_BORDER_OUTSET;
    let border_size = (
        bar_w + 2.0 * FLARE_BORDER_OUTSET,
        bar_h + 2.0 * FLARE_BORDER_OUTSET,
    );
    let border = flare_border_tinted(XP_BAR, border_size, FLARE_BORDER_EDGE, FLARE_BORDER_COLOR);
    rsx! {
        r#frame {
            name: "ExperienceBarBackground",
            width: bar_w,
            height: bar_h,
            background_color: BAR_BACKGROUND,
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        r#frame {
            name: "ExperienceBarPrediction",
            width: {extents.prediction.unwrap_or(0.0)},
            height: bar_h,
            hidden: no_prediction,
            background_color: FLARE_PREDICTION,
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        r#frame {
            name: "ExperienceBarFill",
            width: {extents.fill},
            height: bar_h,
            background_color: fill_color,
            pos_type: "absolute",
            pos_x: BAR_INSET,
            pos_y: BAR_INSET,
        }
        {flare_layer(
            "ExperienceBarBorder".into(),
            (border_origin, border_origin, border_size.0, border_size.1),
            border,
        )}
    }
}

/// `OverlayFrame.Text`, shown while hovered (ExpBar.lua:72-75,90-93), centred 1 above the
/// bar's centre (StatusTrackingBarTemplate.xml:37-41), in a frame built after the fill and
/// border so they do not cover it. `XP_STATUS_BAR_TEXT` is "XP: %d/%d" (GlobalStrings).
fn hover_text(style: &XpBarStyle, state: &XpBarState) -> Element {
    let (bar_w, bar_h) = style.bar;
    let text = format!("XP: {}/{}", state.xp, state.next_level_xp);
    let hidden = !state.hovered;
    let text_h = style.font_size + 2.0;
    let label = rsx! {
        fontstring {
            name: "ExperienceBarText",
            width: bar_w,
            height: text_h,
            hidden,
            text: {text.as_str()},
            font: "FrizQuadrata",
            font_size: {style.font_size},
            font_color: "1.0,1.0,1.0,1.0",
            outline: "OUTLINE",
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: {(bar_h - text_h) / 2.0 - 1.0},
        }
    };
    flare_layer(
        "ExperienceBarOverlay".into(),
        (BAR_INSET, BAR_INSET, bar_w, bar_h),
        label,
    )
}
