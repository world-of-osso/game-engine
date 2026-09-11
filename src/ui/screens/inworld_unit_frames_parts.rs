use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{
    BAR_EDGE, BarConfig, DynName, STATUS_BAR_FONT, STATUS_BAR_FONT_SIZE, VALUE_TEXT, dyn_name,
};

pub(super) struct BarBlockSpec<'a> {
    pub(super) name: String,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) bg_color: &'a str,
    pub(super) fill_color: &'a str,
    pub(super) fill_width: f32,
    pub(super) value_text: &'a str,
    pub(super) text_x: f32,
    pub(super) hidden: bool,
    pub(super) mask_texture: Option<&'a str>,
}

pub(super) struct UnitFrameBarSpec<'a> {
    pub(super) prefix: &'a str,
    pub(super) label: &'a str,
    pub(super) layout: &'a BarConfig,
    pub(super) height: f32,
    pub(super) bg_color: &'a str,
    pub(super) fill_color: &'a str,
    pub(super) fill_width: f32,
    pub(super) value_text: &'a str,
    pub(super) hidden: bool,
}

struct BarBlockNames {
    frame_name: DynName,
    fill_name: DynName,
    text_name: DynName,
    edge_name: DynName,
}

struct BarBlockShellSpec<'a> {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bg_color: &'a str,
    hidden: bool,
}

pub(super) fn anchored_marker(name: String, x: f32, y: f32) -> Element {
    sized_marker(name, x, y, 0.0, 0.0)
}

pub(super) fn anchored_top_marker(name: String, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width: 0.0,
            height: 0.0,
            hidden: true,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            translate_x: "-50%",
        }
    }
}

pub(super) fn anchored_topright_marker(name: String, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width: 0.0,
            height: 0.0,
            hidden: true,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            translate_x: "-100%",
        }
    }
}

pub(super) fn sized_marker(name: String, x: f32, y: f32, width: f32, height: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

pub(super) fn marker_group(
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    children: Element,
) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {children}
        }
    }
}

pub(super) fn centered_marker(name: String, width: f32, height: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

pub(super) fn portrait_edge_marker(
    name: String,
    top_edge: bool,
    width: f32,
    height: f32,
) -> Element {
    let top = if top_edge { "0%" } else { "100%" };
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            pos_type: "absolute",
            left: "50%",
            top,
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

pub(super) fn unit_frame_bar(spec: UnitFrameBarSpec<'_>) -> Element {
    bar_block(BarBlockSpec {
        name: format!("{}{}", spec.prefix, spec.label),
        x: spec.layout.x,
        y: spec.layout.y,
        width: spec.layout.width,
        height: spec.height,
        bg_color: spec.bg_color,
        fill_color: spec.fill_color,
        fill_width: spec.fill_width,
        value_text: spec.value_text,
        text_x: spec.layout.text_x,
        hidden: spec.hidden,
        mask_texture: spec.layout.mask_texture,
    })
}

fn bar_block_names(name: &str) -> BarBlockNames {
    let frame_name = dyn_name(name.to_string());
    let fill_name = dyn_name(format!("{}Fill", frame_name.0));
    let text_name = dyn_name(format!("{}Text", frame_name.0));
    let edge_name = dyn_name(format!("{}Edge", frame_name.0));
    BarBlockNames {
        frame_name,
        fill_name,
        text_name,
        edge_name,
    }
}

fn bar_block_fill(fill_name: DynName, fill_width: f32, height: f32, fill_color: &str) -> Element {
    // Explicit zero width stays empty; hide the fill until it has content.
    let hide_fill = fill_width <= 0.0;
    rsx! {
        r#frame {
            name: {fill_name},
            width: fill_width,
            height,
            hidden: hide_fill,
            background_color: fill_color,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn bar_block_edge(edge_name: DynName, width: f32) -> Element {
    rsx! {
        r#frame {
            name: edge_name,
            width,
            height: 1.0,
            background_color: BAR_EDGE,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn bar_block_text(
    text_name: DynName,
    width: f32,
    height: f32,
    value_text: &str,
    text_x: f32,
) -> Element {
    rsx! {
        fontstring {
            name: {text_name},
            width,
            height,
            text: value_text,
            font: STATUS_BAR_FONT,
            font_size: STATUS_BAR_FONT_SIZE,
            font_color: VALUE_TEXT,
            outline: "OUTLINE",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            margin_left: text_x,
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

fn bar_block_parts(
    spec: &BarBlockSpec<'_>,
    fill_name: DynName,
    edge_name: DynName,
    text_name: DynName,
) -> Element {
    if let Some(mask) = spec.mask_texture {
        return masked_bar_parts(spec, mask, fill_name, text_name);
    }
    rsx! {
        {bar_block_fill(fill_name, spec.fill_width, spec.height, spec.fill_color)}
        {bar_block_edge(edge_name, spec.width)}
        {bar_block_text(text_name, spec.width, spec.height, spec.value_text, spec.text_x)}
    }
}

fn masked_bar_parts(spec: &BarBlockSpec<'_>, mask: &str, fill: DynName, text: DynName) -> Element {
    let background = dyn_name(format!("{}Background", spec.name));
    let fraction = (spec.fill_width / spec.width).clamp(0.0, 1.0);
    rsx! {
        {masked_bar_texture(background, spec, mask, spec.bg_color, 1.0)}
        {masked_bar_texture(fill, spec, mask, spec.fill_color, fraction)}
        {bar_block_text(text, spec.width, spec.height, spec.value_text, spec.text_x)}
    }
}

fn masked_bar_texture(
    name: DynName,
    spec: &BarBlockSpec<'_>,
    mask: &str,
    color: &str,
    fraction: f32,
) -> Element {
    let hidden = fraction <= 0.0;
    let coordinates = format!("0,{fraction},0,1");
    rsx! {
        texture {
            name,
            width: {spec.width * fraction},
            height: spec.height,
            hidden,
            texture_file: mask,
            vertex_color: color,
            tex_coords: coordinates,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn bar_block_shell(frame_name: DynName, spec: BarBlockShellSpec<'_>, content: Element) -> Element {
    let BarBlockShellSpec {
        x,
        y,
        width,
        height,
        bg_color,
        hidden,
    } = spec;
    rsx! {
        r#frame {
            name: frame_name,
            width,
            height,
            background_color: bg_color,
            hidden,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {content}
        }
    }
}

fn bar_block_shell_spec(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bg_color: &str,
    hidden: bool,
) -> BarBlockShellSpec<'_> {
    BarBlockShellSpec {
        x,
        y,
        width,
        height,
        bg_color,
        hidden,
    }
}

fn bar_block(spec: BarBlockSpec<'_>) -> Element {
    let BarBlockNames {
        frame_name,
        fill_name,
        text_name,
        edge_name,
    } = bar_block_names(&spec.name);
    let content = bar_block_parts(&spec, fill_name, edge_name, text_name);
    let background = if spec.mask_texture.is_some() {
        "0,0,0,0"
    } else {
        spec.bg_color
    };
    let shell = bar_block_shell_spec(
        spec.x,
        spec.y,
        spec.width,
        spec.height,
        background,
        spec.hidden,
    );
    bar_block_shell(frame_name, shell, content)
}
