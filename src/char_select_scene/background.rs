use std::path::PathBuf;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::char_select_scene_tree::{self as scene_tree, ActiveWarbandSceneId};
use crate::ground;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::warband_scene::{SelectedWarbandScene, WarbandScenes};
use crate::water_material::WaterMaterial;

use super::CharSelectScene;

fn spawn_tagged_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Entity {
    let grass_path = asset::casc_resolver::ensure_texture(187126)
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
) -> Option<&'a crate::warband_scene::WarbandSceneEntry> {
    warband
        .as_ref()
        .zip(selected.as_ref())
        .and_then(|(w, sel)| w.scenes.iter().find(|s| s.id == sel.scene_id))
}

#[allow(clippy::too_many_arguments)]
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    active: &mut ActiveWarbandSceneId,
) -> game_engine::scene_tree::SceneNode {
    if let Some(s) = scene
        && let Some(result) = scene_tree::spawn_warband_terrain(
            commands,
            meshes,
            materials,
            terrain_materials,
            water_materials,
            images,
            inv_bp,
            heightmap,
            s,
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
    let ground = spawn_tagged_ground(commands, meshes, materials, images);
    scene_tree::background_scene_node(ground, "ground", 0, vec![])
}
