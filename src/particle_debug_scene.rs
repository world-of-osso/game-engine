use std::f32::consts::PI;
use std::path::PathBuf;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::outfit_data::OutfitData;

use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;

const TORCH_M2: &str = "data/models/club_1h_torch_a_01.m2";

#[derive(Component)]
struct ParticleDebugScene;

pub struct ParticleDebugScenePlugin;

impl Plugin for ParticleDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::ParticleDebug), setup_scene);
        app.add_systems(OnExit(GameState::ParticleDebug), teardown_scene);
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    outfit_data: Res<OutfitData>,
) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_torch(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &outfit_data,
    );
}

fn spawn_camera(commands: &mut Commands) {
    let focus = Vec3::Y * 0.5;
    let orbit = OrbitCamera::new(focus, 3.0);
    let eye = orbit.eye_position();
    commands.spawn((
        Name::new("ParticleDebugCamera"),
        ParticleDebugScene,
        Camera3d::default(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        orbit,
    ));
}

fn spawn_lighting(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 20.0,
        ..default()
    });
    commands.spawn((
        Name::new("ParticleDebugLight"),
        ParticleDebugScene,
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_torch(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    outfit_data: &OutfitData,
) {
    let path = PathBuf::from(TORCH_M2);
    if !path.exists() {
        eprintln!("particle_debug_scene: torch model not found at {TORCH_M2}");
        return;
    }
    let skin_fdids = outfit_data
        .resolve_item_model_skin_fdids_for_model_path(&path)
        .or_else(|| creature_display_map.resolve_skin_fdids_for_model_path(&path))
        .unwrap_or([0, 0, 0]);
    m2_scene::spawn_animated_static_m2_parts_with_skin_fdids(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        &path,
        Transform::IDENTITY,
        &skin_fdids,
        1.0,
    );
}

fn teardown_scene(mut commands: Commands, query: Query<Entity, With<ParticleDebugScene>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
