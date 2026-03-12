use bevy::prelude::*;

/// Semantic scene tree for high-level introspection.
#[derive(Resource)]
pub struct SceneTree {
    pub root: SceneNode,
}

pub struct SceneNode {
    pub label: String,
    pub entity: Option<Entity>,
    pub props: NodeProps,
    pub children: Vec<SceneNode>,
}

pub enum NodeProps {
    Scene,
    Character {
        model: String,
        race: String,
        gender: String,
    },
    Background {
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
    },
    Player {
        name: String,
        is_local: bool,
    },
    Npc {
        name: String,
        display_id: Option<u32>,
    },
    Terrain,
}
