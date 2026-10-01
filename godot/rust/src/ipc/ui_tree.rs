//! Original registry dump text/filter semantics, using mounted native registries.

use game_engine_network::ipc_wire::Response;
use godot::{classes::Node, prelude::*};
use ui_toolkit::{
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    widgets::texture::TextureSource,
};

pub(super) fn dump_mounted_ui(client: &Gd<Node>, filter: Option<&str>) -> Response {
    let root = match super::tree::scene_root(client) {
        Ok(root) => root,
        Err(error) => return Response::Error(error),
    };
    let mut lines = Vec::new();
    collect_registries(&root, filter, &mut lines);
    Response::Tree(lines.join("\n"))
}

fn collect_registries(node: &Gd<Node>, filter: Option<&str>, lines: &mut Vec<String>) {
    if let Ok(ui) = node.clone().try_cast::<crate::ui::RegistryUi>() {
        let ui = ui.bind();
        if let Some(registry) = ui.registry() {
            emit_registry(registry, filter, lines);
        }
    }
    for child in node.get_children().iter_shared() {
        collect_registries(&child, filter, lines);
    }
}

fn emit_registry(registry: &FrameRegistry, filter: Option<&str>, lines: &mut Vec<String>) {
    let mut roots: Vec<u64> = registry
        .frames_iter()
        .filter(|frame| frame.parent_id.is_none())
        .map(|frame| frame.id)
        .collect();
    roots.sort_by(|a, b| {
        let an = registry
            .get(*a)
            .and_then(|frame| frame.name.as_deref())
            .unwrap_or("");
        let bn = registry
            .get(*b)
            .and_then(|frame| frame.name.as_deref())
            .unwrap_or("");
        an.cmp(bn)
    });
    for id in roots {
        if let Some(frame) = registry.get(id) {
            emit_ui_frame(frame, 0, filter, false, registry, lines);
        }
    }
}

fn emit_ui_frame(
    frame: &Frame,
    depth: usize,
    filter: Option<&str>,
    ancestor_matched: bool,
    registry: &FrameRegistry,
    lines: &mut Vec<String>,
) {
    let main_line = format_ui_frame(frame);
    let passes =
        filter.is_none_or(|filter| main_line.to_lowercase().contains(&filter.to_lowercase()));
    let emit_self = ancestor_matched || passes;
    if emit_self {
        let indent = "  ".repeat(depth);
        lines.push(format!("{indent}{main_line}"));
        emit_position_lines(frame, &indent, lines);
        emit_texture_lines(frame, &indent, lines);
    }
    for &child_id in &frame.children {
        if let Some(child) = registry.get(child_id) {
            emit_ui_frame(child, depth + 1, filter, emit_self, registry, lines);
        }
    }
}

fn format_ui_frame(frame: &Frame) -> String {
    let name = frame.name.as_deref().unwrap_or("(anon)");
    let widget = format!("{:?}", frame.widget_type);
    let visible = if frame.visible { "visible" } else { "hidden" };
    let size = format_size_info(frame);
    let strata = format!("{}:{}", frame.strata.as_str(), frame.frame_level);
    let layout = if frame.layout_rect.is_some() {
        ""
    } else {
        " [layout_rect=None]"
    };
    let position = frame
        .layout_rect
        .as_ref()
        .map(|rect| {
            format!(
                " x={:.0} y={:.0} w={:.0} h={:.0}",
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .unwrap_or_default();
    let alpha = format!(" alpha={:.2}", frame.alpha);
    let scale = if (frame.scale - 1.0).abs() > 0.001 {
        format!(" scale={:.2}", frame.scale)
    } else {
        String::new()
    };
    let extra = format_widget_extra(frame);
    format!("{name} [{widget}] {size} {visible} {strata}{layout}{position}{alpha}{scale}{extra}")
}

fn format_size_info(frame: &Frame) -> String {
    let width = frame.resolved_width();
    let height = frame.resolved_height();
    let stored_width = frame.width.value();
    let stored_height = frame.height.value();
    let differs = (stored_width - width).abs() > 0.5 || (stored_height - height).abs() > 0.5;
    if differs && (stored_width > 0.0 || stored_height > 0.0) {
        format!("({width:.0}x{height:.0}) [stored={stored_width:.0}x{stored_height:.0}]")
    } else {
        format!("({width:.0}x{height:.0})")
    }
}

fn format_widget_extra(frame: &Frame) -> String {
    match &frame.widget_data {
        Some(WidgetData::FontString(font)) => {
            let text = truncate(&font.text, 40);
            let name = format!("{:?}", font.font);
            format!(
                " text=\"{text}\" font=\"{name}\" size={:.0}",
                font.font_size
            )
        }
        Some(WidgetData::EditBox(edit)) => {
            let displayed = if edit.password {
                "*".repeat(edit.text.len())
            } else {
                edit.text.clone()
            };
            let text = truncate(&displayed, 30);
            let password = if edit.password { " password" } else { "" };
            format!(" text=\"{text}\" cursor={}{password}", edit.cursor_position)
        }
        Some(WidgetData::Button(button)) if !button.text.is_empty() => {
            format!(" text=\"{}\"", truncate(&button.text, 20))
        }
        Some(WidgetData::StatusBar(bar)) => format!(" value={:.1}/{:.1}", bar.value, bar.max),
        _ => String::new(),
    }
}

fn emit_position_lines(frame: &Frame, indent: &str, lines: &mut Vec<String>) {
    lines.push(format!(
        "{indent}  [position] {:?} anchor={:?} insets={:?} translation={:?} margin={:?}",
        frame.position_type, frame.anchor, frame.position, frame.translation, frame.margin,
    ));
}

fn emit_texture_lines(frame: &Frame, indent: &str, lines: &mut Vec<String>) {
    if let Some(WidgetData::Texture(texture)) = &frame.widget_data {
        emit_texture_source_line("[texture]", &texture.source, indent, lines);
    }
    if let Some(WidgetData::Button(button)) = &frame.widget_data {
        for (label, source) in [
            ("[normal]", &button.normal_texture),
            ("[pushed]", &button.pushed_texture),
            ("[highlight]", &button.highlight_texture),
            ("[disabled]", &button.disabled_texture),
        ] {
            if let Some(source) = source {
                emit_texture_source_line(label, source, indent, lines);
            }
        }
    }
}

fn emit_texture_source_line(
    label: &str,
    source: &TextureSource,
    indent: &str,
    lines: &mut Vec<String>,
) {
    if let Some(detail) = format_texture_source(source) {
        lines.push(format!("{indent}  {label} {detail}"));
    }
}

fn format_texture_source(source: &TextureSource) -> Option<String> {
    match source {
        TextureSource::File(path) => {
            let short = path.rsplit('/').next().unwrap_or(path);
            Some(format!("file=\"{short}\""))
        }
        TextureSource::FileDataId(id) => Some(format!("fdid={id}")),
        TextureSource::SolidColor(color) => Some(format!(
            "solid({:.2},{:.2},{:.2},{:.2})",
            color[0], color[1], color[2], color[3],
        )),
        TextureSource::Atlas(name) => Some(format!("atlas=\"{name}\"")),
        TextureSource::Dynamic(_) => Some("dynamic".into()),
        TextureSource::None => None,
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}…", &text[..text.floor_char_boundary(max)])
    }
}
