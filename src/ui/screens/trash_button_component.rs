use std::fmt::Display;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const DELETE_ICON_FILE: &str = "data/ui/delete-trash-icon-gold.ktx2";

pub const TRASH_BUTTON_ROOT: FrameName = FrameName("TrashButtonRoot");
pub const TRASH_BUTTON: FrameName = FrameName("TrashButton");
pub const TRASH_BUTTON_ICON: FrameName = FrameName("TrashButtonIcon");

pub struct ButtonAnchor {
    pub point: AnchorPoint,
    pub relative_to: Option<FrameName>,
    pub relative_point: AnchorPoint,
    pub x: f32,
    pub y: f32,
}

pub fn trash_icon_button(
    name: FrameName,
    icon_name: FrameName,
    onclick: impl Display,
    anchor: ButtonAnchor,
) -> Element {
    if let Some(relative_to) = anchor.relative_to {
        rsx! {
            button {
                name,
                width: 46.0,
                height: 42.0,
                text: "",
                font_size: 14.0,
                onclick,
                button_atlas_up: BUTTON_ATLAS_UP,
                button_atlas_pressed: BUTTON_ATLAS_PRESSED,
                button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
                button_atlas_disabled: BUTTON_ATLAS_DISABLED,
                anchor {
                    point: anchor.point,
                    relative_to,
                    relative_point: anchor.relative_point,
                    x: {anchor.x.to_string()},
                    y: {anchor.y.to_string()},
                }
                texture {
                    name: icon_name,
                    width: 24.0,
                    height: 24.0,
                    frame_level: 100.0,
                    texture_file: DELETE_ICON_FILE,
                    anchor {
                        point: AnchorPoint::Center,
                        relative_to: name,
                        relative_point: AnchorPoint::Center,
                    }
                }
            }
        }
    } else {
        rsx! {
            button {
                name,
                width: 46.0,
                height: 42.0,
                text: "",
                font_size: 14.0,
                onclick,
                button_atlas_up: BUTTON_ATLAS_UP,
                button_atlas_pressed: BUTTON_ATLAS_PRESSED,
                button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
                button_atlas_disabled: BUTTON_ATLAS_DISABLED,
                anchor {
                    point: anchor.point,
                    relative_point: anchor.relative_point,
                    x: {anchor.x.to_string()},
                    y: {anchor.y.to_string()},
                }
                texture {
                    name: icon_name,
                    width: 24.0,
                    height: 24.0,
                    frame_level: 100.0,
                    texture_file: DELETE_ICON_FILE,
                    anchor {
                        point: AnchorPoint::Center,
                        relative_to: name,
                        relative_point: AnchorPoint::Center,
                    }
                }
            }
        }
    }
}

pub fn trash_button_screen(_shared: &SharedContext) -> Element {
    rsx! {
        r#frame {
            name: TRASH_BUTTON_ROOT,
            stretch: true,
            background_color: "0.02,0.02,0.03,1.0",
            strata: FrameStrata::Background,
            {
                trash_icon_button(
                    TRASH_BUTTON,
                    TRASH_BUTTON_ICON,
                    "noop",
                    ButtonAnchor {
                        point: AnchorPoint::Center,
                        relative_to: None,
                        relative_point: AnchorPoint::Center,
                        x: 0.0,
                        y: 0.0,
                    },
                )
            }
        }
    }
}
