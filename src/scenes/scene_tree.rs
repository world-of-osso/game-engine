use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Semantic scene tree for high-level introspection.
#[derive(Resource)]
pub struct SceneTree {
    pub root: SceneNode,
}

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub label: String,
    pub entity: Option<Entity>,
    pub props: NodeProps,
    pub children: Vec<SceneNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeProps {
    Scene,
    Character {
        model: String,
        race: String,
        gender: String,
    },
    Background {
        model: String,
        doodad_count: usize,
    },
    Object {
        kind: String,
        model: String,
    },
    Ground,
    Camera {
        fov: f32,
    },
    Light {
        kind: String,
        intensity: f32,
    },
    EquipmentSlot {
        slot: String,
        model: Option<String>,
        anchor: Option<String>,
        attachment: Option<String>,
        attachment_anchor: Option<String>,
    },
    Player {
        name: String,
        is_local: bool,
        model_path: Option<String>,
        skin_path: Option<String>,
        display_scale: Option<f32>,
    },
    Npc {
        name: String,
        display_id: Option<u32>,
        model_path: Option<String>,
        skin_path: Option<String>,
        display_scale: Option<f32>,
    },
    Terrain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneSnapshot {
    pub root: SceneSnapshotNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneSnapshotNode {
    pub label: String,
    pub transform: Option<SceneNodeTransform>,
    pub props: NodeProps,
    pub children: Vec<SceneSnapshotNode>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SceneNodeTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl SceneNodeTransform {
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            translation: transform.translation.to_array(),
            rotation: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
        }
    }
}

pub fn snapshot_scene_tree(tree: &SceneTree, transforms: &Query<&Transform>) -> SceneSnapshot {
    SceneSnapshot {
        root: snapshot_scene_node(&tree.root, transforms),
    }
}

fn snapshot_scene_node(node: &SceneNode, transforms: &Query<&Transform>) -> SceneSnapshotNode {
    SceneSnapshotNode {
        label: node.label.clone(),
        transform: node
            .entity
            .and_then(|entity| transforms.get(entity).ok())
            .map(SceneNodeTransform::from_transform),
        props: node.props.clone(),
        children: node
            .children
            .iter()
            .map(|child| snapshot_scene_node(child, transforms))
            .collect(),
    }
}

pub fn scene_tree_from_snapshot(snapshot: SceneSnapshot) -> SceneTree {
    SceneTree {
        root: scene_node_from_snapshot(snapshot.root),
    }
}

fn scene_node_from_snapshot(node: SceneSnapshotNode) -> SceneNode {
    SceneNode {
        label: node.label,
        entity: None,
        props: node.props,
        children: node
            .children
            .into_iter()
            .map(scene_node_from_snapshot)
            .collect(),
    }
}

pub fn write_scene_snapshot_file(
    output_path: &Path,
    snapshot: &SceneSnapshot,
) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("failed to encode scene snapshot: {e}"))?;
    std::fs::write(output_path, serialized)
        .map_err(|e| format!("failed to write {}: {e}", output_path.display()))?;
    Ok(())
}

pub fn read_scene_snapshot_file(path: &Path) -> Result<SceneSnapshot, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("failed to parse scene snapshot {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_snapshot_file_roundtrips_json() {
        let output = std::env::temp_dir().join(format!(
            "game-engine-scene-snapshot-{}.json",
            std::process::id()
        ));
        let snapshot = SceneSnapshot {
            root: SceneSnapshotNode {
                label: "InWorldScene".into(),
                transform: None,
                props: NodeProps::Scene,
                children: vec![SceneSnapshotNode {
                    label: "Player".into(),
                    transform: Some(SceneNodeTransform {
                        translation: [1.0, 2.0, 3.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    }),
                    props: NodeProps::Player {
                        name: "Thrall".into(),
                        is_local: true,
                        model_path: Some("data/models/thrall.m2".into()),
                        skin_path: None,
                        display_scale: Some(1.0),
                    },
                    children: vec![],
                }],
            },
        };

        write_scene_snapshot_file(&output, &snapshot).expect("snapshot should write");
        let loaded = read_scene_snapshot_file(&output).expect("snapshot should load");

        assert_eq!(loaded, snapshot);

        let _ = std::fs::remove_file(output);
    }
}
