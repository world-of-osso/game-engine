//! Retail `MainStatusTrackingBarContainer` holding the experience bar
//! (Blizzard_ActionBar `StatusTrackingBar.xml`, `ExpBar.xml`, `ExpBar.lua`).

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::experience_data::{
    ExperienceState, bar_text, fill_fraction, is_rested, rested_end_fraction,
};
use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;

pub const CONTAINER_NAME: FrameName = FrameName("MainStatusTrackingBarContainer");
pub const EXP_BAR_NAME: FrameName = FrameName("MainStatusTrackingBarContainerExpBar");

/// `StatusTrackingBarContainerTemplate` size (StatusTrackingBar.xml:4).
pub const CONTAINER_W: f32 = 571.0;
pub const CONTAINER_H: f32 = 17.0;
/// `InitializeBars`: container size minus 6, at BOTTOMLEFT (1, 5)
/// (StatusTrackingManagerOverrides.lua:43-51).
pub const BAR_W: f32 = CONTAINER_W - 6.0;
pub const BAR_H: f32 = CONTAINER_H - 6.0;
const BAR_X: f32 = 1.0;
const BAR_Y: f32 = CONTAINER_H - 5.0 - BAR_H;
/// `ExhaustionTick` button size and `yOffset` (ExpBar.xml:20-25).
const TICK_W: f32 = 10.0;
const TICK_H: f32 = 14.0;
const TICK_Y_OFFSET: f32 = 2.0;
/// `TextStatusBarText` = `SystemFont_Outline_Small` (FRIZQT 10, outline), white.
const TEXT_FONT: &str = "FrizQuadrata";
const TEXT_SIZE: f32 = 10.0;
const TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";

/// UiTextureAtlas 1988 `interface/hud/uiexperiencebar2x.blp` (2048×256).
const fn exp_bar_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_615_784,
        atlas: (2048.0, 256.0),
        rect,
    }
}

/// `UI-HUD-ExperienceBar-Frame-2x` (18383).
const FRAME_ART: AtlasArt = exp_bar_art((1.0, 1143.0, 201.0, 235.0));
/// `UI-HUD-ExperienceBar-Background-2x` (18371).
const BACKGROUND_ART: AtlasArt = exp_bar_art((1.0, 1127.0, 21.0, 39.0));
/// `UI-HUD-ExperienceBar-Fill-Experience-2x` (18373), the unrested bar.
const FILL_XP_ART: AtlasArt = exp_bar_art((1.0, 1127.0, 41.0, 59.0));
/// `UI-HUD-ExperienceBar-Fill-Rested-2x` (18382).
const FILL_RESTED_ART: AtlasArt = exp_bar_art((1.0, 1127.0, 101.0, 119.0));
/// `UI-HUD-ExperienceBar-Fill-Prediction-2x` (18375), `ExhaustionLevelFillBar`.
const PREDICTION_ART: AtlasArt = exp_bar_art((1.0, 1127.0, 81.0, 99.0));
/// `UI-HUD-ExperienceBar-Frame-Pip-2x` (18384), the exhaustion tick.
const PIP_ART: AtlasArt = exp_bar_art((1171.0, 1191.0, 201.0, 229.0));

/// What the XP bar draws for one `PlayerXpUpdate`.
#[derive(Clone, Debug, PartialEq)]
pub struct XpBarView {
    /// Status bar value, 0..=1.
    pub fill: f32,
    /// Rested bar texture instead of the normal one (`UpdateExhaustionColor`).
    pub rested: bool,
    /// `ExhaustionLevelFillBar` width fraction, when shown.
    pub prediction: Option<f32>,
    /// Exhaustion tick centre as a fraction of the bar width, when shown.
    pub tick: Option<f32>,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusTrackingBarState {
    /// Container shown: it holds a bar, or HUD edit mode is on (`UpdateShownState`).
    pub shown: bool,
    pub xp: Option<XpBarView>,
    /// Bar text shown while the mouse is over the bar (`ShowText` from `OnEnter`).
    pub text_shown: bool,
}

impl StatusTrackingBarState {
    pub fn new(experience: &ExperienceState, edit_mode: bool, hovered: bool) -> Self {
        let xp = experience.leveling().map(|update| {
            let rested_end = rested_end_fraction(&update);
            XpBarView {
                fill: fill_fraction(&update),
                rested: is_rested(&update),
                // Hidden once the pool reaches past the bar's right edge.
                prediction: rested_end.filter(|end| *end <= 1.0),
                // `hideAtBarEdge`: no pip within 1% of either edge.
                tick: rested_end.filter(|end| (0.01..=0.99).contains(end)),
                text: bar_text(&update),
            }
        });
        Self {
            shown: xp.is_some() || edit_mode,
            text_shown: hovered && xp.is_some(),
            xp,
        }
    }
}

pub fn status_tracking_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<StatusTrackingBarState>()
        .expect("StatusTrackingBarState must be in SharedContext");
    let hide = !state.shown;
    let bar = state
        .xp
        .as_ref()
        .map(|xp| exp_bar(xp, state.text_shown))
        .unwrap_or_default();
    rsx! {
        r#frame {
            name: CONTAINER_NAME,
            width: CONTAINER_W,
            height: CONTAINER_H,
            hidden: hide,
            pos_type: "absolute",
            left: "50%",
            bottom: 0.0,
            translate_x: "-50%",
            {bar}
            {atlas_texture(FrameName("MainStatusTrackingBarContainerBarFrameTexture"), &FRAME_ART, (0.0, 0.0, CONTAINER_W, CONTAINER_H), 1.0, false)}
            {exhaustion_tick(xp_tick(state))}
        }
    }
}

fn xp_tick(state: &StatusTrackingBarState) -> Option<f32> {
    state.xp.as_ref().and_then(|xp| xp.tick)
}

fn exp_bar(xp: &XpBarView, text_shown: bool) -> Element {
    let fill_art = if xp.rested {
        &FILL_RESTED_ART
    } else {
        &FILL_XP_ART
    };
    let prediction = xp.prediction.unwrap_or(0.0);
    let text_hidden = !text_shown;
    rsx! {
        r#frame {
            name: EXP_BAR_NAME,
            width: BAR_W,
            height: BAR_H,
            mouse_enabled: true,
            pos_type: "absolute",
            pos_x: BAR_X,
            pos_y: BAR_Y,
            {atlas_texture(FrameName("MainStatusTrackingBarContainerExpBarBackground"), &BACKGROUND_ART, (0.0, 0.0, BAR_W, BAR_H), 1.0, false)}
            {atlas_texture(FrameName("MainStatusTrackingBarContainerExhaustionLevelFillBar"), &PREDICTION_ART, (0.0, 0.0, BAR_W * prediction, BAR_H), prediction, xp.prediction.is_none())}
            {atlas_texture(FrameName("MainStatusTrackingBarContainerExpBarFill"), fill_art, (0.0, 0.0, BAR_W * xp.fill, BAR_H), xp.fill, xp.fill <= 0.0)}
            fontstring {
                name: FrameName("MainStatusTrackingBarContainerExpBarText"),
                width: BAR_W,
                height: BAR_H,
                text: xp.text.as_str(),
                font: TEXT_FONT,
                font_size: TEXT_SIZE,
                font_color: TEXT_COLOR,
                outline: "OUTLINE",
                justify_h: "CENTER",
                hidden: text_hidden,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: -1.0,
            }
        }
    }
}

/// `ExhaustionTick`: CENTER at the bar's LEFT plus the rested end, `yOffset` up.
fn exhaustion_tick(tick: Option<f32>) -> Element {
    let center_x = BAR_X + BAR_W * tick.unwrap_or(0.0);
    let center_y = BAR_Y + BAR_H / 2.0 - TICK_Y_OFFSET;
    let rect = (
        center_x - TICK_W / 2.0,
        center_y - TICK_H / 2.0,
        TICK_W,
        TICK_H,
    );
    atlas_texture(
        FrameName("MainStatusTrackingBarContainerExhaustionTick"),
        &PIP_ART,
        rect,
        1.0,
        tick.is_none(),
    )
}

/// The leftmost `fraction` of an atlas crop over `rect` (`SetTexCoord(0, fraction, 0, 1)`).
fn atlas_texture(
    name: FrameName,
    art: &AtlasArt,
    (x, y, width, height): (f32, f32, f32, f32),
    fraction: f32,
    hidden: bool,
) -> Element {
    let coords = art.tex_coords(fraction);
    rsx! {
        texture {
            name,
            width,
            height,
            hidden,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

#[cfg(test)]
#[path = "status_tracking_bar_component_tests.rs"]
mod tests;
