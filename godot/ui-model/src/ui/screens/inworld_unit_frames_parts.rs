use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_art::{AtlasArt, FRAME_PORTRAIT_OFF};
use super::{DynName, UNIT_FONT, VALUE_TEXT, dyn_name};

/// Rect `(x, y, width, height)` from the parent's top-left.
pub(super) type Rect = (f32, f32, f32, f32);

pub(super) struct BarSpec<'a> {
    pub(super) name: String,
    pub(super) rect: Rect,
    pub(super) fraction: f32,
    pub(super) art: Option<AtlasArt>,
    pub(super) text: &'a str,
    pub(super) font_size: f32,
    pub(super) hidden: bool,
}

/// Cluster frame: the Retail portrait-off art filling the root, anchored from the screen's
/// bottom centre (`left` is the offset of the frame's left edge from the centre line).
pub(super) fn art_root(
    name: DynName,
    (width, height): (f32, f32),
    (left, bottom): (f32, f32),
    hidden: bool,
    content: Element,
) -> Element {
    let art = art_texture(
        dyn_name(format!("{}Art", name.0)),
        &FRAME_PORTRAIT_OFF,
        (0.0, 0.0, width, height),
        false,
    );
    rsx! {
        r#frame {
            name,
            width,
            height,
            hidden,
            mouse_enabled: true,
            pos_type: "absolute",
            left: "50%",
            margin_left: left,
            bottom,
            {art}
            {content}
        }
    }
}

/// One atlas crop stretched over `rect`.
pub(super) fn art_texture(name: DynName, art: &AtlasArt, rect: Rect, hidden: bool) -> Element {
    tinted_art_texture(name, art, rect, "1.0,1.0,1.0,1.0", hidden)
}

pub(super) fn tinted_art_texture(
    name: DynName,
    art: &AtlasArt,
    (x, y, width, height): Rect,
    vertex_color: &str,
    hidden: bool,
) -> Element {
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name,
            width,
            height,
            hidden,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

pub(super) fn unit_label(
    name: DynName,
    text: &str,
    (x, y, width, height): Rect,
    (color, font_size): (&str, f32),
    justify_h: &str,
) -> Element {
    rsx! {
        fontstring {
            name,
            width,
            height,
            text,
            font: UNIT_FONT,
            font_size,
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

/// Retail `StatusBar`: the bar texture revealed left to right by `fraction`, value text
/// centred on top (`TextStatusBarText`).
pub(super) fn status_bar(spec: BarSpec<'_>) -> Element {
    let (x, y, width, height) = spec.rect;
    let fill = spec
        .art
        .map(|art| {
            bar_fill(
                format!("{}Fill", spec.name),
                &art,
                (width, height),
                spec.fraction,
            )
        })
        .unwrap_or_default();
    let text = dyn_name(format!("{}Text", spec.name));
    rsx! {
        r#frame {
            name: {dyn_name(spec.name.clone())},
            width,
            height,
            hidden: spec.hidden,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {fill}
            fontstring {
                name: text,
                width,
                height,
                text: spec.text,
                font: UNIT_FONT,
                font_size: spec.font_size,
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

fn bar_fill(name: String, art: &AtlasArt, (width, height): (f32, f32), fraction: f32) -> Element {
    let fraction = fraction.clamp(0.0, 1.0);
    let fill_w = width * fraction;
    let coords = art.tex_coords(fraction);
    rsx! {
        texture {
            name: {dyn_name(name)},
            width: fill_w,
            height,
            hidden: {fill_w <= 0.0},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}
