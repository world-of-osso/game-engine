//! Scene spawning helpers for the game-engine binary.

use std::f32::consts::PI;
use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use game_engine::paths;

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

#[derive(SystemParam)]
pub struct SceneSetupSystemParams<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    terrain_mats: ResMut<'w, Assets<terrain_material::TerrainMaterial>>,
    water_mats: ResMut<'w, Assets<water_material::WaterMaterial>>,
    sky_mats: ResMut<'w, Assets<sky::SkyMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inverse_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    heightmap: ResMut<'w, TerrainHeightmap>,
    adt_manager: ResMut<'w, AdtManager>,
    server_addr: Option<Res<'w, networking::ServerAddr>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
}

struct SceneSetupContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    terrain_mats: &'a mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &'a mut Assets<water_material::WaterMaterial>,
    sky_mats: &'a mut Assets<sky::SkyMaterial>,
    images: &'a mut Assets<Image>,
    inverse_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &'a mut TerrainHeightmap,
    adt_manager: &'a mut AdtManager,
    creature_display_map: &'a creature_display::CreatureDisplayMap,
}

impl<'a, 'w, 's> SceneSetupContext<'a, 'w, 's> {
    fn from_system_params(params: &'a mut SceneSetupSystemParams<'w, 's>) -> Self {
        Self {
            commands: &mut params.commands,
            meshes: &mut params.meshes,
            materials: &mut params.materials,
            effect_materials: &mut params.effect_materials,
            terrain_mats: &mut params.terrain_mats,
            water_mats: &mut params.water_mats,
            sky_mats: &mut params.sky_mats,
            images: &mut params.images,
            inverse_bp: &mut params.inverse_bp,
            heightmap: &mut params.heightmap,
            adt_manager: &mut params.adt_manager,
            creature_display_map: &params.creature_display_map,
        }
    }

    fn setup_world_scene(&mut self, asset_path: Option<&Path>) {
        let is_terrain = asset_path.is_some_and(|p| p.extension().is_some_and(|e| e == "adt"))
            || asset_path.is_none();
        let camera = spawn_scene_environment(
            self.commands,
            self.meshes,
            self.materials,
            self.sky_mats,
            self.images,
            is_terrain,
        );
        match asset_path {
            Some(p) if p.extension().is_some_and(|e| e == "adt") => {
                let center = self.spawn_terrain(camera, p);
                self.spawn_default_character_if_present();
                if let Some(pos) = center {
                    set_player_position(self.commands, pos);
                }
            }
            Some(p) => self.spawn_m2_scene(p),
            None => self.spawn_default_scene(),
        }
    }

    fn spawn_terrain(&mut self, camera: Entity, adt_path: &Path) -> Option<Vec3> {
        let mut assets = terrain::AdtSpawnAssets {
            commands: self.commands,
            meshes: self.meshes,
            materials: self.materials,
            effect_materials: self.effect_materials,
            terrain_materials: self.terrain_mats,
            water_materials: self.water_mats,
            images: self.images,
            inverse_bp: self.inverse_bp,
        };
        match terrain::spawn_adt(&mut assets, self.heightmap, adt_path) {
            Ok(result) => {
                self.commands.entity(camera).insert(result.camera);
                self.adt_manager.map_name = result.map_name;
                self.adt_manager.initial_tile = (result.tile_y, result.tile_x);
                self.adt_manager
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

    fn spawn_default_scene(&mut self) {
        let center = self.load_default_terrain();
        self.spawn_default_character_if_present();
        if let Some(pos) = center {
            set_player_position(self.commands, pos);
        }
    }

    fn load_default_terrain(&mut self) -> Option<Vec3> {
        let adt_path = paths::resolve_data_path("terrain/azeroth_32_48.adt");
        if !adt_path.exists() {
            return None;
        }
        let mut assets = terrain::AdtSpawnAssets {
            commands: self.commands,
            meshes: self.meshes,
            materials: self.materials,
            effect_materials: self.effect_materials,
            terrain_materials: self.terrain_mats,
            water_materials: self.water_mats,
            images: self.images,
            inverse_bp: self.inverse_bp,
        };
        match terrain::spawn_adt(&mut assets, self.heightmap, &adt_path) {
            Ok(result) => {
                self.adt_manager.map_name = result.map_name;
                self.adt_manager.initial_tile = (result.tile_y, result.tile_x);
                self.adt_manager
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

    fn spawn_default_character_if_present(&mut self) {
        let m2_path = paths::resolve_data_path("models/humanmale_hd.m2");
        if m2_path.exists() {
            let mut ctx = m2_scene::M2SceneSpawnContext {
                commands: self.commands,
                assets: crate::m2_spawn::SpawnAssets {
                    meshes: self.meshes,
                    materials: self.materials,
                    effect_materials: self.effect_materials,
                    skybox_materials: None,
                    images: self.images,
                    inverse_bindposes: self.inverse_bp,
                },
                creature_display_map: self.creature_display_map,
            };
            m2_scene::spawn_m2_model(&mut ctx, &m2_path);
        }
    }

    fn spawn_m2_scene(&mut self, m2_path: &Path) {
        let mut ctx = m2_scene::M2SceneSpawnContext {
            commands: self.commands,
            assets: crate::m2_spawn::SpawnAssets {
                meshes: self.meshes,
                materials: self.materials,
                effect_materials: self.effect_materials,
                skybox_materials: None,
                images: self.images,
                inverse_bindposes: self.inverse_bp,
            },
            creature_display_map: self.creature_display_map,
        };
        m2_scene::spawn_m2_model(&mut ctx, m2_path);
        ground::spawn_ground_clutter(
            self.commands,
            self.meshes,
            self.materials,
            self.effect_materials,
            self.images,
            self.inverse_bp,
            self.creature_display_map,
        );
    }
}

pub fn setup_explicit_asset_scene(mut params: SceneSetupSystemParams) {
    let asset_path = crate::parse_asset_path();
    if !should_load_explicit_scene_at_startup(params.server_addr.is_some(), asset_path.as_deref()) {
        return;
    }
    SceneSetupContext::from_system_params(&mut params).setup_world_scene(asset_path.as_deref());
}

pub fn setup_default_world_scene(mut params: SceneSetupSystemParams) {
    if params.server_addr.is_some() || crate::parse_asset_path().is_some() {
        return;
    }
    SceneSetupContext::from_system_params(&mut params).setup_world_scene(None);
}

pub fn set_player_position(commands: &mut Commands, pos: Vec3) {
    commands.queue(move |world: &mut World| {
        let mut q = world.query_filtered::<&mut Transform, With<Player>>();
        for mut xf in q.iter_mut(world) {
            xf.translation = pos;
        }
    });
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
