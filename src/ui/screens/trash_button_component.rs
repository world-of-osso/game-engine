use std::fmt::Display;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const DELETE_ICON_FILE: &str = "data/ui/delete-trash-icon-gold.ktx2";

pub const TRASH_BUTTON_ROOT: FrameName = FrameName("TrashButtonRoot");
pub const TRASH_BUTTON: FrameName = FrameName("TrashButton");
pub const TRASH_BUTTON_ICON: FrameName = FrameName("TrashButtonIcon");

/// Distances inward from the enclosing parent's right and bottom edges.
pub struct ButtonPosition {
    pub right: f32,
    pub bottom: f32,
}

pub fn trash_icon_button(
    name: FrameName,
    icon_name: FrameName,
    onclick: impl Display,
    position: ButtonPosition,
) -> Element {
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
            pos_type: "absolute",
            right: position.right,
            bottom: position.bottom,
            {trash_icon_texture(icon_name)}
        }
    }
}

fn trash_icon_texture(icon_name: FrameName) -> Element {
    rsx! {
        texture {
            name: icon_name,
            width: 24.0,
            height: 24.0,
            frame_level: 100.0,
            texture_file: DELETE_ICON_FILE,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
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
            r#frame {
                name: "TrashButtonMount",
                width: 46.0,
                height: 42.0,
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                translate_x: "-50%",
                translate_y: "-50%",
                {trash_icon_button(TRASH_BUTTON, TRASH_BUTTON_ICON, "noop", ButtonPosition { right: 0.0, bottom: 0.0 })}
            }
        }
    }
}
