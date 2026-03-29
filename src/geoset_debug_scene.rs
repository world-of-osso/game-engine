use std::f32::consts::{FRAC_PI_8, PI};
use std::path::PathBuf;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset;
use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::equipment_appearance::resolve_equipment_appearance;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn::{BatchTextureType, GeosetMesh};
use crate::scene_setup::DEFAULT_M2;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
use game_engine::outfit_data::OutfitData;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

#[derive(Component)]
struct GeosetDebugScene;

#[derive(Component)]
struct GeosetDebugModelRoot;

#[derive(Component)]
struct GeosetDebugOrbit {
    yaw: f32,
    pitch: f32,
    focus: Vec3,
    distance: f32,
    base_pitch: f32,
}

#[derive(Resource, Clone)]
struct GeosetDebugConfig {
    mesh_part_id: u16,
    race: u8,
    class: u8,
    sex: u8,
    appearance: CharacterAppearance,
    equipment_appearance: EquipmentAppearance,
}

#[derive(Resource, Default)]
struct GeosetDebugModel {
    root: Option<Entity>,
    applied: bool,
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

pub struct GeosetDebugScenePlugin;

impl Plugin for GeosetDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GeosetDebugConfig::from_env());
        app.init_resource::<GeosetDebugModel>();
        app.add_systems(OnEnter(GameState::GeosetDebug), setup_scene);
        app.add_systems(
            Update,
            (apply_debug_character_once, isolate_debug_geoset_mesh, orbit_camera)
                .run_if(in_state(GameState::GeosetDebug)),
        );
        app.add_systems(OnExit(GameState::GeosetDebug), teardown_scene);
    }
}

impl GeosetDebugConfig {
    fn from_env() -> Self {
        Self {
            mesh_part_id: env_u16("GEOSET_DEBUG_MESH_PART_ID", 5),
            race: env_u8("GEOSET_DEBUG_RACE", 1),
            class: env_u8("GEOSET_DEBUG_CLASS", 1),
            sex: env_u8("GEOSET_DEBUG_SEX", 0),
            appearance: CharacterAppearance {
                sex: env_u8("GEOSET_DEBUG_SEX", 0),
                skin_color: env_u8("GEOSET_DEBUG_SKIN_COLOR", 0),
                face: env_u8("GEOSET_DEBUG_FACE", 1),
                hair_style: env_u8("GEOSET_DEBUG_HAIR_STYLE", 1),
                hair_color: env_u8("GEOSET_DEBUG_HAIR_COLOR", 2),
                facial_style: env_u8("GEOSET_DEBUG_FACIAL_STYLE", 1),
            },
            equipment_appearance: EquipmentAppearance {
                entries: vec![
                    equipped_entry(EquipmentVisualSlot::Chest, 5729),
                    equipped_entry(EquipmentVisualSlot::Legs, 6050),
                    equipped_entry(EquipmentVisualSlot::Feet, 703),
                    equipped_entry(EquipmentVisualSlot::Hands, 155438),
                    equipped_entry(EquipmentVisualSlot::Head, 720086),
                ],
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

fn env_u16(name: &str, default: u16) -> u16 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
}

fn spawn_camera(commands: &mut Commands) {
    let focus = Vec3::new(0.0, 1.0, 0.0);
    let eye = Vec3::new(0.0, 1.8, 6.0);
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = (offset.y / distance).asin();
    commands.spawn((
        Name::new("GeosetDebugCamera"),
        GeosetDebugScene,
        Camera3d::default(),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        GeosetDebugOrbit {
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
        Name::new("GeosetDebugLight"),
        GeosetDebugScene,
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
        Name::new("GeosetDebugGround"),
        GeosetDebugScene,
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
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    config: Res<GeosetDebugConfig>,
    mut model: ResMut<GeosetDebugModel>,
) {
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    let Some(model_path) = resolve_model_path(config.race, config.sex) else {
        return;
    };
    let Some(root) = m2_scene::spawn_animated_static_m2(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        &model_path,
        model_transform(),
        &creature_display_map,
    ) else {
        return;
    };
    commands
        .entity(root)
        .insert((GeosetDebugScene, GeosetDebugModelRoot));
    model.root = Some(root);
    model.applied = false;
}

#[allow(clippy::too_many_arguments)]
fn apply_debug_character_once(
    config: Res<GeosetDebugConfig>,
    outfit_data: Res<OutfitData>,
    customization_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    mut model: ResMut<GeosetDebugModel>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) {
    if model.applied {
        return;
    }
    let Some(root) = model.root else {
        return;
    };
    let resolved_equipment = resolve_equipment_appearance(
        &config.equipment_appearance,
        &outfit_data,
        config.race,
        config.sex,
    );
    apply_character_customization(
        CharacterCustomizationSelection {
            race: config.race,
            class: config.class,
            sex: config.sex,
            appearance: config.appearance,
        },
        &customization_db,
        &char_tex,
        Some(&resolved_equipment),
        root,
        &mut images,
        &mut materials,
        &parent_query,
        &geoset_query,
        &mut visibility_query,
        &material_query,
    );
    model.applied = true;
}

fn isolate_debug_geoset_mesh(
    config: Res<GeosetDebugConfig>,
    model: Res<GeosetDebugModel>,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let Some(root) = model.root else {
        return;
    };
    for (entity, geoset_mesh, child_of) in &geoset_query {
        if child_of.parent() != root && !is_descendant_of(entity, root, &parent_query) {
            continue;
        }
        if let Ok(mut visibility) = visibility_query.get_mut(entity) {
            *visibility = if geoset_mesh.0 == config.mesh_part_id {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut GeosetDebugOrbit, &mut Transform)>,
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
    query: Query<Entity, With<GeosetDebugScene>>,
    mut model: ResMut<GeosetDebugModel>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    model.root = None;
    model.applied = false;
}

fn is_descendant_of(entity: Entity, root: Entity, parent_query: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    loop {
        let Ok(child_of) = parent_query.get(current) else {
            return false;
        };
        let parent = child_of.parent();
        if parent == root {
            return true;
        }
        current = parent;
    }
}
