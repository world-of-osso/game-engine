use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};

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
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
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
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
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
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
        }
    }
}

pub(super) fn centered_marker(
    name: String,
    relative_to: FrameName,
    width: f32,
    height: f32,
) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            anchor {
                point: AnchorPoint::Center,
                relative_to,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

pub(super) fn portrait_centered_marker(
    name: String,
    relative_to: FrameName,
    relative_point: AnchorPoint,
    width: f32,
    height: f32,
) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name),
            width,
            height,
            hidden: true,
            anchor {
                point: AnchorPoint::Center,
                relative_to,
                relative_point,
            }
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
    rsx! {
        r#frame {
            name: {fill_name},
            width: fill_width,
            height,
            background_color: fill_color,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
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
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
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
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: {text_x},
            }
        }
    }
}

fn bar_block_parts(
    spec: &BarBlockSpec<'_>,
    fill_name: DynName,
    edge_name: DynName,
    text_name: DynName,
) -> Element {
    rsx! {
        {bar_block_fill(fill_name, spec.fill_width, spec.height, spec.fill_color)}
        {bar_block_edge(edge_name, spec.width)}
        {bar_block_text(text_name, spec.width, spec.height, spec.value_text, spec.text_x)}
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
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: {-y},
            }
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
    let shell = bar_block_shell_spec(
        spec.x,
        spec.y,
        spec.width,
        spec.height,
        spec.bg_color,
        spec.hidden,
    );
    bar_block_shell(frame_name, shell, content)
}
