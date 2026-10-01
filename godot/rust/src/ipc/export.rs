//! Legacy scene snapshots from selected live native rendering nodes.

use std::path::Path;

use game_engine_core::scene_snapshot::{
    NodeProps, SceneNodeTransform, SceneSnapshot, SceneSnapshotNode, write_scene_snapshot_file,
};
use game_engine_network::ipc_wire::Response;
use godot::{
    classes::{Camera3D, Light3D, Node, Node3D, light_3d::Param},
    prelude::*,
};

pub(super) fn export_scene(client: &Gd<Node>, output_path: &str) -> Response {
    let result = read_scene_snapshot(client)
        .and_then(|snapshot| write_scene_snapshot_file(Path::new(output_path), &snapshot));
    match result {
        Ok(()) => Response::Text(format!("scene exported to {output_path}")),
        Err(error) => Response::Error(error),
    }
}

fn read_scene_snapshot(client: &Gd<Node>) -> Result<SceneSnapshot, String> {
    let root = super::tree::scene_root(client)?;
    Ok(SceneSnapshot {
        root: SceneSnapshotNode {
            label: root.get_name().to_string(),
            transform: None,
            props: NodeProps::Scene,
            children: read_semantic_children(&root, Transform3D::IDENTITY)?,
        },
    })
}

fn read_semantic_children(
    node: &Gd<Node>,
    accumulated: Transform3D,
) -> Result<Vec<SceneSnapshotNode>, String> {
    let subtrees = node
        .get_children()
        .iter_shared()
        .map(|child| read_semantic_subtree(&child, accumulated))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(subtrees.into_iter().flatten().collect())
}

fn read_semantic_subtree(
    node: &Gd<Node>,
    accumulated: Transform3D,
) -> Result<Vec<SceneSnapshotNode>, String> {
    let transform = accumulated * read_local_transform(node);
    let Some(props) = read_semantic_props(node)? else {
        return read_semantic_children(node, transform);
    };
    Ok(vec![SceneSnapshotNode {
        label: node.get_name().to_string(),
        transform: Some(snapshot_transform(transform)),
        props,
        children: read_semantic_children(node, Transform3D::IDENTITY)?,
    }])
}

fn read_local_transform(node: &Gd<Node>) -> Transform3D {
    match node.clone().try_cast::<Node3D>() {
        Ok(spatial) => spatial.get_transform(),
        // UI and implementation containers contribute no spatial transform.
        Err(_) => Transform3D::IDENTITY,
    }
}

fn read_semantic_props(node: &Gd<Node>) -> Result<Option<NodeProps>, String> {
    if let Ok(camera) = node.clone().try_cast::<Camera3D>() {
        return Ok(Some(NodeProps::Camera {
            fov: camera.get_fov(),
        }));
    }
    if let Ok(light) = node.clone().try_cast::<Light3D>() {
        return Ok(Some(NodeProps::Light {
            kind: light.get_class().to_string(),
            intensity: light.get_param(Param::ENERGY),
        }));
    }
    if node.has_meta(crate::assets::M2_SOURCE_META) {
        return read_m2_props(node).map(Some);
    }
    Ok(None)
}

fn read_m2_props(node: &Gd<Node>) -> Result<NodeProps, String> {
    let label = node.get_name();
    node.clone()
        .try_cast::<Node3D>()
        .map_err(|_| format!("native IPC: M2 source metadata on non-spatial node {label}"))?;
    let source = node
        .get_meta(crate::assets::M2_SOURCE_META)
        .try_to::<GString>()
        .map_err(|_| {
            format!("native IPC: invalid M2 source metadata on {label}: expected String")
        })?;
    let model = source.to_string();
    if model.is_empty() {
        return Err(format!("native IPC: empty M2 source metadata on {label}"));
    }
    Ok(NodeProps::Object {
        kind: "M2".into(),
        model,
    })
}

fn snapshot_transform(transform: Transform3D) -> SceneNodeTransform {
    let translation = transform.origin;
    // godot-rust names Godot's get_rotation_quaternion() get_quaternion().
    let rotation = transform.basis.get_quaternion();
    let scale = transform.basis.get_scale();
    SceneNodeTransform {
        translation: [translation.x, translation.y, translation.z],
        rotation: [rotation.x, rotation.y, rotation.z, rotation.w],
        scale: [scale.x, scale.y, scale.z],
    }
}
