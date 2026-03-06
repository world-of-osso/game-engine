//! Entity hierarchy dump for the `dump-tree` and `dump-ui-tree` IPC commands.

use bevy::prelude::*;

use crate::ui::frame::{Frame, WidgetData};
use crate::ui::registry::FrameRegistry;

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
    let label = format_ui_frame(frame);
    let passes = filter
        .map(|f| label.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);
    if passes {
        let indent = "  ".repeat(depth);
        lines.push(format!("{indent}{label}"));
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
    let vis = if !f.visible { " hidden" } else { "" };
    let size = format!("{:.0}x{:.0}", f.width, f.height);
    let pos = f.layout_rect.as_ref().map_or_else(
        || "no-layout".to_string(),
        |r| format!("at ({:.0}, {:.0})", r.x, r.y),
    );
    let extra = format_widget_extra(f);
    format!("[{wtype}] {name} #{} {size} {pos}{vis}{extra}", f.id)
}

fn format_widget_extra(f: &Frame) -> String {
    match &f.widget_data {
        Some(WidgetData::FontString(fs)) => {
            let text = truncate(&fs.text, 40);
            format!(" text=\"{text}\" size={:.0}", fs.font_size)
        }
        Some(WidgetData::EditBox(eb)) => {
            let text = truncate(&eb.text, 30);
            format!(" value=\"{text}\" cursor={}", eb.cursor_position)
        }
        Some(WidgetData::Button(btn)) => {
            let text = truncate(&btn.text, 20);
            format!(" label=\"{text}\" {:?}", btn.state)
        }
        _ => String::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
