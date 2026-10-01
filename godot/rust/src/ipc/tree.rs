//! Live Godot hierarchy and renderer-semantic scene diagnostics.

use game_engine_network::ipc_wire::Response;
use godot::{
    classes::{Camera3D, CanvasItem, Control, Light3D, MeshInstance3D, Node, Node2D, Node3D},
    prelude::*,
};

pub(super) fn scene_root(client: &Gd<Node>) -> Result<Gd<Node>, String> {
    let tree = client
        .get_tree()
        .ok_or("native IPC: client is outside the scene tree")?;
    Ok(tree.get_root().upcast())
}

pub(super) fn dump_tree(client: &Gd<Node>, filter: Option<&str>) -> Response {
    let root = match scene_root(client) {
        Ok(root) => root,
        Err(error) => return Response::Error(error),
    };
    let mut lines = Vec::new();
    emit_node(
        &root,
        0,
        filter.map(str::to_lowercase).as_deref(),
        &mut lines,
    );
    Response::Tree(lines.join("\n"))
}

fn emit_node(node: &Gd<Node>, depth: usize, filter: Option<&str>, lines: &mut Vec<String>) {
    let name = node.get_name().to_string();
    if filter.is_none_or(|filter| name.to_lowercase().contains(filter)) {
        let hidden = if node_hidden(node) { " hidden" } else { "" };
        lines.push(format!(
            "{}{} ({:?}){}{}",
            "  ".repeat(depth),
            name,
            node.instance_id(),
            node_transform(node),
            hidden,
        ));
    }
    // Filtering is per node, not subtree pruning or ancestor propagation.
    for child in node.get_children().iter_shared() {
        emit_node(&child, depth + 1, filter, lines);
    }
}

fn node_hidden(node: &Gd<Node>) -> bool {
    if let Ok(spatial) = node.clone().try_cast::<Node3D>() {
        return !spatial.is_visible();
    }
    node.clone()
        .try_cast::<CanvasItem>()
        .is_ok_and(|canvas| !canvas.is_visible())
}

fn node_transform(node: &Gd<Node>) -> String {
    if let Ok(spatial) = node.clone().try_cast::<Node3D>() {
        return format_transform(spatial.get_position(), spatial.get_scale());
    }
    if let Ok(control) = node.clone().try_cast::<Control>() {
        return format_transform(
            lift_position(control.get_position()),
            lift_scale(control.get_scale()),
        );
    }
    if let Ok(canvas) = node.clone().try_cast::<Node2D>() {
        return format_transform(
            lift_position(canvas.get_position()),
            lift_scale(canvas.get_scale()),
        );
    }
    String::new()
}

fn lift_position(position: Vector2) -> Vector3 {
    Vector3::new(position.x, position.y, 0.0)
}

fn lift_scale(scale: Vector2) -> Vector3 {
    Vector3::new(scale.x, scale.y, 1.0)
}

fn format_transform(position: Vector3, scale: Vector3) -> String {
    let mut output = format!(
        " at ({:.1}, {:.1}, {:.1})",
        position.x, position.y, position.z
    );
    if (scale - Vector3::ONE).length_squared() > 1e-6 {
        output.push_str(&format!(
            " scale({:.1}, {:.1}, {:.1})",
            scale.x, scale.y, scale.z
        ));
    }
    output
}

pub(super) fn dump_scene(client: &Gd<Node>) -> Response {
    let root = match scene_root(client) {
        Ok(root) => root,
        Err(error) => return Response::Error(error),
    };
    let mut lines = vec![format!("Scene \"{}\"", root.get_name())];
    for child in root.get_children().iter_shared() {
        lines.extend(semantic_subtree(&child, 1));
    }
    Response::Tree(lines.join("\n"))
}

fn semantic_subtree(node: &Gd<Node>, depth: usize) -> Vec<String> {
    let label = semantic_label(node);
    let child_depth = depth + usize::from(label.is_some());
    let mut children = Vec::new();
    for child in node.get_children().iter_shared() {
        children.extend(semantic_subtree(&child, child_depth));
    }
    let Some(label) = label else {
        // Non-rendering UI/container nodes do not become invented semantic objects.
        return children;
    };
    let mut lines = vec![format!("{}{label}", "  ".repeat(depth))];
    lines.extend(children);
    lines
}

fn semantic_label(node: &Gd<Node>) -> Option<String> {
    let name = node.get_name().to_string();
    if let Ok(camera) = node.clone().try_cast::<Camera3D>() {
        return Some(format!(
            "Camera \"{name}\" fov={}{} current={}",
            camera.get_fov(),
            semantic_position(&camera.clone().upcast()),
            camera.is_current(),
        ));
    }
    if let Ok(light) = node.clone().try_cast::<Light3D>() {
        return Some(format!(
            "Light \"{name}\" {}={}{}",
            light.get_class(),
            light.get_param(godot::classes::light_3d::Param::ENERGY),
            semantic_position(&light.upcast()),
        ));
    }
    if let Ok(mesh) = node.clone().try_cast::<MeshInstance3D>() {
        let surfaces = mesh.get_mesh().map(|resource| resource.get_surface_count());
        return Some(format!(
            "Object \"{name}\" MeshInstance3D{} surfaces={surfaces:?} is_displayed={}",
            semantic_position(&mesh.clone().upcast()),
            mesh.is_visible_in_tree(),
        ));
    }
    let spatial = node.clone().try_cast::<Node3D>().ok()?;
    if node.has_meta(crate::assets::M2_BOUNDS_META) {
        return Some(format!(
            "Model \"{name}\"{} bounds={} is_displayed={}",
            semantic_position(&spatial),
            node.get_meta(crate::assets::M2_BOUNDS_META),
            spatial.is_visible_in_tree(),
        ));
    }
    // Live spatial group names retain the native scene's ownership hierarchy.
    Some(format!("{name}{}", semantic_position(&spatial)))
}

fn semantic_position(node: &Gd<Node3D>) -> String {
    let position = node.get_position();
    format!(
        " @ ({:.1}, {:.1}, {:.1})",
        position.x, position.y, position.z
    )
}
