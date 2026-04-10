//! Scene tree construction for the char-select 3D scene.

use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::m2_effect_material::M2EffectMaterial;
use crate::scenes::char_select::scene::CharSelectScene;
use crate::scenes::char_select::warband;
use crate::terrain;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::terrain_objects;
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

pub struct WarbandTerrainSpawnContext<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub terrain_materials: &'a mut Assets<TerrainMaterial>,
    pub water_materials: &'a mut Assets<WaterMaterial>,
    pub images: &'a mut Assets<Image>,
    pub inv_bp: &'a mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    pub heightmap: &'a mut TerrainHeightmap,
}

const CHAR_SELECT_PRIMARY_DOODAD_RADIUS: f32 = 75.0;
const CHAR_SELECT_PRIMARY_WMO_RADIUS: f32 = 120.0;

/// Spawn warband scene terrain from ADT tiles extracted via CASC.
pub fn spawn_warband_terrain(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    scene: &warband::WarbandSceneEntry,
    focus: Vec3,
) -> Option<WarbandTerrainSpawnResult> {
    let root_entity = spawn_warband_terrain_root(ctx.commands);
    let primary = spawn_warband_primary_terrain(ctx, scene, root_entity)?;
    Some(finalize_warband_terrain_spawn(ctx, primary, focus))
}

struct WarbandPrimaryTerrain {
    adt_path: std::path::PathBuf,
    root_entity: Entity,
    terrain: terrain::AdtSpawnResult,
}

fn spawn_warband_primary_terrain(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    scene: &warband::WarbandSceneEntry,
    root_entity: Entity,
) -> Option<WarbandPrimaryTerrain> {
    let adt_path = warband::ensure_warband_terrain(scene)?;
    let Some(terrain) = spawn_warband_terrain_tile(ctx, &adt_path) else {
        ctx.commands.entity(root_entity).despawn();
        return None;
    };
    Some(WarbandPrimaryTerrain {
        adt_path,
        root_entity,
        terrain,
    })
}

fn finalize_warband_terrain_spawn(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    primary: WarbandPrimaryTerrain,
    focus: Vec3,
) -> WarbandTerrainSpawnResult {
    attach_warband_terrain_root(
        ctx.commands,
        primary.root_entity,
        primary.terrain.root_entity,
    );
    let (doodad_count, wmo_entities) = spawn_warband_primary_objects(
        ctx,
        &primary.adt_path,
        focus,
        primary.terrain.tile_y,
        primary.terrain.tile_x,
    );
    WarbandTerrainSpawnResult {
        root_entity: primary.root_entity,
        doodad_count,
        wmo_entities,
    }
}

fn spawn_warband_primary_objects(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    adt_path: &std::path::Path,
    focus: Vec3,
    tile_y: u32,
    tile_x: u32,
) -> (usize, Vec<(Entity, String)>) {
    let Some(obj_data) = load_warband_primary_object_data(adt_path) else {
        return (0, Vec::new());
    };
    build_warband_primary_object_result(spawn_warband_primary_nearby_objects(
        ctx, &obj_data, focus, tile_y, tile_x,
    ))
}

fn load_warband_primary_object_data(
    adt_path: &std::path::Path,
) -> Option<crate::asset::adt_format::adt_obj::AdtObjData> {
    terrain_objects::load_obj0(adt_path)
}

fn spawn_warband_primary_nearby_objects(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    obj_data: &crate::asset::adt_format::adt_obj::AdtObjData,
    focus: Vec3,
    tile_y: u32,
    tile_x: u32,
) -> terrain_objects::SpawnedTerrainObjects {
    terrain_objects::spawn_nearby_campsite_objects(
        ctx.commands,
        ctx.meshes,
        ctx.materials,
        ctx.effect_materials,
        ctx.water_materials,
        ctx.images,
        ctx.inv_bp,
        Some(ctx.heightmap),
        tile_y,
        tile_x,
        obj_data,
        focus,
        CHAR_SELECT_PRIMARY_DOODAD_RADIUS,
        CHAR_SELECT_PRIMARY_WMO_RADIUS,
    )
}

fn build_warband_primary_object_result(
    spawned_objects: terrain_objects::SpawnedTerrainObjects,
) -> (usize, Vec<(Entity, String)>) {
    (
        spawned_objects.doodads.len(),
        spawned_objects
            .wmos
            .iter()
            .map(|wmo| (wmo.entity, wmo.model.clone()))
            .collect(),
    )
}

fn attach_warband_terrain_root(commands: &mut Commands, root_entity: Entity, terrain_root: Entity) {
    commands.entity(root_entity).add_child(terrain_root);
    commands
        .entity(root_entity)
        .insert((CharSelectScene, CharSelectTerrain));
}

fn spawn_warband_terrain_tile(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    adt_path: &std::path::Path,
) -> Option<terrain::AdtSpawnResult> {
    let mut terrain_assets = terrain::TerrainOnlySpawnAssets {
        commands: ctx.commands,
        meshes: ctx.meshes,
        terrain_materials: ctx.terrain_materials,
        water_materials: ctx.water_materials,
        images: ctx.images,
    };
    terrain::spawn_adt_terrain_only(&mut terrain_assets, ctx.heightmap, adt_path).ok()
}

fn spawn_warband_terrain_root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("WarbandTerrain"),
            CharSelectScene,
            CharSelectTerrain,
            Transform::default(),
            Visibility::default(),
        ))
        .id()
}

pub fn spawn_warband_supplemental_terrain(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    scene: &warband::WarbandSceneEntry,
    root_entity: Entity,
) -> usize {
    let mut doodad_count = 0;
    for (tile_y, tile_x) in warband::supplemental_terrain_tile_coords(scene) {
        let adt_path = supplemental_terrain_path(scene, tile_y, tile_x);
        let Some(result) = spawn_warband_supplemental_tile(ctx, &adt_path) else {
            continue;
        };
        attach_warband_terrain_root(ctx.commands, root_entity, result.root_entity);
        doodad_count += spawn_warband_supplemental_doodads(ctx, &adt_path, &result);
    }
    doodad_count
}

fn supplemental_terrain_path(
    scene: &warband::WarbandSceneEntry,
    tile_y: u32,
    tile_x: u32,
) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "data/terrain/{}_{}_{}.adt",
        scene.map_name(),
        tile_y,
        tile_x
    ))
}

fn spawn_warband_supplemental_tile(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    adt_path: &std::path::Path,
) -> Option<terrain::AdtSpawnResult> {
    let mut terrain_assets = terrain::TerrainOnlySpawnAssets {
        commands: ctx.commands,
        meshes: ctx.meshes,
        terrain_materials: ctx.terrain_materials,
        water_materials: ctx.water_materials,
        images: ctx.images,
    };
    terrain::spawn_adt_terrain_only(&mut terrain_assets, ctx.heightmap, adt_path).ok()
}

fn spawn_warband_supplemental_doodads(
    ctx: &mut WarbandTerrainSpawnContext<'_, '_, '_>,
    adt_path: &std::path::Path,
    result: &terrain::AdtSpawnResult,
) -> usize {
    let Some(obj_data) = terrain_objects::load_obj0(adt_path) else {
        return 0;
    };
    terrain_objects::spawn_waterfall_backdrop_doodads(
        ctx.commands,
        ctx.meshes,
        ctx.materials,
        ctx.effect_materials,
        ctx.water_materials,
        ctx.images,
        ctx.inv_bp,
        Some(ctx.heightmap),
        result.tile_y,
        result.tile_x,
        &obj_data,
    )
    .len()
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
    name: Option<String>,
    character_id: Option<u64>,
) -> SceneNode {
    SceneNode {
        label: "Character".into(),
        entity: Some(entity),
        props: NodeProps::Character {
            model,
            race,
            gender,
            name,
            character_id,
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
    fill_light: Option<Entity>,
) -> Vec<SceneNode> {
    let mut nodes = vec![
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
                kind: "spot".into(),
                intensity: 220000.0,
            },
            children: vec![],
        },
    ];
    if let Some(fill_light) = fill_light {
        nodes.push(SceneNode {
            label: "FillLight".into(),
            entity: Some(fill_light),
            props: NodeProps::Light {
                kind: "directional".into(),
                intensity:
                    crate::scenes::char_select::scene::lighting::CHAR_SELECT_FILL_LIGHT_ILLUMINANCE,
            },
            children: vec![],
        });
    }
    nodes
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

    #[test]
    fn light_scene_nodes_report_spot_primary_light() {
        let camera = Entity::from_raw_u32(1).expect("valid entity id");
        let light = Entity::from_raw_u32(2).expect("valid entity id");
        let fill = Entity::from_raw_u32(3).expect("valid entity id");
        let nodes = light_scene_nodes(camera, 45.0, None, 150.0, light, Some(fill));

        assert_eq!(nodes[2].label, "PrimaryLight");
        assert_eq!(
            nodes[2].props,
            NodeProps::Light {
                kind: "spot".into(),
                intensity: 220000.0,
            }
        );
        assert_eq!(nodes[3].label, "FillLight");
        assert_eq!(
            nodes[3].props,
            NodeProps::Light {
                kind: "directional".into(),
                intensity:
                    crate::scenes::char_select::scene::lighting::CHAR_SELECT_FILL_LIGHT_ILLUMINANCE,
            }
        );
    }
}
