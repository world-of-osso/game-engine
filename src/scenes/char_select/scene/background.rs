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

const CAMPSITE_GROUND_PATCH_SIZE: f32 = 42.0;
const CAMPSITE_GROUND_PATCH_UV_SCALE: f32 = 9.0;
const CAMPSITE_GROUND_PATCH_Y_OFFSET: f32 = 0.03;

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

fn spawn_campsite_ground_patch(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    heightmap: &TerrainHeightmap,
    focus: Vec3,
) -> Option<Entity> {
    let terrain_y = heightmap.height_at(focus.x, focus.z)?;
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
    let mut mesh = Plane3d::default()
        .mesh()
        .size(CAMPSITE_GROUND_PATCH_SIZE, CAMPSITE_GROUND_PATCH_SIZE)
        .build();
    ground::scale_mesh_uvs(&mut mesh, CAMPSITE_GROUND_PATCH_UV_SCALE);
    Some(
        commands
            .spawn((
                Name::new("CampsiteGroundPatch"),
                CharSelectScene,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(
                    focus.x,
                    terrain_y + CAMPSITE_GROUND_PATCH_Y_OFFSET,
                    focus.z,
                )),
            ))
            .id(),
    )
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
    // Keep the selected warband scene marked active even when terrain falls back
    // to the placeholder ground. Otherwise the scene-switch system retries every
    // frame and stomps orbit camera drag back to the authored view.
    active.0 = scene.map(|scene| scene.id);
    if let Some(s) = scene
        && let Some(result) = scene_tree::spawn_warband_terrain(
            &mut scene_tree::WarbandTerrainSpawnContext {
                commands: ctx.commands,
                meshes: ctx.meshes,
                materials: ctx.materials,
                effect_materials: ctx.effect_materials,
                terrain_materials: ctx.terrain_materials,
                water_materials: ctx.water_materials,
                images: ctx.images,
                inv_bp: ctx.inv_bp,
                heightmap: ctx.heightmap,
            },
            s,
            focus.unwrap_or_else(|| s.bevy_look_at()),
        )
    {
        if let Some(focus) = focus {
            let _ = spawn_campsite_ground_patch(
                ctx.commands,
                ctx.meshes,
                ctx.materials,
                ctx.images,
                ctx.heightmap,
                focus,
            );
        }
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
    skybox_translation: Vec3,
) -> Option<Entity> {
    let m2_path = crate::scenes::char_select::warband::ensure_warband_skybox(scene?)?;
    let spawned = {
        let mut spawn_ctx = m2_scene::M2SceneSpawnContext {
            commands: ctx.commands,
            assets: crate::m2_spawn::SpawnAssets {
                meshes: ctx.meshes,
                materials: ctx.materials,
                effect_materials: ctx.effect_materials,
                skybox_materials: Some(ctx.skybox_materials),
                images: ctx.images,
                inverse_bindposes: ctx.inv_bp,
            },
            creature_display_map: ctx.creature_display_map,
        };
        m2_scene::spawn_animated_static_skybox_m2_parts(
            &mut spawn_ctx,
            &m2_path,
            Transform::from_translation(skybox_translation),
            None,
        )?
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;
    use game_engine::scene_tree::NodeProps;

    fn missing_scene() -> crate::scenes::char_select::warband::WarbandSceneEntry {
        crate::scenes::char_select::warband::WarbandSceneEntry {
            id: 77,
            name: "Missing".to_string(),
            description: "missing terrain fixture".to_string(),
            position: [0.0, 0.0, 0.0],
            look_at: [0.0, 1.0, 0.0],
            map_id: 999_999,
            fov: 45.0,
            texture_kit: 0,
        }
    }

    #[test]
    fn fallback_ground_marks_selected_scene_active() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<M2EffectMaterial>>();
        app.init_resource::<Assets<TerrainMaterial>>();
        app.init_resource::<Assets<WaterMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        app.init_resource::<TerrainHeightmap>();

        let scene = missing_scene();
        let expected_scene_id = scene.id;
        let (node, active) = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>,
                      mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
                      mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
                      mut water_materials: ResMut<Assets<WaterMaterial>>,
                      mut images: ResMut<Assets<Image>>,
                      mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
                      mut heightmap: ResMut<TerrainHeightmap>| {
                    let mut active = ActiveWarbandSceneId::default();
                    let node = spawn(
                        &mut WarbandBackgroundSpawnContext {
                            commands: &mut commands,
                            meshes: &mut meshes,
                            materials: &mut materials,
                            effect_materials: &mut effect_materials,
                            terrain_materials: &mut terrain_materials,
                            water_materials: &mut water_materials,
                            images: &mut images,
                            inv_bp: &mut inv_bp,
                            heightmap: &mut heightmap,
                        },
                        Some(&scene),
                        None,
                        &mut active,
                    );
                    (node, active)
                },
            )
            .expect("background spawn should run");

        assert_eq!(active.0, Some(expected_scene_id));
        match node.props {
            NodeProps::Background { model, .. } => assert_eq!(model, "ground"),
            props => panic!("expected fallback background node, got {props:?}"),
        }
    }

    #[test]
    fn terrain_background_spawns_campsite_ground_patch_at_focus() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<M2EffectMaterial>>();
        app.init_resource::<Assets<TerrainMaterial>>();
        app.init_resource::<Assets<WaterMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        app.init_resource::<TerrainHeightmap>();

        let warband = crate::scenes::char_select::warband::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .cloned()
            .expect("expected warband scene 1");
        let focus = warband
            .solo_character_placement(&scene)
            .expect("expected solo placement")
            .bevy_position();

        let node = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>,
                      mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
                      mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
                      mut water_materials: ResMut<Assets<WaterMaterial>>,
                      mut images: ResMut<Assets<Image>>,
                      mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
                      mut heightmap: ResMut<TerrainHeightmap>| {
                    let mut active = ActiveWarbandSceneId::default();
                    spawn(
                        &mut WarbandBackgroundSpawnContext {
                            commands: &mut commands,
                            meshes: &mut meshes,
                            materials: &mut materials,
                            effect_materials: &mut effect_materials,
                            terrain_materials: &mut terrain_materials,
                            water_materials: &mut water_materials,
                            images: &mut images,
                            inv_bp: &mut inv_bp,
                            heightmap: &mut heightmap,
                        },
                        Some(&scene),
                        Some(focus),
                        &mut active,
                    )
                },
            )
            .expect("background spawn should run");
        app.update();

        match node.props {
            NodeProps::Background { model, .. } => {
                assert!(
                    model.starts_with("terrain:"),
                    "expected terrain-backed background, got {model}"
                );
            }
            props => panic!("expected terrain background node, got {props:?}"),
        }

        let mut query = app
            .world_mut()
            .query::<(&Name, &Transform, &CharSelectScene)>();
        let Some((_, transform, _)) = query
            .iter(app.world())
            .find(|(name, _, _)| name.as_str() == "CampsiteGroundPatch")
        else {
            panic!("expected campsite ground patch to be spawned");
        };
        assert!(
            (transform.translation.x - focus.x).abs() < 0.01
                && (transform.translation.z - focus.z).abs() < 0.01,
            "ground patch should be centered on the selected character focus"
        );
    }
}
