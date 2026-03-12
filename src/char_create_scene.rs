//! 3D scene behind the character creation screen.
//!
//! Reuses the same orbit camera, lighting, and ground as char select.

use std::f32::consts::{FRAC_PI_8, PI};
use std::path::PathBuf;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::char_create::CharCreateState;
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_scene;
use crate::scene_setup::DEFAULT_M2;

#[derive(Component)]
struct CharCreateScene;

#[derive(Component)]
struct CharCreateModelRoot;

#[derive(Resource, Default, PartialEq, Eq)]
struct DisplayedRaceSex(Option<(u8, u8)>);

#[derive(Component)]
struct CharCreateOrbit {
    yaw: f32,
    pitch: f32,
    focus: Vec3,
    distance: f32,
    base_pitch: f32,
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

pub struct CharCreateScenePlugin;

impl Plugin for CharCreateScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedRaceSex>();
        app.add_systems(OnEnter(GameState::CharCreate), setup_scene);
        app.add_systems(
            Update,
            (sync_model, orbit_camera).run_if(in_state(GameState::CharCreate)),
        );
        app.add_systems(OnExit(GameState::CharCreate), teardown_scene);
    }
}

fn spawn_camera(commands: &mut Commands) -> Entity {
    let focus = Vec3::new(0.0, 1.0, 0.0);
    let eye = Vec3::new(0.0, 1.8, 6.0);
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = (offset.y / distance).asin();
    commands
        .spawn((
            Name::new("CharCreateCamera"),
            CharCreateScene,
            Camera3d::default(),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            CharCreateOrbit { yaw: 0.0, pitch: 0.0, focus, distance, base_pitch },
        ))
        .id()
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut CharCreateOrbit, &mut Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) || motion.delta == Vec2::ZERO {
        return;
    }
    for (mut orbit, mut transform) in &mut query {
        orbit.yaw = (orbit.yaw - motion.delta.x * ORBIT_SENSITIVITY)
            .clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + motion.delta.y * ORBIT_SENSITIVITY)
            .clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        let pitch = orbit.base_pitch + orbit.pitch;
        let eye = orbit.focus
            + Vec3::new(
                orbit.yaw.sin() * pitch.cos(),
                pitch.sin(),
                orbit.yaw.cos() * pitch.cos(),
            ) * orbit.distance;
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
    }
}

fn spawn_lighting(commands: &mut Commands) {
    commands.spawn((
        Name::new("AmbientLight"),
        CharCreateScene,
        AmbientLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            brightness: 80.0,
            ..default()
        },
    ));
    commands.spawn((
        Name::new("DirectionalLight"),
        CharCreateScene,
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.92, 0.8),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
}

fn spawn_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let grass_path = asset::casc_resolver::ensure_texture(187126)
        .unwrap_or_else(|| PathBuf::from("data/textures/187126.blp"));
    let mut img = asset::blp::load_blp_gpu_image(&grass_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        ground::generate_grass_texture()
    });
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(img)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut mesh = Plane3d::default().mesh().size(30.0, 30.0).build();
    ground::scale_mesh_uvs(&mut mesh, 6.0);
    commands.spawn((
        Name::new("Ground"),
        CharCreateScene,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
    ));
}

#[allow(clippy::too_many_arguments)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    mut displayed: ResMut<DisplayedRaceSex>,
) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_race_model(
        &mut commands, &mut meshes, &mut materials, &mut images,
        &mut inv_bp, &creature_display_map, 1, 0,
    );
    displayed.0 = Some((1, 0));
}

#[allow(clippy::too_many_arguments)]
fn spawn_race_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    race: u8,
    sex: u8,
) -> Option<Entity> {
    let model_path = resolve_model_path(race, sex)?;
    let entity = m2_scene::spawn_static_m2(
        commands, meshes, materials, images, inv_bp, &model_path,
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        creature_display_map,
    )?;
    commands.entity(entity).insert((CharCreateScene, CharCreateModelRoot));
    Some(entity)
}

fn resolve_model_path(race: u8, sex: u8) -> Option<PathBuf> {
    race_model_wow_path(race, sex)
        .and_then(ensure_named_model_bundle)
        .or_else(|| {
            let p = PathBuf::from(DEFAULT_M2);
            p.exists().then_some(p)
        })
}

#[allow(clippy::too_many_arguments)]
fn sync_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    state: Option<Res<CharCreateState>>,
    current_model: Query<Entity, With<CharCreateModelRoot>>,
    mut displayed: ResMut<DisplayedRaceSex>,
) {
    let Some(state) = state else { return };
    let desired = Some((state.selected_race, state.selected_sex));
    if displayed.0 == desired {
        return;
    }
    for entity in current_model.iter() {
        commands.entity(entity).despawn();
    }
    spawn_race_model(
        &mut commands, &mut meshes, &mut materials, &mut images,
        &mut inv_bp, &creature_display_map, state.selected_race, state.selected_sex,
    );
    displayed.0 = desired;
}

fn teardown_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharCreateScene>>,
    mut displayed: ResMut<DisplayedRaceSex>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = None;
}
