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
const DELETE_ICON_FILE: &str = "output/imagegen/delete-trash-icon.png";
const DELETE_ICON_TINT: &str = "1.0,0.82,0.0,1.0";

pub const TRASH_BUTTON_ROOT: FrameName = FrameName("TrashButtonRoot");
pub const TRASH_BUTTON: FrameName = FrameName("TrashButton");
pub const TRASH_BUTTON_ICON: FrameName = FrameName("TrashButtonIcon");

#[allow(clippy::too_many_arguments)]
pub fn trash_icon_button(
    name: FrameName,
    icon_name: FrameName,
    onclick: impl Display,
    point: AnchorPoint,
    relative_to: Option<FrameName>,
    relative_point: AnchorPoint,
    x: f32,
    y: f32,
) -> Element {
    if let Some(relative_to) = relative_to {
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
                    point,
                    relative_to,
                    relative_point,
                    x: {x.to_string()},
                    y: {y.to_string()},
                }
            texture {
                name: icon_name,
                width: 24.0,
                height: 24.0,
                frame_level: 100.0,
                texture_file: DELETE_ICON_FILE,
                vertex_color: DELETE_ICON_TINT,
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
                    point,
                    relative_point,
                    x: {x.to_string()},
                    y: {y.to_string()},
                }
            texture {
                name: icon_name,
                width: 24.0,
                height: 24.0,
                frame_level: 100.0,
                texture_file: DELETE_ICON_FILE,
                vertex_color: DELETE_ICON_TINT,
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
            {trash_icon_button(
                TRASH_BUTTON,
                TRASH_BUTTON_ICON,
                "noop",
                AnchorPoint::Center,
                None,
                AnchorPoint::Center,
                0.0,
                0.0,
            )}
        }
    }
}
