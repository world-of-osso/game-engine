use super::*;
use crate::asset::adt;
use crate::terrain_material::TerrainMaterial;

const ROOT_PATH: &str = "data/terrain/2703_31_36.adt";
const TEXTURE_PATH: &str = "data/terrain/2703_31_36_tex0.adt";

#[test]
fn supplemental_waterfall_spawns_placements_and_shadowed_terrain() {
    let root_data = std::fs::read(ROOT_PATH).unwrap();
    let texture_data = std::fs::read(TEXTURE_PATH).unwrap();
    let expected = adt::load_adt_for_tile_with_tex0(&root_data, &texture_data, 31, 36).unwrap();
    assert_eq!(expected.chunks.len(), 256);
    assert_eq!(
        expected
            .chunks
            .iter()
            .filter(|chunk| chunk.shadow_map.is_some())
            .count(),
        227
    );

    let mut app = render_path_test_app();
    let scene = app
        .world()
        .resource::<WarbandScenes>()
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .unwrap()
        .clone();
    let root = app
        .world_mut()
        .spawn((Transform::default(), Visibility::default()))
        .id();
    let spawned = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut assets: scene_types::CharSelectRenderAssets,
                  mut heightmap: ResMut<TerrainHeightmap>| {
                scene_tree::spawn_warband_supplemental_terrain(
                    &mut scene_tree::WarbandTerrainSpawnContext {
                        commands: &mut commands,
                        meshes: &mut assets.meshes,
                        materials: &mut assets.materials,
                        effect_materials: &mut assets.effect_materials,
                        terrain_materials: &mut assets.terrain_materials,
                        water_materials: &mut assets.water_materials,
                        images: &mut assets.images,
                        inv_bp: &mut assets.inv_bp,
                        heightmap: &mut heightmap,
                    },
                    &scene,
                    root,
                )
            },
        )
        .unwrap();
    app.update();
    assert_eq!(
        spawned, 42,
        "all authored waterfall/ripple placements must spawn"
    );

    let handles: Vec<_> = app
        .world_mut()
        .query::<(&Mesh3d, &MeshMaterial3d<TerrainMaterial>)>()
        .iter(app.world())
        .map(|(_, material)| material.0.clone())
        .collect();
    assert_eq!(
        handles.len(),
        256,
        "supplemental ADT geometry must remain present"
    );
    let materials = app.world().resource::<Assets<TerrainMaterial>>();
    let images = app.world().resource::<Assets<Image>>();
    let mut actual_shadows: Vec<_> = handles
        .iter()
        .map(|handle| {
            let material = materials.get(handle).unwrap();
            images
                .get(&material.shadow_map)
                .unwrap()
                .data
                .clone()
                .unwrap()
        })
        .collect();
    let mut expected_shadows: Vec<_> = expected
        .chunks
        .iter()
        .map(|chunk| expected_shadow_pixels(chunk.shadow_map.as_ref()))
        .collect();
    actual_shadows.sort();
    expected_shadows.sort();
    assert_eq!(
        actual_shadows, expected_shadows,
        "spawned materials must retain companion shadow pixels"
    );
}

#[test]
fn primary_waterfall_backdrop_is_not_limited_to_nearby_props() {
    let mut app = render_path_test_app();
    let warband = app.world().resource::<WarbandScenes>();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .unwrap()
        .clone();
    let focus = warband
        .solo_character_placement(&scene)
        .unwrap()
        .bevy_position();
    let spawned = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut assets: scene_types::CharSelectRenderAssets,
                  mut heightmap: ResMut<TerrainHeightmap>| {
                scene_tree::spawn_warband_terrain(
                    &mut scene_tree::WarbandTerrainSpawnContext {
                        commands: &mut commands,
                        meshes: &mut assets.meshes,
                        materials: &mut assets.materials,
                        effect_materials: &mut assets.effect_materials,
                        terrain_materials: &mut assets.terrain_materials,
                        water_materials: &mut assets.water_materials,
                        images: &mut assets.images,
                        inv_bp: &mut assets.inv_bp,
                        heightmap: &mut heightmap,
                    },
                    &scene,
                    focus,
                )
            },
        )
        .unwrap()
        .expect("primary campsite loads");
    app.update();
    let waterfall_present = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .any(|name| name.as_str() == "4661358");
    assert!(
        waterfall_present,
        "the authored waterfall04 backdrop must be present beyond the prop radius"
    );
    assert_eq!(
        spawned.doodad_count, 76,
        "62 nearby props plus 14 authored waterfall/ripple placements"
    );
}

#[test]
fn waterfall_mist_attachment_spawns_authored_particle_emitter() {
    let (app, root, authored) = spawn_waterfall_mist_attachment(true, true);
    let mut app = app;
    let emitters: Vec<_> = app
        .world_mut()
        .query::<(Entity, &crate::particle::ParticleEmitterComp, &ChildOf)>()
        .iter(app.world())
        .collect();
    assert_eq!(
        emitters.len(),
        1,
        "placed waterfall mist must retain its emitter"
    );
    let (_, emitter, parent) = emitters[0];
    assert_eq!(emitter.emitter.texture_fdid, authored.texture_fdid);
    assert_eq!(emitter.emitter.position, authored.position);
    assert_eq!(emitter.emitter.bone_index, authored.bone_index);
    assert_eq!(emitter.emitter.emission_rate, authored.emission_rate);
    let bone = emitter
        .bone_entity
        .expect("authored mist bone must be bound");
    assert!(app.world().get::<Transform>(bone).is_some());
    const PARTICLE_FLAG_WORLD_SPACE: u32 = 0x200;
    if authored.flags & PARTICLE_FLAG_WORLD_SPACE == 0 {
        assert_eq!(parent.parent(), bone, "bone-local mist follows its bone");
    }
    let mut ancestor = parent.parent();
    while ancestor != root {
        ancestor = app
            .world()
            .get::<ChildOf>(ancestor)
            .expect("mist emitter must remain attached to its placed model")
            .parent();
    }
}

#[test]
fn waterfall_mist_attachment_respects_disabled_particle_effects() {
    let (mut app, _, _) = spawn_waterfall_mist_attachment(false, true);
    let count = app
        .world_mut()
        .query::<&crate::particle::ParticleEmitterComp>()
        .iter(app.world())
        .count();
    assert_eq!(count, 0, "disabled particle effects must not spawn mist");
}

#[test]
fn waterfall_mist_attachment_leaves_unselected_emitters_disabled() {
    let (mut app, _, _) = spawn_waterfall_mist_attachment(true, false);
    assert_eq!(
        app.world_mut()
            .query::<&crate::particle::ParticleEmitterComp>()
            .iter(app.world())
            .count(),
        0,
    );
}

fn spawn_waterfall_mist_attachment(
    particle_effects_enabled: bool,
    spawn_particles: bool,
) -> (App, Entity, crate::asset::m2_particle::M2ParticleEmitter) {
    let model =
        crate::asset::m2::load_m2_uncached(std::path::Path::new("data/models/1028937.m2"), &[0; 3])
            .expect("authored waterfall mist model must load");
    assert_eq!(model.particle_emitters.len(), 1);
    let authored = model.particle_emitters[0].clone();
    assert!(authored.texture_fdid.is_some());
    assert!((authored.bone_index as usize) < model.bones.len());
    let mut app = render_path_test_app();
    app.insert_resource(crate::client_options::GraphicsOptions {
        particle_effects_enabled,
        ..default()
    });
    let root = app.world_mut().spawn(Transform::IDENTITY).id();
    let mut model = Some(model);
    let attached = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut assets: scene_types::CharSelectRenderAssets| {
                crate::m2_spawn::spawn_m2_model_on_entity(
                    &mut commands,
                    &mut crate::m2_spawn::SpawnAssets {
                        meshes: &mut assets.meshes,
                        materials: &mut assets.materials,
                        effect_materials: &mut assets.effect_materials,
                        skybox_materials: Some(&mut assets.skybox_materials),
                        images: &mut assets.images,
                        inverse_bindposes: &mut assets.inv_bp,
                    },
                    model.take().expect("attachment runs once"),
                    root,
                    spawn_particles,
                )
            },
        )
        .expect("mist attachment system must run");
    assert!(attached);
    app.update();
    (app, root, authored)
}

fn expected_shadow_pixels(shadow: Option<&[u8; 512]>) -> Vec<u8> {
    (0..4096)
        .flat_map(|pixel| {
            let shadowed = shadow.is_some_and(|bits| bits[pixel / 8] & (1 << (pixel % 8)) != 0);
            let value = if shadowed { 0 } else { 255 };
            [value, value, value, 255]
        })
        .collect()
}

#[test]
fn supplemental_waterfall_rejects_empty_texture_companion() {
    let root = std::fs::read(ROOT_PATH).unwrap();
    let error = adt::load_adt_for_tile_with_tex0(&root, &[], 31, 36)
        .err()
        .expect("empty companion must fail without panicking");
    assert!(error.contains("chunk") && error.contains("256"), "{error}");
}

#[test]
fn supplemental_waterfall_rejects_companion_without_required_shadows() {
    let root = std::fs::read(ROOT_PATH).unwrap();
    let mut texture = Vec::new();
    for _ in 0..256 {
        texture.extend_from_slice(b"KNCM");
        texture.extend_from_slice(&0_u32.to_le_bytes());
    }
    let error = adt::load_adt_for_tile_with_tex0(&root, &texture, 31, 36)
        .err()
        .expect("shadow-flagged chunks must retain validation");
    assert!(error.contains("MCSH") && error.contains("HSCM"), "{error}");
}
