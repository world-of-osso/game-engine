//! Scene tree construction for the char-select 3D scene.

use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::char_select_scene::CharSelectScene;
use crate::m2_effect_material::M2EffectMaterial;
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

pub struct WarbandTerrainSpawnResult {
    pub root_entity: Entity,
    pub doodad_count: usize,
    pub wmo_entities: Vec<(Entity, String)>,
}

/// Spawn warband scene terrain from ADT tiles extracted via CASC.
#[allow(clippy::too_many_arguments)]
pub fn spawn_warband_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    _effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    _inv_bp: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    scene: &warband_scene::WarbandSceneEntry,
) -> Option<WarbandTerrainSpawnResult> {
    let Some(adt_path) = warband_scene::local_warband_terrain(scene) else {
        return None;
    };
    let root_entity = commands
        .spawn((
            Name::new("WarbandTerrain"),
            CharSelectScene,
            CharSelectTerrain,
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    let Ok(result) = terrain::spawn_adt_terrain_only(
        commands,
        meshes,
        materials,
        terrain_materials,
        water_materials,
        images,
        heightmap,
        &adt_path,
    ) else {
        commands.entity(root_entity).despawn();
        return None;
    };
    commands.entity(root_entity).add_child(result.root_entity);
    commands
        .entity(result.root_entity)
        .insert((CharSelectScene, CharSelectTerrain));
    commands
        .entity(root_entity)
        .insert((CharSelectScene, CharSelectTerrain));
    Some(WarbandTerrainSpawnResult {
        root_entity,
        doodad_count: 0,
        wmo_entities: Vec::new(),
    })
}

/// Build the background scene node (terrain or fallback ground).
pub fn background_scene_node(
    entity: Entity,
    label: &str,
    doodad_count: usize,
    wmos: Vec<SceneNode>,
) -> SceneNode {
    SceneNode {
        label: "Background".into(),
        entity: Some(entity),
        props: NodeProps::Background {
            model: label.to_string(),
            doodad_count,
        },
        children: wmos,
    }
}

pub fn wmo_scene_node(entity: Entity, model: String) -> SceneNode {
    SceneNode {
        label: "Object".into(),
        entity: Some(entity),
        props: NodeProps::Object {
            kind: "WMO".into(),
            model,
        },
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
        props: NodeProps::Character {
            model,
            race,
            gender,
        },
        children: vec![
            SceneNode {
                label: "Slot:Head".into(),
                entity: None,
                props: NodeProps::EquipmentSlot {
                    slot: "Head".into(),
                    model: None,
                },
                children: vec![],
            },
            SceneNode {
                label: "Slot:MainHand".into(),
                entity: None,
                props: NodeProps::EquipmentSlot {
                    slot: "MainHand".into(),
                    model: None,
                },
                children: vec![],
            },
        ],
    }
}

pub fn light_scene_nodes(
    camera: Entity,
    fov: f32,
    ambient: Option<Entity>,
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
            entity: ambient,
            props: NodeProps::Light {
                kind: "ambient".into(),
                intensity: 80.0,
            },
            children: vec![],
        },
        SceneNode {
            label: "DirectionalLight".into(),
            entity: Some(directional),
            props: NodeProps::Light {
                kind: "directional".into(),
                intensity: 8000.0,
            },
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
