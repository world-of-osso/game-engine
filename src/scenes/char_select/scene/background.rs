use std::path::PathBuf;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::scenes::char_select::scene_tree::{self as scene_tree, ActiveWarbandSceneId};
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};
use crate::skybox_m2_material::SkyboxM2Material;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

use super::{CharSelectScene, CharSelectSkybox};

pub(super) struct WarbandBackgroundSpawnContext<'a, 'w, 's> {
    pub(super) commands: &'a mut Commands<'w, 's>,
    pub(super) meshes: &'a mut Assets<Mesh>,
    pub(super) materials: &'a mut Assets<StandardMaterial>,
    pub(super) effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub(super) terrain_materials: &'a mut Assets<TerrainMaterial>,
    pub(super) water_materials: &'a mut Assets<WaterMaterial>,
    pub(super) images: &'a mut Assets<Image>,
    pub(super) inv_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
    pub(super) heightmap: &'a mut TerrainHeightmap,
}

pub(super) struct WarbandSkyboxSpawnContext<'a, 'w, 's> {
    pub(super) commands: &'a mut Commands<'w, 's>,
    pub(super) meshes: &'a mut Assets<Mesh>,
    pub(super) materials: &'a mut Assets<StandardMaterial>,
    pub(super) effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub(super) skybox_materials: &'a mut Assets<SkyboxM2Material>,
    pub(super) images: &'a mut Assets<Image>,
    pub(super) inv_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
    pub(super) creature_display_map: &'a crate::creature_display::CreatureDisplayMap,
}

fn spawn_tagged_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Entity {
    let grass_path = asset::asset_cache::texture(187126)
        .unwrap_or_else(|| PathBuf::from("data/textures/187126.blp"));
    let mut grass_image = asset::blp::load_blp_gpu_image(&grass_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        ground::generate_grass_texture()
    });
    grass_image.sampler =
        bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..bevy::image::ImageSamplerDescriptor::linear()
        });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(grass_image)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut mesh = Plane3d::default().mesh().size(30.0, 30.0).build();
    ground::scale_mesh_uvs(&mut mesh, 6.0);
    commands
        .spawn((
            Name::new("Ground"),
            CharSelectScene,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
        ))
        .id()
}

pub fn find_scene_entry<'a>(
    warband: &'a Option<Res<WarbandScenes>>,
    selected: &Option<Res<SelectedWarbandScene>>,
) -> Option<&'a crate::scenes::char_select::warband::WarbandSceneEntry> {
    warband
        .as_ref()
        .zip(selected.as_ref())
        .and_then(|(w, sel)| w.scenes.iter().find(|s| s.id == sel.scene_id))
}

pub fn spawn(
    ctx: &mut WarbandBackgroundSpawnContext<'_, '_, '_>,
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    focus: Option<Vec3>,
    active: &mut ActiveWarbandSceneId,
) -> game_engine::scene_tree::SceneNode {
    if let Some(s) = scene
        && let Some(result) = scene_tree::spawn_warband_terrain(
            ctx.commands,
            ctx.meshes,
            ctx.materials,
            ctx.effect_materials,
            ctx.terrain_materials,
            ctx.water_materials,
            ctx.images,
            ctx.inv_bp,
            ctx.heightmap,
            s,
            focus.unwrap_or_else(|| s.bevy_look_at()),
        )
    {
        active.0 = Some(s.id);
        let (ty, tx) = s.tile_coords();
        let wmos = result
            .wmo_entities
            .into_iter()
            .map(|(entity, model)| scene_tree::wmo_scene_node(entity, model))
            .collect();
        return scene_tree::background_scene_node(
            result.root_entity,
            &format!("terrain:{}_{ty}_{tx}", s.map_name()),
            result.doodad_count,
            wmos,
        );
    }
    let ground = spawn_tagged_ground(ctx.commands, ctx.meshes, ctx.materials, ctx.images);
    scene_tree::background_scene_node(ground, "ground", 0, vec![])
}

pub fn spawn_skybox(
    ctx: &mut WarbandSkyboxSpawnContext<'_, '_, '_>,
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    camera_translation: Vec3,
) -> Option<Entity> {
    let m2_path = crate::scenes::char_select::warband::ensure_warband_skybox(scene?)?;
    let spawned = m2_scene::spawn_animated_static_skybox_m2_parts(
        ctx.commands,
        ctx.meshes,
        ctx.materials,
        ctx.effect_materials,
        ctx.skybox_materials,
        ctx.images,
        ctx.inv_bp,
        &m2_path,
        Transform::from_translation(camera_translation),
        ctx.creature_display_map,
        None,
    )?;
    ctx.commands.entity(spawned.root).insert((
        CharSelectScene,
        CharSelectSkybox {
            path: m2_path.clone(),
        },
    ));
    ctx.commands
        .entity(spawned.model_root)
        .insert(CharSelectScene);
    Some(spawned.root)
}
