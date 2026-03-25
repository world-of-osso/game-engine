use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

pub fn framed_title(
    frame: FrameName,
    label: FrameName,
    relative_to: FrameName,
    width: f32,
    text: &str,
) -> Element {
    rsx! {
        panel {
            name: frame,
            width: {width},
            height: 36.0,
            strata: FrameStrata::Fullscreen,
            frame_level: 10.0,
            anchor {
                point: AnchorPoint::Top,
                relative_to: relative_to,
                relative_point: AnchorPoint::Top,
                y: "18",
            }
            fontstring {
                name: label,
                text: {text},
                font_size: 20.0,
                color: "0.96,0.84,0.56,1.0",
                width: {width - 20.0},
                height: 30.0,
                justify_h: "CENTER",
                frame_level: 100.0,
                draw_layer: "OVERLAY",
                anchor {
                    point: AnchorPoint::Center,
                    relative_to: frame,
                    relative_point: AnchorPoint::Center,
                }
            }
        }
    }
}
