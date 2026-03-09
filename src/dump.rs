//! Entity hierarchy dump for the `dump-tree` and `dump-ui-tree` IPC commands.

use bevy::prelude::*;

use crate::ui::frame::{Frame, WidgetData};
use crate::ui::registry::FrameRegistry;
use crate::ui::widgets::texture::TextureSource;

/// Snapshot of one entity's tree-relevant data.
struct EntityInfo {
    entity: Entity,
    name: Option<String>,
    children: Vec<Entity>,
    hidden: bool,
    translation: Vec3,
    scale: Vec3,
}

/// Build a formatted tree string for all root entities.
#[allow(clippy::type_complexity)]
pub fn build_tree(
    tree_query: &Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: &Query<&ChildOf>,
    filter: Option<&str>,
) -> String {
    let infos: Vec<EntityInfo> = tree_query
        .iter()
        .map(|(e, name, children, vis, transform)| EntityInfo {
            entity: e,
            name: name.map(|n| n.as_str().to_owned()),
            children: children.map(|c| c.iter().collect()).unwrap_or_default(),
            hidden: matches!(vis, Some(Visibility::Hidden)),
            translation: transform.translation,
            scale: transform.scale,
        })
        .collect();

    let mut lines: Vec<String> = Vec::new();
    for info in &infos {
        if parent_query.get(info.entity).is_ok() {
            continue; // not a root
        }
        emit_entity(info, 0, filter, &infos, &mut lines);
    }
    lines.join("\n")
}

fn emit_entity(
    info: &EntityInfo,
    depth: usize,
    filter: Option<&str>,
    all: &[EntityInfo],
    lines: &mut Vec<String>,
) {
    let label = info.name.as_deref().unwrap_or("unnamed").to_owned();

    let passes = filter
        .map(|f| label.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);

    if passes {
        let indent = "  ".repeat(depth);
        lines.push(format!("{}{}", indent, format_entity(&label, info)));
    }

    for &child in &info.children {
        if let Some(child_info) = all.iter().find(|e| e.entity == child) {
            emit_entity(child_info, depth + 1, filter, all, lines);
        }
    }
}

fn format_entity(label: &str, info: &EntityInfo) -> String {
    let id = format!("({:?})", info.entity);
    let pos = format_position(info.translation);
    let scale = format_scale(info.scale);
    let vis = if info.hidden { " hidden" } else { "" };
    format!("{label} {id} {pos}{scale}{vis}")
}

fn format_position(t: Vec3) -> String {
    format!("at ({:.1}, {:.1}, {:.1})", t.x, t.y, t.z)
}

fn format_scale(s: Vec3) -> String {
    if (s - Vec3::ONE).length_squared() > 1e-6 {
        format!(" scale({:.1}, {:.1}, {:.1})", s.x, s.y, s.z)
    } else {
        String::new()
    }
}

// --- UI frame tree ---

/// Build a formatted tree string for all root UI frames in the registry.
pub fn build_ui_tree(registry: &FrameRegistry, filter: Option<&str>) -> String {
    let mut lines: Vec<String> = Vec::new();
    let roots: Vec<u64> = registry
        .frames_iter()
        .filter(|f| f.parent_id.is_none())
        .map(|f| f.id)
        .collect();
    for id in roots {
        if let Some(frame) = registry.get(id) {
            emit_ui_frame(frame, 0, filter, registry, &mut lines);
        }
    }
    lines.join("\n")
}

fn emit_ui_frame(
    frame: &Frame,
    depth: usize,
    filter: Option<&str>,
    registry: &FrameRegistry,
    lines: &mut Vec<String>,
) {
    let main_line = format_ui_frame(frame);
    let passes = filter
        .map(|f| main_line.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);
    if passes {
        let indent = "  ".repeat(depth);
        lines.push(format!("{indent}{main_line}"));
        emit_anchor_lines(frame, registry, &indent, lines);
        emit_texture_lines(frame, &indent, lines);
    }
    for &child_id in &frame.children {
        if let Some(child) = registry.get(child_id) {
            emit_ui_frame(child, depth + 1, filter, registry, lines);
        }
    }
}

fn format_ui_frame(f: &Frame) -> String {
    let name = f.name.as_deref().unwrap_or("(anon)");
    let wtype = format!("{:?}", f.widget_type);
    let vis = if f.visible { "visible" } else { "hidden" };
    let size = format!("{:.0}x{:.0}", f.width, f.height);
    let strata = format!("{:?}:{}", f.strata, f.frame_level);
    let pos = format_position_info(f);
    let alpha = format!(" alpha={:.2}", f.effective_alpha);
    let scale = format_scale_info(f);
    let extra = format_widget_extra(f);
    format!("{name} [{wtype}] {size} {vis} {strata}{pos}{alpha}{scale}{extra}")
}

fn format_position_info(f: &Frame) -> String {
    f.layout_rect.as_ref().map_or_else(
        || " no-layout".to_string(),
        |r| format!(" x={:.0} y={:.0}", r.x, r.y),
    )
}

fn format_scale_info(f: &Frame) -> String {
    if (f.effective_scale - 1.0).abs() > 0.001 {
        format!(" scale={:.2}", f.effective_scale)
    } else {
        String::new()
    }
}

fn format_widget_extra(f: &Frame) -> String {
    match &f.widget_data {
        Some(WidgetData::FontString(fs)) => {
            let text = truncate(&fs.text, 40);
            let font = format!("{:?}", fs.font);
            format!(" text=\"{text}\" font=\"{font}\" size={:.0}", fs.font_size)
        }
        Some(WidgetData::EditBox(eb)) => {
            let text = truncate(&eb.text, 30);
            let pw = if eb.password { " password" } else { "" };
            format!(" value=\"{text}\" cursor={}{pw}", eb.cursor_position)
        }
        Some(WidgetData::Button(btn)) => {
            let text = truncate(&btn.text, 20);
            format!(" label=\"{text}\" {:?}", btn.state)
        }
        Some(WidgetData::Texture(tex)) => format_texture_source(&tex.source),
        Some(WidgetData::StatusBar(sb)) => {
            format!(" value={:.1}/{:.1}", sb.value, sb.max)
        }
        _ => String::new(),
    }
}

fn emit_anchor_lines(f: &Frame, registry: &FrameRegistry, indent: &str, lines: &mut Vec<String>) {
    for anchor in &f.anchors {
        let rel_name = anchor
            .relative_to
            .and_then(|id| registry.get(id))
            .and_then(|rf| rf.name.as_deref())
            .unwrap_or("screen");
        lines.push(format!(
            "{indent}  [anchor] {:?} -> {rel_name}:{:?} offset({:.0},{:.0})",
            anchor.point, anchor.relative_point, anchor.x_offset, anchor.y_offset,
        ));
    }
}

fn emit_texture_lines(f: &Frame, indent: &str, lines: &mut Vec<String>) {
    if let Some(WidgetData::Button(btn)) = &f.widget_data {
        if let Some(src) = &btn.normal_texture {
            lines.push(format!("{indent}  [normal]{}", format_texture_source(src)));
        }
        if let Some(src) = &btn.pushed_texture {
            lines.push(format!("{indent}  [pushed]{}", format_texture_source(src)));
        }
        if let Some(src) = &btn.highlight_texture {
            lines.push(format!(
                "{indent}  [highlight]{}",
                format_texture_source(src)
            ));
        }
    }
}

fn format_texture_source(src: &TextureSource) -> String {
    match src {
        TextureSource::File(path) => {
            let short = path.rsplit('/').next().unwrap_or(path);
            format!(" file=\"{short}\"")
        }
        TextureSource::FileDataId(fdid) => format!(" fdid={fdid}"),
        TextureSource::SolidColor(c) => {
            format!(" solid({:.2},{:.2},{:.2},{:.2})", c[0], c[1], c[2], c[3])
        }
        TextureSource::Atlas(name) => format!(" atlas=\"{name}\""),
        TextureSource::None => String::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
