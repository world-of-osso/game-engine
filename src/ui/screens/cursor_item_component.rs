//! The held item's icon, centered on the logical UI pointer above other frames.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

pub const CURSOR_ICON_NAME: &str = "CursorItemIcon";
pub const CURSOR_ICON_SIZE: f32 = 32.0;

#[derive(Clone, Default, PartialEq)]
pub struct CursorItemFrameState {
    pub icon_fdid: Option<u32>,
    /// Pointer position in logical UI coordinates, not the icon's top-left.
    pub position: [f32; 2],
}

pub fn cursor_item_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CursorItemFrameState>()
        .expect("CursorItemFrameState must be in SharedContext");
    let Some(icon_fdid) = state.icon_fdid else {
        return Element::default();
    };
    let half = CURSOR_ICON_SIZE * 0.5;
    rsx! {
        texture {
            name: {FrameName(CURSOR_ICON_NAME)},
            width: CURSOR_ICON_SIZE,
            height: CURSOR_ICON_SIZE,
            texture_fdid: icon_fdid,
            strata: FrameStrata::Tooltip,
            mouse_enabled: false,
            pos_type: "absolute",
            left: {state.position[0] - half},
            top: {state.position[1] - half},
        }
    }
}
