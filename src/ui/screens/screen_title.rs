use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

/// A title attached to the top of its enclosing RSX parent.
pub fn framed_title(frame: FrameName, label: FrameName, width: f32, text: &str) -> Element {
    rsx! {
        panel {
            name: frame,
            width: {width},
            height: 36.0,
            strata: FrameStrata::Fullscreen,
            frame_level: 10.0,
            pos_type: "absolute",
            left: "50%",
            top: -18.0,
            translate_x: "-50%",
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
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                translate_x: "-50%",
                translate_y: "-50%",
            }
        }
    }
}
