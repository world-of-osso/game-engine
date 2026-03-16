//! Scene spawning helpers for the game-engine binary.

use std::f32::consts::PI;
use std::path::Path;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::camera::Player;
use crate::creature_display;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::networking;
use crate::sky;
use crate::terrain::{self, AdtManager};
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material;
use crate::water_material;

pub const DEFAULT_M2: &str = "data/models/humanmale_hd.m2";
pub const DEFAULT_ADT: &str = "data/terrain/azeroth_32_48.adt";

pub fn should_load_explicit_scene_at_startup(server_mode: bool, asset_path: Option<&Path>) -> bool {
    !server_mode && asset_path.is_some()
}

#[allow(clippy::too_many_arguments)]
pub fn setup_world_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    sky_mats: &mut Assets<sky::SkyMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    creature_display_map: &creature_display::CreatureDisplayMap,
    asset_path: Option<&Path>,
) {
    let is_terrain = asset_path.is_some_and(|p| p.extension().is_some_and(|e| e == "adt"))
        || asset_path.is_none();
    let camera = spawn_scene_environment(commands, meshes, materials, sky_mats, images, is_terrain);
    match asset_path {
        Some(p) if p.extension().is_some_and(|e| e == "adt") => {
            let center = spawn_terrain(
                commands,
                meshes,
                materials,
                effect_materials,
                terrain_mats,
                water_mats,
                images,
                inverse_bp,
                heightmap,
                adt_manager,
                camera,
                p,
            );
            let m2_path = Path::new(DEFAULT_M2);
            if m2_path.exists() {
                m2_scene::spawn_m2_model(
                    commands,
                    meshes,
                    materials,
                    effect_materials,
                    images,
                    inverse_bp,
                    m2_path,
                    creature_display_map,
                );
            }
            if let Some(pos) = center {
                set_player_position(commands, pos);
            }
        }
        Some(p) => spawn_m2_scene(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bp,
            p,
            creature_display_map,
        ),
        None => spawn_default_scene(
            commands,
            meshes,
            materials,
            effect_materials,
            terrain_mats,
            water_mats,
            images,
            inverse_bp,
            heightmap,
            adt_manager,
            creature_display_map,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_explicit_asset_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut terrain_mats: ResMut<Assets<terrain_material::TerrainMaterial>>,
    mut water_mats: ResMut<Assets<water_material::WaterMaterial>>,
    mut sky_mats: ResMut<Assets<sky::SkyMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut heightmap: ResMut<TerrainHeightmap>,
    mut adt_manager: ResMut<AdtManager>,
    server_addr: Option<Res<networking::ServerAddr>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
) {
    let asset_path = crate::parse_asset_path();
    if !should_load_explicit_scene_at_startup(server_addr.is_some(), asset_path.as_deref()) {
        return;
    }
    setup_world_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut terrain_mats,
        &mut water_mats,
        &mut sky_mats,
        &mut images,
        &mut inverse_bp,
        &mut heightmap,
        &mut adt_manager,
        &creature_display_map,
        asset_path.as_deref(),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn setup_default_world_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut terrain_mats: ResMut<Assets<terrain_material::TerrainMaterial>>,
    mut water_mats: ResMut<Assets<water_material::WaterMaterial>>,
    mut sky_mats: ResMut<Assets<sky::SkyMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut heightmap: ResMut<TerrainHeightmap>,
    mut adt_manager: ResMut<AdtManager>,
    server_addr: Option<Res<networking::ServerAddr>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
) {
    if server_addr.is_some() || crate::parse_asset_path().is_some() {
        return;
    }
    setup_world_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut terrain_mats,
        &mut water_mats,
        &mut sky_mats,
        &mut images,
        &mut inverse_bp,
        &mut heightmap,
        &mut adt_manager,
        &creature_display_map,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    camera: Entity,
    adt_path: &Path,
) -> Option<Vec3> {
    match terrain::spawn_adt(
        commands,
        meshes,
        materials,
        effect_materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_path,
    ) {
        Ok(result) => {
            commands.entity(camera).insert(result.camera);
            adt_manager.map_name = result.map_name;
            adt_manager.initial_tile = (result.tile_y, result.tile_x);
            adt_manager
                .loaded
                .insert((result.tile_y, result.tile_x), result.root_entity);
            Some(result.center)
        }
        Err(e) => {
            eprintln!("ADT load error: {e}");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn load_default_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
) -> Option<Vec3> {
    let adt_path = Path::new(DEFAULT_ADT);
    if !adt_path.exists() {
        return None;
    }
    match terrain::spawn_adt(
        commands,
        meshes,
        materials,
        effect_materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_path,
    ) {
        Ok(result) => {
            adt_manager.map_name = result.map_name;
            adt_manager.initial_tile = (result.tile_y, result.tile_x);
            adt_manager
                .loaded
                .insert((result.tile_y, result.tile_x), result.root_entity);
            Some(result.center)
        }
        Err(e) => {
            eprintln!("ADT load error: {e}");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_default_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let center = load_default_terrain(
        commands,
        meshes,
        materials,
        effect_materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_manager,
    );
    let m2_path = Path::new(DEFAULT_M2);
    if m2_path.exists() {
        m2_scene::spawn_m2_model(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bp,
            m2_path,
            creature_display_map,
        );
    }
    if let Some(pos) = center {
        set_player_position(commands, pos);
    }
}

pub fn set_player_position(commands: &mut Commands, pos: Vec3) {
    commands.queue(move |world: &mut World| {
        let mut q = world.query_filtered::<&mut Transform, With<Player>>();
        for mut xf in q.iter_mut(world) {
            xf.translation = pos;
        }
    });
}

fn spawn_m2_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    m2_scene::spawn_m2_model(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bindposes,
        m2_path,
        creature_display_map,
    );
    ground::spawn_ground_clutter(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bindposes,
        creature_display_map,
    );
}

pub fn spawn_scene_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sky_materials: &mut Assets<sky::SkyMaterial>,
    images: &mut Assets<Image>,
    is_terrain: bool,
) -> Entity {
    let camera = crate::camera::spawn_wow_camera(commands);
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 150.0,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
    ));
    sky::spawn_sky_dome(commands, meshes, sky_materials, images, camera);
    if !is_terrain {
        ground::spawn_ground_plane(commands, meshes, materials, images);
    }
    camera
}
