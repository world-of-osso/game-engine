//! Entity hierarchy dump for the `dump-tree` and `dump-ui-tree` IPC commands.

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
        .map(|r| format!(" x={:.0}, y={:.0}", r.x, r.y))
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

// --- Scene tree ---

use crate::scene_tree::{
    NodeProps, SceneNode, SceneNodeTransform, SceneSnapshot, SceneSnapshotNode, SceneTree,
};

/// Build a formatted scene tree string.
pub fn build_scene_tree(tree: &SceneTree, transforms: &Query<&Transform>) -> String {
    let mut lines = Vec::new();
    emit_scene_node(&tree.root, 0, transforms, &mut lines);
    lines.join("\n")
}

pub fn build_scene_snapshot(snapshot: &SceneSnapshot) -> String {
    let mut lines = Vec::new();
    emit_scene_snapshot_node(&snapshot.root, 0, &mut lines);
    lines.join("\n")
}

fn emit_scene_node(
    node: &SceneNode,
    depth: usize,
    transforms: &Query<&Transform>,
    lines: &mut Vec<String>,
) {
    let indent = "  ".repeat(depth);
    let line = format_scene_node(node, transforms);
    lines.push(format!("{indent}{line}"));
    for child in &node.children {
        emit_scene_node(child, depth + 1, transforms, lines);
    }
}

fn emit_scene_snapshot_node(node: &SceneSnapshotNode, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let line = format_scene_snapshot_node(node);
    lines.push(format!("{indent}{line}"));
    for child in &node.children {
        emit_scene_snapshot_node(child, depth + 1, lines);
    }
}

fn format_scene_node(node: &SceneNode, transforms: &Query<&Transform>) -> String {
    format_node_with_position(
        &node.label,
        &node.props,
        &format_node_position(node, transforms),
    )
}

fn format_scene_snapshot_node(node: &SceneSnapshotNode) -> String {
    format_node_with_position(
        &node.label,
        &node.props,
        &format_snapshot_position(node.transform),
    )
}

fn format_node_with_position(label: &str, props: &NodeProps, pos: &str) -> String {
    match props {
        NodeProps::Scene => label.to_string(),
        NodeProps::Character {
            model,
            race,
            gender,
        } => {
            format!("{label} \"{model}\" race={race} gender={gender}{pos}")
        }
        NodeProps::Background {
            model,
            doodad_count,
        } => format!("{label} \"{model}\" doodads={doodad_count}{pos}"),
        NodeProps::Object { kind, model } => {
            format!("{label} {kind} \"{model}\"{pos}")
        }
        NodeProps::Ground | NodeProps::Terrain => format!("{label}{pos}"),
        NodeProps::Camera { fov } => format!("{label} fov={fov}{pos}"),
        NodeProps::Light { kind, intensity } => format!("{label} {kind}={intensity}"),
        NodeProps::EquipmentSlot { model, .. } => format_equipment_slot(label, model),
        NodeProps::Player {
            name,
            is_local,
            model_path,
            skin_path,
            display_scale,
        } => format_player_node(
            label,
            name,
            *is_local,
            model_path,
            skin_path,
            *display_scale,
            pos,
        ),
        NodeProps::Npc {
            name,
            display_id,
            model_path,
            skin_path,
            display_scale,
        } => format_npc_node(
            label,
            name,
            *display_id,
            model_path,
            skin_path,
            *display_scale,
            pos,
        ),
    }
}

fn format_node_position(node: &SceneNode, transforms: &Query<&Transform>) -> String {
    node.entity
        .and_then(|e| transforms.get(e).ok())
        .map(|t| {
            format!(
                " @ ({:.1}, {:.1}, {:.1})",
                t.translation.x, t.translation.y, t.translation.z
            )
        })
        .unwrap_or_default()
}

fn format_snapshot_position(transform: Option<SceneNodeTransform>) -> String {
    transform
        .map(|t| {
            format!(
                " @ ({:.1}, {:.1}, {:.1})",
                t.translation[0], t.translation[1], t.translation[2]
            )
        })
        .unwrap_or_default()
}

fn format_equipment_slot(label: &str, model: &Option<String>) -> String {
    match model {
        Some(m) => format!("{label} \"{m}\""),
        None => format!("{label} (empty)"),
    }
}

fn format_player_node(
    label: &str,
    name: &str,
    is_local: bool,
    model_path: &Option<String>,
    skin_path: &Option<String>,
    display_scale: Option<f32>,
    pos: &str,
) -> String {
    let tag = if is_local { " (local)" } else { "" };
    let assets = format_model_assets(model_path, skin_path, display_scale);
    format!("{label} \"{name}\"{tag}{assets}{pos}")
}

fn format_npc_node(
    label: &str,
    name: &str,
    display_id: Option<u32>,
    model_path: &Option<String>,
    skin_path: &Option<String>,
    display_scale: Option<f32>,
    pos: &str,
) -> String {
    let disp = display_id
        .map(|d| format!(" display={d}"))
        .unwrap_or_default();
    let assets = format_model_assets(model_path, skin_path, display_scale);
    format!("{label} \"{name}\"{disp}{assets}{pos}")
}

fn format_model_assets(
    model_path: &Option<String>,
    skin_path: &Option<String>,
    display_scale: Option<f32>,
) -> String {
    let mut parts = Vec::new();
    if let Some(scale) = display_scale
        && (scale - 1.0).abs() > 0.001
    {
        parts.push(format!(" scale={scale:.2}"));
    }
    if let Some(model_path) = model_path {
        parts.push(format!(" m2=\"{model_path}\""));
    }
    if let Some(skin_path) = skin_path {
        parts.push(format!(" skin=\"{skin_path}\""));
    }
    parts.concat()
}
