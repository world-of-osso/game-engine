use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

pub const GAME_MENU_ROOT: FrameName = FrameName("GameMenuRoot");
pub const GAME_MENU_PANEL: FrameName = FrameName("GameMenuPanel");
pub const GAME_MENU_TITLE: FrameName = FrameName("GameMenuTitle");

pub fn game_menu_screen(_shared: &SharedContext) -> Element {
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.02,0.02,0.03,0.85",
            strata: FrameStrata::Background,
            panel {
                name: GAME_MENU_PANEL,
                width: 340.0,
                height: 420.0,
                strata: FrameStrata::Dialog,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
                fontstring {
                    name: GAME_MENU_TITLE,
                    text: "Game Menu",
                    font_size: 24.0,
                    color: "0.96,0.84,0.56,1.0",
                    width: 300.0,
                    height: 40.0,
                    justify_h: "CENTER",
                    frame_level: 100.0,
                    draw_layer: "OVERLAY",
                    anchor {
                        point: AnchorPoint::Top,
                        relative_to: GAME_MENU_PANEL,
                        relative_point: AnchorPoint::Top,
                        y: 24.0,
                    }
                }
            }
        }
    }
}
