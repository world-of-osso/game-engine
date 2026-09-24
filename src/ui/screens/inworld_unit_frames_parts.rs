use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{
    BAR_BG, BAR_X, BORDER, DARK_BACKING, DynName, METAL_BORDER, UNIT_FONT, UNIT_FONT_SIZE,
    VALUE_TEXT, dyn_name,
};

pub(super) struct BarSpec<'a> {
    pub(super) name: String,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) fraction: f32,
    pub(super) color: &'a str,
    pub(super) text: &'a str,
    pub(super) hidden: bool,
}

/// Cluster frame: thin metal border around a dark backing, anchored from the screen's
/// bottom centre (`left` is the offset of the frame's left edge from the centre line).
pub(super) fn bordered_root(
    name: DynName,
    (width, height): (f32, f32),
    (left, bottom): (f32, f32),
    hidden: bool,
    content: Element,
) -> Element {
    let backing = dyn_name(format!("{}Backing", name.0));
    rsx! {
        r#frame {
            name,
            width,
            height,
            hidden,
            mouse_enabled: true,
            background_color: METAL_BORDER,
            pos_type: "absolute",
            left: "50%",
            margin_left: left,
            bottom,
            r#frame {
                name: backing,
                width: {width - 2.0 * BORDER},
                height: {height - 2.0 * BORDER},
                background_color: DARK_BACKING,
                pos_type: "absolute",
                pos_x: BORDER,
                pos_y: BORDER,
            }
            {content}
        }
    }
}

pub(super) fn unit_label(
    name: DynName,
    text: &str,
    (x, y): (f32, f32),
    width: f32,
    color: &str,
    justify_h: &str,
) -> Element {
    rsx! {
        fontstring {
            name,
            width,
            height: 12.0,
            text,
            font: UNIT_FONT,
            font_size: UNIT_FONT_SIZE,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

pub(super) fn status_bar(spec: BarSpec<'_>) -> Element {
    let fill_w = spec.width * spec.fraction.clamp(0.0, 1.0);
    let fill = dyn_name(format!("{}Fill", spec.name));
    let text = dyn_name(format!("{}Text", spec.name));
    rsx! {
        r#frame {
            name: {dyn_name(spec.name.clone())},
            width: spec.width,
            height: spec.height,
            hidden: spec.hidden,
            background_color: BAR_BG,
            pos_type: "absolute",
            pos_x: BAR_X,
            pos_y: spec.y,
            r#frame {
                name: fill,
                width: fill_w,
                height: spec.height,
                hidden: {fill_w <= 0.0},
                background_color: spec.color,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
            fontstring {
                name: text,
                width: spec.width,
                height: spec.height,
                text: spec.text,
                font: UNIT_FONT,
                font_size: UNIT_FONT_SIZE,
                font_color: VALUE_TEXT,
                outline: "OUTLINE",
                justify_h: "CENTER",
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
        }
    }
}
