// Original renderer-independent scene export representation and JSON file contract.
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeProps {
    Scene,
    Character {
        model: String,
        race: String,
        gender: String,
        name: Option<String>,
        character_id: Option<u64>,
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
