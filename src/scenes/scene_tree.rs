use bevy::prelude::*;

include!("scene_snapshot_data.rs");

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
