//! Scene tree construction for the char-select 3D scene.

use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::char_select_scene::CharSelectScene;
use crate::terrain;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::warband_scene;
use crate::water_material::WaterMaterial;

/// Marker for terrain entities in char-select (for selective teardown on scene switch).
#[derive(Component)]
pub struct CharSelectTerrain;

/// Tracks which warband scene is currently rendered.
#[derive(Resource, Default)]
pub struct ActiveWarbandSceneId(pub Option<u32>);

/// Spawn warband scene terrain from ADT tiles extracted via CASC.
pub fn spawn_warband_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    scene: &warband_scene::WarbandSceneEntry,
) -> Option<Entity> {
    let adt_path = warband_scene::ensure_warband_terrain(scene)?;
    let mut heightmap = TerrainHeightmap::default();
    let result = terrain::spawn_adt(
        commands, meshes, materials, terrain_materials,
        water_materials, images, inv_bp, &mut heightmap, &adt_path,
    )
    .ok()?;
    commands
        .entity(result.root_entity)
        .insert((CharSelectScene, CharSelectTerrain));
    Some(result.root_entity)
}

/// Build the background scene node (terrain or fallback ground).
pub fn background_scene_node(entity: Entity, label: &str) -> SceneNode {
    SceneNode {
        label: "Background".into(),
        entity: Some(entity),
        props: NodeProps::Background { model: label.to_string() },
        children: vec![],
    }
}

pub fn ground_scene_node(entity: Entity) -> SceneNode {
    SceneNode {
        label: "Ground".into(),
        entity: Some(entity),
        props: NodeProps::Ground,
        children: vec![],
    }
}

pub fn character_scene_node(
    entity: Entity,
    model: String,
    race: String,
    gender: String,
) -> SceneNode {
    SceneNode {
        label: "Character".into(),
        entity: Some(entity),
        props: NodeProps::Character { model, race, gender },
        children: vec![
            SceneNode {
                label: "Slot:Head".into(),
                entity: None,
                props: NodeProps::EquipmentSlot { slot: "Head".into(), model: None },
                children: vec![],
            },
            SceneNode {
                label: "Slot:MainHand".into(),
                entity: None,
                props: NodeProps::EquipmentSlot { slot: "MainHand".into(), model: None },
                children: vec![],
            },
        ],
    }
}

pub fn light_scene_nodes(
    camera: Entity,
    fov: f32,
    ambient: Entity,
    directional: Entity,
) -> Vec<SceneNode> {
    vec![
        SceneNode {
            label: "Camera".into(),
            entity: Some(camera),
            props: NodeProps::Camera { fov },
            children: vec![],
        },
        SceneNode {
            label: "AmbientLight".into(),
            entity: Some(ambient),
            props: NodeProps::Light { kind: "ambient".into(), intensity: 80.0 },
            children: vec![],
        },
        SceneNode {
            label: "DirectionalLight".into(),
            entity: Some(directional),
            props: NodeProps::Light { kind: "directional".into(), intensity: 8000.0 },
            children: vec![],
        },
    ]
}

pub fn build_scene_tree(children: Vec<SceneNode>) -> SceneTree {
    SceneTree {
        root: SceneNode {
            label: "CharSelectScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children,
        },
    }
}
