use std::collections::HashMap;

use bevy::camera::primitives::Aabb;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::picking::prelude::MeshRayCastSettings;
use bevy::prelude::*;

use crate::scene_tree::{
    NodeProps, SceneNode, SceneNodeTransform, SceneSnapshot, SceneSnapshotNode, SceneTree,
};

pub fn build_scene_tree(
    tree: &SceneTree,
    transforms: &Query<&Transform>,
    global_transforms: &Query<&GlobalTransform>,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ray_cast: &mut MeshRayCast,
) -> String {
    let mut lines = Vec::new();
    let displayed = compute_displayed_nodes(
        tree,
        global_transforms,
        parent_query,
        aabb_query,
        camera_query,
        ray_cast,
    );
    emit_scene_node(&tree.root, 0, transforms, &displayed, &mut lines);
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
    displayed: &HashMap<Entity, bool>,
    lines: &mut Vec<String>,
) {
    let indent = "  ".repeat(depth);
    lines.push(format!(
        "{indent}{}",
        format_scene_node(node, transforms, displayed)
    ));
    for child in &node.children {
        emit_scene_node(child, depth + 1, transforms, displayed, lines);
    }
}

fn emit_scene_snapshot_node(node: &SceneSnapshotNode, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    lines.push(format!("{indent}{}", format_scene_snapshot_node(node)));
    for child in &node.children {
        emit_scene_snapshot_node(child, depth + 1, lines);
    }
}

fn format_scene_node(
    node: &SceneNode,
    transforms: &Query<&Transform>,
    displayed: &HashMap<Entity, bool>,
) -> String {
    let pos = format_node_position(node, transforms);
    let displayed = format_displayed_suffix(node, displayed);
    format_node_with_position(&node.label, &node.props, &pos, displayed)
}

fn format_scene_snapshot_node(node: &SceneSnapshotNode) -> String {
    format_node_with_position(
        &node.label,
        &node.props,
        &format_snapshot_position(node.transform),
        String::new(),
    )
}

fn format_node_with_position(
    label: &str,
    props: &NodeProps,
    pos: &str,
    displayed: String,
) -> String {
    match props {
        NodeProps::Scene => format_scene_label(label),
        NodeProps::Character {
            model,
            race,
            gender,
        } => format_character_label(label, model, race, gender, pos, &displayed),
        NodeProps::Background {
            model,
            doodad_count,
        } => format_background_label(label, model, *doodad_count, pos, &displayed),
        _ => format_misc_node(label, props, pos, &displayed),
    }
}

fn format_misc_node(label: &str, props: &NodeProps, pos: &str, displayed: &str) -> String {
    match props {
        NodeProps::Object { kind, model } => {
            format_object_label(label, kind, model, pos, displayed)
        }
        NodeProps::Ground | NodeProps::Terrain => format_terrain_label(label, pos, displayed),
        NodeProps::Camera { fov } => format_camera_label(label, *fov, pos, displayed),
        NodeProps::Light { kind, intensity } => format_light_label(label, kind, *intensity),
        NodeProps::EquipmentSlot {
            model,
            anchor,
            attachment,
            attachment_anchor,
            ..
        } => format_equipment_slot(label, model, anchor, attachment, attachment_anchor),
        NodeProps::Player { .. } => format_player_props(label, props, pos, displayed),
        NodeProps::Npc { .. } => format_npc_props(label, props, pos, displayed),
        NodeProps::Scene | NodeProps::Character { .. } | NodeProps::Background { .. } => {
            unreachable!("handled by format_node_with_position")
        }
    }
}

fn format_player_props(label: &str, props: &NodeProps, pos: &str, displayed: &str) -> String {
    let NodeProps::Player {
        name,
        is_local,
        model_path,
        skin_path,
        display_scale,
    } = props
    else {
        unreachable!("player props")
    };
    format_player_node(
        label,
        name,
        *is_local,
        model_path,
        skin_path,
        *display_scale,
        pos,
        displayed,
    )
}

fn format_npc_props(label: &str, props: &NodeProps, pos: &str, displayed: &str) -> String {
    let NodeProps::Npc {
        name,
        display_id,
        model_path,
        skin_path,
        display_scale,
    } = props
    else {
        unreachable!("npc props")
    };
    format_npc_node(
        label,
        name,
        *display_id,
        model_path,
        skin_path,
        *display_scale,
        pos,
        displayed,
    )
}

fn format_scene_label(label: &str) -> String {
    label.to_string()
}

fn format_character_label(
    label: &str,
    model: &str,
    race: &str,
    gender: &str,
    pos: &str,
    displayed: &str,
) -> String {
    format!("{label} \"{model}\" race={race} gender={gender}{pos}{displayed}")
}

fn format_background_label(
    label: &str,
    model: &str,
    doodad_count: usize,
    pos: &str,
    displayed: &str,
) -> String {
    format!("{label} \"{model}\" doodads={doodad_count}{pos}{displayed}")
}

fn format_object_label(label: &str, kind: &str, model: &str, pos: &str, displayed: &str) -> String {
    format!("{label} {kind} \"{model}\"{pos}{displayed}")
}

fn format_terrain_label(label: &str, pos: &str, displayed: &str) -> String {
    format!("{label}{pos}{displayed}")
}

fn format_camera_label(label: &str, fov: f32, pos: &str, displayed: &str) -> String {
    format!("{label} fov={fov}{pos}{displayed}")
}

fn format_light_label(label: &str, kind: &str, intensity: f32) -> String {
    format!("{label} {kind}={intensity}")
}

fn format_displayed_suffix(node: &SceneNode, displayed: &HashMap<Entity, bool>) -> String {
    let Some(entity) = node.entity else {
        return String::new();
    };
    if !supports_displayed_flag(&node.props) {
        return String::new();
    }
    format!(
        " is_displayed={}",
        displayed.get(&entity).copied().unwrap_or(false)
    )
}

fn supports_displayed_flag(props: &NodeProps) -> bool {
    matches!(
        props,
        NodeProps::Character { .. }
            | NodeProps::Background { .. }
            | NodeProps::Object { .. }
            | NodeProps::Ground
            | NodeProps::Player { .. }
            | NodeProps::Npc { .. }
            | NodeProps::Terrain
    )
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

fn format_equipment_slot(
    label: &str,
    model: &Option<String>,
    anchor: &Option<String>,
    attachment: &Option<String>,
    attachment_anchor: &Option<String>,
) -> String {
    let anchor = anchor
        .as_ref()
        .map(|anchor| format!(" anchor={anchor}"))
        .unwrap_or_default();
    let attachment = attachment
        .as_ref()
        .map(|attachment| format!(" attachment={attachment}"))
        .unwrap_or_default();
    let attachment_anchor = attachment_anchor
        .as_ref()
        .map(|anchor| format!(" attachment_anchor={anchor}"))
        .unwrap_or_default();
    match model {
        Some(m) => format!("{label} \"{m}\"{anchor}{attachment}{attachment_anchor}"),
        None => format!("{label} (empty){anchor}{attachment}{attachment_anchor}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn format_player_node(
    label: &str,
    name: &str,
    is_local: bool,
    model_path: &Option<String>,
    skin_path: &Option<String>,
    display_scale: Option<f32>,
    pos: &str,
    displayed: &str,
) -> String {
    let tag = if is_local { " (local)" } else { "" };
    let assets = format_model_assets(model_path, skin_path, display_scale);
    format!("{label} \"{name}\"{tag}{assets}{pos}{displayed}")
}

#[allow(clippy::too_many_arguments)]
fn format_npc_node(
    label: &str,
    name: &str,
    display_id: Option<u32>,
    model_path: &Option<String>,
    skin_path: &Option<String>,
    display_scale: Option<f32>,
    pos: &str,
    displayed: &str,
) -> String {
    let disp = display_id
        .map(|d| format!(" display={d}"))
        .unwrap_or_default();
    let assets = format_model_assets(model_path, skin_path, display_scale);
    format!("{label} \"{name}\"{disp}{assets}{pos}{displayed}")
}

fn compute_displayed_nodes(
    tree: &SceneTree,
    global_transforms: &Query<&GlobalTransform>,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ray_cast: &mut MeshRayCast,
) -> HashMap<Entity, bool> {
    let Some((camera, camera_transform)) = active_camera(camera_query) else {
        return HashMap::new();
    };
    let mut displayed = HashMap::new();
    collect_displayed_nodes(
        &tree.root,
        camera,
        camera_transform,
        global_transforms,
        parent_query,
        aabb_query,
        ray_cast,
        &mut displayed,
    );
    displayed
}

#[allow(clippy::too_many_arguments)]
fn collect_displayed_nodes(
    node: &SceneNode,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    global_transforms: &Query<&GlobalTransform>,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    ray_cast: &mut MeshRayCast,
    displayed: &mut HashMap<Entity, bool>,
) {
    if let Some(entity) = node.entity
        && supports_displayed_flag(&node.props)
    {
        let is_displayed = node_is_displayed(
            entity,
            camera,
            camera_transform,
            global_transforms,
            parent_query,
            aabb_query,
            ray_cast,
        );
        displayed.insert(entity, is_displayed);
    }
    for child in &node.children {
        collect_displayed_nodes(
            child,
            camera,
            camera_transform,
            global_transforms,
            parent_query,
            aabb_query,
            ray_cast,
            displayed,
        );
    }
}

fn active_camera<'a>(
    camera_query: &'a Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<(&'a Camera, &'a GlobalTransform)> {
    camera_query.iter().find(|(camera, _)| camera.is_active)
}

fn node_is_displayed(
    entity: Entity,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    global_transforms: &Query<&GlobalTransform>,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    ray_cast: &mut MeshRayCast,
) -> bool {
    let Some(samples) = projected_sample_points(
        entity,
        camera,
        camera_transform,
        global_transforms,
        parent_query,
        aabb_query,
    ) else {
        return false;
    };
    let settings = MeshRayCastSettings::default();
    samples.into_iter().any(|viewport_point| {
        let Ok(ray) = camera.viewport_to_world(camera_transform, viewport_point) else {
            return false;
        };
        let Some((hit, _)) = ray_cast.cast_ray(ray, &settings).first() else {
            return false;
        };
        is_descendant_or_self(*hit, entity, parent_query)
    })
}

fn projected_sample_points(
    entity: Entity,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    global_transforms: &Query<&GlobalTransform>,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
) -> Option<Vec<Vec2>> {
    let viewport = camera.logical_viewport_rect()?;
    let (mut min, mut max, mut any) =
        projected_bounds(entity, camera, camera_transform, parent_query, aabb_query);
    if !any {
        (min, max, any) =
            fallback_projected_bounds(entity, camera, camera_transform, global_transforms);
    }
    if !any {
        return None;
    }
    min = min.max(viewport.min);
    max = max.min(viewport.max);
    if min.x > max.x || min.y > max.y {
        return None;
    }
    Some(sample_rect(min, max))
}

fn projected_bounds(
    entity: Entity,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    parent_query: &Query<&ChildOf>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
) -> (Vec2, Vec2, bool) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let mut any = false;
    for (candidate, aabb, transform) in aabb_query.iter() {
        if !is_descendant_or_self(candidate, entity, parent_query) {
            continue;
        }
        for point in world_aabb_corners(aabb, transform) {
            let Ok(projected) = camera.world_to_viewport_with_depth(camera_transform, point) else {
                continue;
            };
            if projected.z <= 0.0 {
                continue;
            }
            min = min.min(projected.truncate());
            max = max.max(projected.truncate());
            any = true;
        }
    }
    (min, max, any)
}

fn fallback_projected_bounds(
    entity: Entity,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    global_transforms: &Query<&GlobalTransform>,
) -> (Vec2, Vec2, bool) {
    let Ok(transform) = global_transforms.get(entity) else {
        return (Vec2::ZERO, Vec2::ZERO, false);
    };
    let Ok(projected) =
        camera.world_to_viewport_with_depth(camera_transform, transform.translation())
    else {
        return (Vec2::ZERO, Vec2::ZERO, false);
    };
    if projected.z <= 0.0 {
        return (Vec2::ZERO, Vec2::ZERO, false);
    }
    let point = projected.truncate();
    (point, point, true)
}

fn sample_rect(min: Vec2, max: Vec2) -> Vec<Vec2> {
    let center = (min + max) * 0.5;
    if (max - min).length_squared() < 4.0 {
        return vec![center];
    }
    vec![
        center,
        Vec2::new(min.x, min.y),
        Vec2::new(max.x, min.y),
        Vec2::new(min.x, max.y),
        Vec2::new(max.x, max.y),
        Vec2::new(center.x, min.y),
        Vec2::new(center.x, max.y),
        Vec2::new(min.x, center.y),
        Vec2::new(max.x, center.y),
    ]
}

fn world_aabb_corners(aabb: &Aabb, transform: &GlobalTransform) -> [Vec3; 8] {
    let c: Vec3 = aabb.center.into();
    let e: Vec3 = aabb.half_extents.into();
    let p = transform.affine();
    [
        point(c, e, -1.0, -1.0, -1.0),
        point(c, e, -1.0, -1.0, 1.0),
        point(c, e, -1.0, 1.0, -1.0),
        point(c, e, -1.0, 1.0, 1.0),
        point(c, e, 1.0, -1.0, -1.0),
        point(c, e, 1.0, -1.0, 1.0),
        point(c, e, 1.0, 1.0, -1.0),
        point(c, e, 1.0, 1.0, 1.0),
    ]
    .map(|v| p.transform_point3(v))
}

fn point(center: Vec3, extents: Vec3, sx: f32, sy: f32, sz: f32) -> Vec3 {
    Vec3::new(
        center.x + extents.x * sx,
        center.y + extents.y * sy,
        center.z + extents.z * sz,
    )
}

fn is_descendant_or_self(
    candidate: Entity,
    ancestor: Entity,
    parent_query: &Query<&ChildOf>,
) -> bool {
    if candidate == ancestor {
        return true;
    }
    let mut current = candidate;
    while let Ok(parent) = parent_query.get(current) {
        current = parent.parent();
        if current == ancestor {
            return true;
        }
    }
    false
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
