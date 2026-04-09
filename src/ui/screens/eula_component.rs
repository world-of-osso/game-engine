use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EulaAction {
    Accept,
    Decline,
}

impl fmt::Display for EulaAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept => f.write_str("eula_accept"),
            Self::Decline => f.write_str("eula_decline"),
        }
    }
}

impl EulaAction {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "eula_accept" => Some(Self::Accept),
            "eula_decline" => Some(Self::Decline),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EulaScreenState {
    pub status_text: String,
}

pub const EULA_ROOT: FrameName = FrameName("EulaRoot");
pub const EULA_ACCEPT_BUTTON: FrameName = FrameName("EulaAcceptButton");
pub const EULA_DECLINE_BUTTON: FrameName = FrameName("EulaDeclineButton");
pub const EULA_STATUS_TEXT: FrameName = FrameName("EulaStatusText");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_BODY: FontColor = FontColor::new(0.93, 0.9, 0.82, 1.0);
const COLOR_WARNING: FontColor = FontColor::new(0.93, 0.46, 0.38, 1.0);
const COLOR_SUBTLE: FontColor = FontColor::new(0.72, 0.7, 0.67, 1.0);

const LEGAL_COPY: &str = "World of Osso is an unofficial game client under active development. By continuing, you acknowledge that this software is provided as-is, may change incompatibly, and may disconnect or fail without warning.\n\nDo not use accounts, credentials, or data you cannot afford to lose. Use of the service is at your own risk. Continuing also means you agree to the project Terms of Service and End User License Agreement for this installation.";

fn legal_panel(state: &EulaScreenState) -> Element {
    rsx! {
        r#frame {
            name: "EulaPanel",
            width: 720.0,
            height: 520.0,
            strata: FrameStrata::Dialog,
            background_color: "0.03,0.02,0.02,0.96",
            border: "1.0,0.82,0.0,0.35",
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
            {title_block()}
            {body_block()}
            {status_block(state)}
            {button_row()}
        }
    }
}

fn title_block() -> Element {
    rsx! {
        fontstring {
            name: "EulaTitle",
            width: 620.0,
            height: 28.0,
            text: "Terms of Service & EULA",
            font: GameFont::FrizQuadrata,
            font_size: 24.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-28",
            }
        }
        fontstring {
            name: "EulaSubtitle",
            width: 620.0,
            height: 18.0,
            text: "Acceptance is required before the client can connect.",
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: COLOR_SUBTLE,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("EulaTitle"),
                relative_point: AnchorPoint::Bottom,
                y: "-8",
            }
        }
    }
}

fn body_block() -> Element {
    rsx! {
        fontstring {
            name: "EulaBodyText",
            width: 620.0,
            height: 250.0,
            text: LEGAL_COPY,
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: COLOR_BODY,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("EulaSubtitle"),
                relative_point: AnchorPoint::Bottom,
                y: "-24",
            }
        }
    }
}

fn status_block(state: &EulaScreenState) -> Element {
    rsx! {
        fontstring {
            name: EULA_STATUS_TEXT,
            width: 620.0,
            height: 22.0,
            text: state.status_text.clone(),
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_WARNING,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: FrameName("EulaAcceptButton"),
                relative_point: AnchorPoint::Top,
                y: "18",
            }
        }
    }
}

fn action_button(name: FrameName, text: &str, action: EulaAction, x: f32) -> Element {
    rsx! {
        button {
            name,
            width: 210.0,
            height: 40.0,
            text,
            font_size: 16.0,
            onclick: action,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: {x},
                y: "28",
            }
        }
    }
}

fn button_row() -> Element {
    [
        action_button(EULA_DECLINE_BUTTON, "Decline", EulaAction::Decline, -118.0),
        action_button(EULA_ACCEPT_BUTTON, "Accept", EulaAction::Accept, 118.0),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn eula_screen(shared: &SharedContext) -> Element {
    let state = shared.get::<EulaScreenState>().cloned().unwrap_or_default();
    rsx! {
        r#frame {
            name: EULA_ROOT,
            stretch: true,
            strata: FrameStrata::Dialog,
            background_color: "0.0,0.0,0.0,1.0",
            {legal_panel(&state)}
        }
    }
}
