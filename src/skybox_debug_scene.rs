use std::f32::consts::PI;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;
use crate::skybox_m2_material::SkyboxM2Material;
use crate::warband_scene::{SelectedWarbandScene, WarbandScenes};

#[derive(Component)]
struct SkyboxDebugScene;

#[derive(Component)]
struct SkyboxDebugSkybox;

pub struct SkyboxDebugScenePlugin;

impl Plugin for SkyboxDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::SkyboxDebug), setup_scene);
        app.add_systems(
            Update,
            sync_skybox_to_camera.run_if(in_state(GameState::SkyboxDebug)),
        );
        app.add_systems(OnExit(GameState::SkyboxDebug), teardown_scene);
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut skybox_materials: ResMut<Assets<SkyboxM2Material>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    warband: Res<WarbandScenes>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
) {
    let scene = selected_scene
        .as_ref()
        .and_then(|selected| {
            warband
                .scenes
                .iter()
                .find(|scene| scene.id == selected.scene_id)
        })
        .or_else(|| warband.scenes.first());

    let focus = Vec3::new(0.0, 1.0, 0.0);
    let orbit = OrbitCamera::new(focus, 7.5);
    let eye = orbit.eye_position();

    commands.insert_resource(ClearColor(Color::BLACK));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 60.0,
        ..default()
    });

    commands.spawn((
        Name::new("SkyboxDebugCamera"),
        SkyboxDebugScene,
        Camera3d::default(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        orbit,
    ));

    commands.spawn((
        Name::new("SkyboxDebugLight"),
        SkyboxDebugScene,
        DirectionalLight {
            illuminance: 2500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 5.0, PI / 6.0, 0.0)),
    ));

    commands.spawn((
        Name::new("SkyboxDebugReferencePlane"),
        SkyboxDebugScene,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1.8, 1.8).build())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.72, 0.42, 0.18),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.05, 0.0),
    ));

    let Some(scene) = scene else {
        warn!("skybox_debug_scene: no warband scene available for skybox selection");
        return;
    };
    let Some(path) = crate::warband_scene::ensure_warband_skybox(scene) else {
        warn!(
            "skybox_debug_scene: failed to resolve skybox model for scene {} ({})",
            scene.id, scene.name
        );
        return;
    };

    let Some(spawned) = m2_scene::spawn_animated_static_skybox_m2_parts(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut skybox_materials,
        &mut images,
        &mut inv_bp,
        &path,
        Transform::from_translation(eye),
        &creature_display_map,
    ) else {
        warn!(
            "skybox_debug_scene: failed to spawn skybox model at {}",
            path.display()
        );
        return;
    };
    let authored_light_params = scene.authored_light_params_id();
    info!(
        "skybox_debug_scene: resolved scene {} ({}) to skybox {} (LightParamsID={:?})",
        scene.id,
        scene.name,
        path.display(),
        authored_light_params
    );
    commands.entity(spawned.root).insert((
        SkyboxDebugScene,
        SkyboxDebugSkybox,
        Name::new(format!("SkyboxDebug:{}", path.display())),
    ));
    commands.entity(spawned.model_root).insert(SkyboxDebugScene);
    commands.insert_resource(SceneTree {
        root: SceneNode {
            label: "SkyboxDebugScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children: vec![
                SceneNode {
                    label: "Camera".into(),
                    entity: None,
                    props: NodeProps::Camera { fov: 60.0 },
                    children: vec![],
                },
                SceneNode {
                    label: "Skybox".into(),
                    entity: Some(spawned.root),
                    props: NodeProps::Object {
                        kind: "Skybox".into(),
                        model: path.display().to_string(),
                    },
                    children: vec![],
                },
            ],
        },
    });
}

fn sync_skybox_to_camera(
    camera_query: Query<&Transform, (With<OrbitCamera>, With<SkyboxDebugScene>)>,
    mut skybox_query: Query<&mut Transform, (With<SkyboxDebugSkybox>, Without<OrbitCamera>)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    for mut transform in &mut skybox_query {
        transform.translation = camera_transform.translation;
    }
}

fn teardown_scene(mut commands: Commands, query: Query<Entity, With<SkyboxDebugScene>>) {
    commands.remove_resource::<SceneTree>();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
