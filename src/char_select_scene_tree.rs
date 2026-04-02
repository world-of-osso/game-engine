//! Scene tree construction for the char-select 3D scene.

use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::char_select_scene::CharSelectScene;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::terrain_objects;
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

const CHAR_SELECT_PRIMARY_DOODAD_RADIUS: f32 = 75.0;
const CHAR_SELECT_PRIMARY_WMO_RADIUS: f32 = 120.0;

/// Spawn warband scene terrain from ADT tiles extracted via CASC.
#[allow(clippy::too_many_arguments)]
pub fn spawn_warband_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    scene: &warband_scene::WarbandSceneEntry,
    focus: Vec3,
) -> Option<WarbandTerrainSpawnResult> {
    let Some(adt_path) = warband_scene::ensure_warband_terrain(scene) else {
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
    let mut doodad_count = 0;
    let mut wmo_entities = Vec::new();
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
    if let Some(obj_data) = terrain_objects::load_obj0(&adt_path) {
        let spawned_objects = terrain_objects::spawn_nearby_campsite_objects(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inv_bp,
            Some(heightmap),
            result.tile_y,
            result.tile_x,
            &obj_data,
            focus,
            CHAR_SELECT_PRIMARY_DOODAD_RADIUS,
            CHAR_SELECT_PRIMARY_WMO_RADIUS,
        );
        doodad_count += spawned_objects.doodads.len();
        wmo_entities.extend(
            spawned_objects
                .wmos
                .iter()
                .map(|wmo| (wmo.entity, wmo.model.clone())),
        );
    }
    commands
        .entity(root_entity)
        .insert((CharSelectScene, CharSelectTerrain));
    Some(WarbandTerrainSpawnResult {
        root_entity,
        doodad_count,
        wmo_entities,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_warband_supplemental_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    scene: &warband_scene::WarbandSceneEntry,
    root_entity: Entity,
) -> usize {
    let mut doodad_count = 0;
    for (tile_y, tile_x) in warband_scene::supplemental_terrain_tile_coords(scene) {
        let adt_path = std::path::PathBuf::from(format!(
            "data/terrain/{}_{}_{}.adt",
            scene.map_name(),
            tile_y,
            tile_x
        ));
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
            continue;
        };
        commands.entity(root_entity).add_child(result.root_entity);
        if let Some(obj_data) = terrain_objects::load_obj0(&adt_path) {
            doodad_count += terrain_objects::spawn_waterfall_backdrop_doodads(
                commands,
                meshes,
                materials,
                effect_materials,
                images,
                inv_bp,
                Some(heightmap),
                result.tile_y,
                result.tile_x,
                &obj_data,
            )
            .len();
        }
    }
    doodad_count
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

pub fn skybox_scene_node(entity: Entity, model: String) -> SceneNode {
    SceneNode {
        label: "Skybox".into(),
        entity: Some(entity),
        props: NodeProps::Object {
            kind: "Skybox".into(),
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
                    anchor: None,
                    attachment: None,
                    attachment_anchor: None,
                },
                children: vec![],
            },
            SceneNode {
                label: "Slot:MainHand".into(),
                entity: None,
                props: NodeProps::EquipmentSlot {
                    slot: "MainHand".into(),
                    model: None,
                    anchor: None,
                    attachment: None,
                    attachment_anchor: None,
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
    ambient_intensity: f32,
    primary_light: Entity,
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
                intensity: ambient_intensity,
            },
            children: vec![],
        },
        SceneNode {
            label: "PrimaryLight".into(),
            entity: Some(primary_light),
            props: NodeProps::Light {
                kind: "point".into(),
                intensity: 220000.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skybox_scene_node_uses_skybox_object_kind() {
        let entity = Entity::from_raw_u32(42).expect("valid entity id");
        let node = skybox_scene_node(entity, "data/models/sky.m2".into());

        assert_eq!(node.label, "Skybox");
        assert_eq!(node.entity, Some(entity));
        assert_eq!(
            node.props,
            NodeProps::Object {
                kind: "Skybox".into(),
                model: "data/models/sky.m2".into(),
            }
        );
        assert!(node.children.is_empty());
    }
}
