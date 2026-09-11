//! Entity hierarchy dump for the `dump-tree` and `dump-ui-tree` IPC commands.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;

use crate::ui::anchor::anchor_position;
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

#[derive(QueryData)]
pub struct TreeQueryData<'w> {
    pub entity: Entity,
    pub name: Option<&'w Name>,
    pub children: Option<&'w Children>,
    pub visibility: Option<&'w Visibility>,
    pub transform: &'w Transform,
}

/// Build a formatted tree string for all root entities.
pub fn build_tree(
    tree_query: &Query<TreeQueryData<'_>>,
    parent_query: &Query<&ChildOf>,
    filter: Option<&str>,
) -> String {
    let infos: Vec<EntityInfo> = tree_query
        .iter()
        .map(|item| EntityInfo {
            entity: item.entity,
            name: item.name.map(|n| n.as_str().to_owned()),
            children: item
                .children
                .map(|c| c.iter().collect())
                .unwrap_or_default(),
            hidden: matches!(item.visibility, Some(Visibility::Hidden)),
            translation: item.transform.translation,
            scale: item.transform.scale,
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
    let mut roots: Vec<u64> = registry
        .frames_iter()
        .filter(|f| f.parent_id.is_none())
        .map(|f| f.id)
        .collect();
    roots.sort_by(|a, b| {
        let an = registry
            .get(*a)
            .and_then(|f| f.name.as_deref())
            .unwrap_or("");
        let bn = registry
            .get(*b)
            .and_then(|f| f.name.as_deref())
            .unwrap_or("");
        an.cmp(bn)
    });
    for id in roots {
        if let Some(frame) = registry.get(id) {
            emit_ui_frame(frame, 0, filter, false, registry, &mut lines);
        }
    }
    lines.join("\n")
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
    let passes = filter
        .map(|f| main_line.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);
    let emit_self = ancestor_matched || passes;
    if emit_self {
        let indent = "  ".repeat(depth);
        lines.push(format!("{indent}{main_line}"));
        emit_anchor_lines(frame, registry, &indent, lines);
        emit_texture_lines(frame, &indent, lines);
    }
    for &child_id in &frame.children {
        if let Some(child) = registry.get(child_id) {
            emit_ui_frame(child, depth + 1, filter, emit_self, registry, lines);
        }
    }
}

fn format_ui_frame(f: &Frame) -> String {
    let name = f.name.as_deref().unwrap_or("(anon)");
    let wtype = format!("{:?}", f.widget_type);
    let vis = if f.visible { "visible" } else { "hidden" };
    let size = format_size_info(f);
    let strata = format!("{}:{}", f.strata.as_str(), f.frame_level);
    let layout = format_layout_rect_info(f);
    let pos = format_position_info(f);
    let alpha = format!(" alpha={:.2}", f.alpha);
    let scale = format_scale_info(f);
    let extra = format_widget_extra(f);
    format!("{name} [{wtype}] {size} {vis} {strata}{layout}{pos}{alpha}{scale}{extra}")
}

fn format_size_info(f: &Frame) -> String {
    let resolved_w = f.resolved_width();
    let resolved_h = f.resolved_height();
    let stored_w = f.width.value();
    let stored_h = f.height.value();
    let differs = (stored_w - resolved_w).abs() > 0.5 || (stored_h - resolved_h).abs() > 0.5;
    if differs && (stored_w > 0.0 || stored_h > 0.0) {
        format!("({resolved_w:.0}x{resolved_h:.0}) [stored={stored_w:.0}x{stored_h:.0}]")
    } else {
        format!("({resolved_w:.0}x{resolved_h:.0})")
    }
}

fn format_layout_rect_info(f: &Frame) -> String {
    if f.layout_rect.is_some() {
        String::new()
    } else {
        " [layout_rect=None]".to_string()
    }
}

fn format_position_info(f: &Frame) -> String {
    f.layout_rect
        .as_ref()
        .map(|r| {
            format!(
                " x={:.0} y={:.0} w={:.0} h={:.0}",
                r.x, r.y, r.width, r.height
            )
        })
        .unwrap_or_default()
}

fn format_scale_info(f: &Frame) -> String {
    if (f.scale - 1.0).abs() > 0.001 {
        format!(" scale={:.2}", f.scale)
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
            format!(" text=\"{text}\" cursor={}{pw}", eb.cursor_position)
        }
        Some(WidgetData::Button(btn)) => {
            if btn.text.is_empty() {
                String::new()
            } else {
                let text = truncate(&btn.text, 20);
                format!(" text=\"{text}\"")
            }
        }
        Some(WidgetData::Texture(_)) => String::new(),
        Some(WidgetData::StatusBar(sb)) => {
            format!(" value={:.1}/{:.1}", sb.value, sb.max)
        }
        _ => String::new(),
    }
}

fn emit_anchor_lines(f: &Frame, registry: &FrameRegistry, indent: &str, lines: &mut Vec<String>) {
    for anchor in &f.anchors {
        let (rel_name, rel_rect) = anchor
            .relative_to
            .and_then(|id| registry.get(id))
            .map(|rf| {
                (
                    rf.name.as_deref().unwrap_or("(anon)"),
                    rf.layout_rect
                        .clone()
                        .unwrap_or_else(|| registry.screen_rect()),
                )
            })
            .unwrap_or_else(|| ("screen", registry.screen_rect()));
        let (ax, ay) = anchor_position(
            anchor.relative_point,
            rel_rect.x,
            rel_rect.y,
            rel_rect.width,
            rel_rect.height,
        );
        lines.push(format!(
            "{indent}  [anchor] {} -> {rel_name}:{} offset({:.0},{:.0}) -> ({:.0},{:.0})",
            anchor.point.as_str(),
            anchor.relative_point.as_str(),
            anchor.x_offset,
            anchor.y_offset,
            ax + anchor.x_offset,
            ay - anchor.y_offset,
        ));
    }
}

fn emit_texture_lines(f: &Frame, indent: &str, lines: &mut Vec<String>) {
    if let Some(WidgetData::Texture(tex)) = &f.widget_data {
        emit_texture_source_line("[texture]", &tex.source, indent, lines);
    }
    if let Some(WidgetData::Button(btn)) = &f.widget_data {
        if let Some(src) = &btn.normal_texture {
            emit_texture_source_line("[normal]", src, indent, lines);
        }
        if let Some(src) = &btn.pushed_texture {
            emit_texture_source_line("[pushed]", src, indent, lines);
        }
        if let Some(src) = &btn.highlight_texture {
            emit_texture_source_line("[highlight]", src, indent, lines);
        }
        if let Some(src) = &btn.disabled_texture {
            emit_texture_source_line("[disabled]", src, indent, lines);
        }
    }
}

fn emit_texture_source_line(
    label: &str,
    src: &TextureSource,
    indent: &str,
    lines: &mut Vec<String>,
) {
    if let Some(detail) = format_texture_source(src) {
        lines.push(format!("{indent}  {label} {detail}"));
    }
}

fn format_texture_source(src: &TextureSource) -> Option<String> {
    match src {
        TextureSource::File(path) => {
            let short = path.rsplit('/').next().unwrap_or(path);
            Some(format!("file=\"{short}\""))
        }
        TextureSource::FileDataId(fdid) => Some(format!("fdid={fdid}")),
        TextureSource::SolidColor(c) => Some(format!(
            "solid({:.2},{:.2},{:.2},{:.2})",
            c[0], c[1], c[2], c[3]
        )),
        TextureSource::Atlas(name) => Some(format!("atlas=\"{name}\"")),
        TextureSource::Dynamic(_) => Some("dynamic".to_string()),
        TextureSource::None => None,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[path = "scenes/dump_tree.rs"]
mod dump_scene_tree;
pub use dump_scene_tree::{build_scene_snapshot, build_scene_tree};
