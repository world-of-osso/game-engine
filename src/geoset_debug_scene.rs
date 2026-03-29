use std::f32::consts::{FRAC_PI_8, PI};
use std::path::PathBuf;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::character_customization::{
    CharacterCustomizationSelection, CharacterRenderRequest,
};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::scene_setup::DEFAULT_M2;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

#[derive(Component)]
struct DebugCharacterScene;

#[derive(Component)]
struct DebugCharacterModelRoot;

#[derive(Component)]
struct DebugCharacterOrbit {
    yaw: f32,
    pitch: f32,
    focus: Vec3,
    distance: f32,
    base_pitch: f32,
}

#[derive(Resource, Clone)]
struct DebugCharacterConfig {
    race: u8,
    class: u8,
    sex: u8,
    appearance: CharacterAppearance,
    equipment_appearance: EquipmentAppearance,
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

pub struct DebugCharacterScenePlugin;

impl Plugin for DebugCharacterScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugCharacterConfig::from_env());
        app.add_systems(OnEnter(GameState::DebugCharacter), setup_scene);
        app.add_systems(Update, orbit_camera.run_if(in_state(GameState::DebugCharacter)));
        app.add_systems(OnExit(GameState::DebugCharacter), teardown_scene);
    }
}

impl DebugCharacterConfig {
    fn from_env() -> Self {
        Self {
            race: env_u8("DEBUG_CHARACTER_RACE", 1),
            class: env_u8("DEBUG_CHARACTER_CLASS", 1),
            sex: env_u8("DEBUG_CHARACTER_SEX", 0),
            appearance: CharacterAppearance {
                sex: env_u8("DEBUG_CHARACTER_SEX", 0),
                skin_color: env_u8("DEBUG_CHARACTER_SKIN_COLOR", 2),
                face: env_u8("DEBUG_CHARACTER_FACE", 3),
                hair_style: env_u8("DEBUG_CHARACTER_HAIR_STYLE", 4),
                hair_color: env_u8("DEBUG_CHARACTER_HAIR_COLOR", 5),
                facial_style: env_u8("DEBUG_CHARACTER_FACIAL_STYLE", 1),
            },
            equipment_appearance: EquipmentAppearance {
                entries: vec![equipped_entry(EquipmentVisualSlot::Head, 685129)],
            },
        }
    }
}

fn equipped_entry(slot: EquipmentVisualSlot, display_info_id: u32) -> EquippedAppearanceEntry {
    EquippedAppearanceEntry {
        slot,
        item_id: None,
        display_info_id: Some(display_info_id),
        inventory_type: 0,
        hidden: false,
    }
}

fn env_u8(name: &str, default: u8) -> u8 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(default)
}

fn spawn_camera(commands: &mut Commands) {
    let focus = Vec3::new(0.0, 1.0, 0.0);
    let eye = Vec3::new(0.0, 1.8, 6.0);
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = (offset.y / distance).asin();
    commands.spawn((
        Name::new("DebugCharacterCamera"),
        DebugCharacterScene,
        Camera3d::default(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        DebugCharacterOrbit {
            yaw: 0.0,
            pitch: 0.0,
            focus,
            distance,
            base_pitch,
        },
    ));
}

fn spawn_lighting(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.85),
        brightness: 80.0,
        ..default()
    });
    commands.spawn((
        Name::new("DebugCharacterLight"),
        DebugCharacterScene,
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
    let mut img = asset::blp::load_blp_gpu_image(&grass_path)
        .unwrap_or_else(|_| ground::generate_grass_texture());
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
        Name::new("DebugCharacterGround"),
        DebugCharacterScene,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
    ));
}

fn model_transform() -> Transform {
    Transform::from_xyz(0.0, 0.0, 0.0)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
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
fn spawn_debug_character_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    config: &DebugCharacterConfig,
) {
    let Some(model_path) = resolve_model_path(config.race, config.sex) else {
        return;
    };
    let Some(spawned) = m2_scene::spawn_animated_static_m2_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        &model_path,
        model_transform(),
        creature_display_map,
    ) else {
        return;
    };
    commands.entity(spawned.root).insert(DebugCharacterScene);
    commands
        .entity(spawned.model_root)
        .insert((
            DebugCharacterScene,
            DebugCharacterModelRoot,
            CharacterRenderRequest {
                selection: CharacterCustomizationSelection {
                    race: config.race,
                    class: config.class,
                    sex: config.sex,
                    appearance: config.appearance,
                },
                equipment_appearance: config.equipment_appearance.clone(),
            },
        ));
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
    config: Res<DebugCharacterConfig>,
) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_debug_character_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &config,
    );
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut DebugCharacterOrbit, &mut Transform)>,
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

fn teardown_scene(
    mut commands: Commands,
    query: Query<Entity, With<DebugCharacterScene>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
