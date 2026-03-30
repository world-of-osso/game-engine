use std::f32::consts::PI;
use std::path::PathBuf;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::asset;
use crate::character_customization::{CharacterCustomizationSelection, CharacterRenderRequest};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::equipment::{Equipment, EquipmentItem, EquipmentSlot};
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
    target_distance: f32,
    min_distance: f32,
    max_distance: f32,
    base_pitch: f32,
}

#[derive(Resource, Clone)]
struct DebugCharacterConfig {
    race: u8,
    class: u8,
    sex: u8,
    appearance: CharacterAppearance,
    left_head_display: u32,
    right_head_display: u32,
    shoulder_display: u32,
    back_display: u32,
    chest_display: u32,
    left_hands_display: u32,
    right_hands_display: u32,
    left_waist_display: u32,
    left_legs_display: u32,
    left_feet_display: u32,
    right_waist_display: u32,
    right_legs_display: u32,
    right_feet_display: u32,
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_ZOOM_STEP: f32 = 0.4;
const ORBIT_ZOOM_LERP: f32 = 0.25;

pub struct DebugCharacterScenePlugin;

impl Plugin for DebugCharacterScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugCharacterConfig::from_env());
        app.add_systems(OnEnter(GameState::DebugCharacter), setup_scene);
        app.add_systems(
            Update,
            (orbit_camera, build_debug_scene_tree.after(orbit_camera))
                .run_if(in_state(GameState::DebugCharacter)),
        );
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
                eye_color: env_u8("DEBUG_CHARACTER_EYE_COLOR", 0),
                hair_style: env_u8("DEBUG_CHARACTER_HAIR_STYLE", 4),
                hair_color: env_u8("DEBUG_CHARACTER_HAIR_COLOR", 5),
                facial_style: env_u8("DEBUG_CHARACTER_FACIAL_STYLE", 1),
            },
            // Display 178116: geoset-only helmet (item 158364)
            left_head_display: env_u32("DEBUG_CHARACTER_LEFT_HEAD_DISPLAY", 178116),
            // Display 178254: cloth helm with runtime M2 model (item 1280)
            right_head_display: env_u32("DEBUG_CHARACTER_RIGHT_HEAD_DISPLAY", 178254),
            // Display 148865: shoulder with runtime models
            shoulder_display: env_u32("DEBUG_CHARACTER_SHOULDER_DISPLAY", 148865),
            // Display 181925: cloak
            back_display: env_u32("DEBUG_CHARACTER_BACK_DISPLAY", 181925),
            // Display 175942: chest
            chest_display: env_u32("DEBUG_CHARACTER_CHEST_DISPLAY", 175942),
            // Display 510: texture-only cloth glove (geoset group 4)
            left_hands_display: env_u32("DEBUG_CHARACTER_LEFT_HANDS_DISPLAY", 510),
            // Display 154616: leather glove with runtime M2 model + textures
            right_hands_display: env_u32("DEBUG_CHARACTER_RIGHT_HANDS_DISPLAY", 154616),
            // https://www.wowhead.com/item=49806/flayers-black-belt
            // Display 109162: belt geoset + TorsoLower/LegUpper textures + runtime buckle
            left_waist_display: env_u32("DEBUG_CHARACTER_LEFT_WAIST_DISPLAY", 109162),
            // Display 159629: hybrid legs (geoset + runtime model)
            left_legs_display: env_u32("DEBUG_CHARACTER_LEFT_LEGS_DISPLAY", 159629),
            left_feet_display: env_u32("DEBUG_CHARACTER_LEFT_FEET_DISPLAY", 154620),
            right_waist_display: env_u32("DEBUG_CHARACTER_RIGHT_WAIST_DISPLAY", 160997),
            // Display 73783: geoset-only legs
            right_legs_display: env_u32("DEBUG_CHARACTER_RIGHT_LEGS_DISPLAY", 73783),
            right_feet_display: env_u32("DEBUG_CHARACTER_RIGHT_FEET_DISPLAY", 154620),
        }
    }
}

fn debug_equipment_appearance(config: &DebugCharacterConfig, head: u32, hands: u32, waist: u32, legs: u32, feet: u32) -> EquipmentAppearance {
    let mut entries = Vec::new();
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Head, head);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Shoulder, config.shoulder_display);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Back, config.back_display);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Chest, config.chest_display);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Hands, hands);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Waist, waist);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Legs, legs);
    push_equipped_entry(&mut entries, EquipmentVisualSlot::Feet, feet);
    EquipmentAppearance { entries }
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

fn push_equipped_entry(
    entries: &mut Vec<EquippedAppearanceEntry>,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
) {
    if display_info_id != 0 {
        entries.push(equipped_entry(slot, display_info_id));
    }
}

fn env_u8(name: &str, default: u8) -> u8 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(default)
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
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
            target_distance: distance,
            min_distance: 1.5,
            max_distance: 12.0,
            base_pitch,
        },
    ));
}

fn spawn_lighting(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.85),
        brightness: 35.0,
        ..default()
    });
    commands.spawn((
        Name::new("DebugCharacterLight"),
        DebugCharacterScene,
        DirectionalLight {
            illuminance: 4200.0,
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
    let grass_path = PathBuf::from("data/textures/187126.blp");
    let mut img = if grass_path.exists() {
        asset::blp::load_blp_gpu_image(&grass_path)
            .unwrap_or_else(|_| ground::generate_grass_texture())
    } else {
        ground::generate_grass_texture()
    };
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

fn model_transform(x: f32) -> Transform {
    Transform::from_xyz(x, 0.0, 0.0)
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
    x: f32,
    head_display: u32,
    hands_display: u32,
    waist_display: u32,
    legs_display: u32,
    feet_display: u32,
    name: &str,
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
        model_transform(x),
        creature_display_map,
    ) else {
        return;
    };
    commands
        .entity(spawned.root)
        .insert((DebugCharacterScene, Name::new(name.to_string())));
    commands.entity(spawned.model_root).insert((
        DebugCharacterScene,
        DebugCharacterModelRoot,
        CharacterRenderRequest {
            selection: CharacterCustomizationSelection {
                race: config.race,
                class: config.class,
                sex: config.sex,
                appearance: config.appearance,
            },
            equipment_appearance: debug_equipment_appearance(config, head_display, hands_display, waist_display, legs_display, feet_display),
        },
    ));
}

struct DebugCharacterSide {
    x: f32,
    head: u32,
    hands: u32,
    waist: u32,
    legs: u32,
    feet: u32,
    name: &'static str,
}

#[allow(clippy::too_many_arguments)]
fn spawn_debug_pair(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    config: &DebugCharacterConfig,
) {
    let sides = [
        DebugCharacterSide { x: -1.7, head: config.left_head_display, hands: config.left_hands_display, waist: config.left_waist_display, legs: config.left_legs_display, feet: config.left_feet_display, name: "DebugCharacterGeoset" },
        DebugCharacterSide { x: 1.7, head: config.right_head_display, hands: config.right_hands_display, waist: config.right_waist_display, legs: config.right_legs_display, feet: config.right_feet_display, name: "DebugCharacterM2" },
    ];
    for side in &sides {
        spawn_debug_character_model(commands, meshes, materials, effect_materials, images, inv_bp, creature_display_map, config, side.x, side.head, side.hands, side.waist, side.legs, side.feet, side.name);
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
    config: Res<DebugCharacterConfig>,
) {
    eprintln!(
        "debugcharacter displays: left_head={} right_head={} shoulder={} back={} chest={} left_hands={} right_hands={} left_waist={} left_legs={} left_feet={} right_waist={} right_legs={} right_feet={}",
        config.left_head_display,
        config.right_head_display,
        config.shoulder_display,
        config.back_display,
        config.chest_display,
        config.left_hands_display,
        config.right_hands_display,
        config.left_waist_display,
        config.left_legs_display,
        config.left_feet_display,
        config.right_waist_display,
        config.right_legs_display,
        config.right_feet_display
    );
    spawn_camera(&mut commands);
    spawn_lighting(&mut commands);
    spawn_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_debug_pair(
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
    scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut DebugCharacterOrbit, &mut Transform)>,
) {
    for (mut orbit, mut transform) in &mut query {
        if scroll.delta.y != 0.0 {
            orbit.target_distance = (orbit.target_distance - scroll.delta.y * ORBIT_ZOOM_STEP)
                .clamp(orbit.min_distance, orbit.max_distance);
        }
        orbit.distance = orbit.distance.lerp(orbit.target_distance, ORBIT_ZOOM_LERP);
        if mouse_buttons.pressed(MouseButton::Left) && motion.delta != Vec2::ZERO {
            orbit.yaw -= motion.delta.x * ORBIT_SENSITIVITY;
            orbit.pitch += motion.delta.y * ORBIT_SENSITIVITY;
        }
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

fn teardown_scene(mut commands: Commands, query: Query<Entity, With<DebugCharacterScene>>) {
    commands.remove_resource::<SceneTree>();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn build_debug_scene_tree(
    mut commands: Commands,
    model_roots: Query<
        (Entity, &CharacterRenderRequest, &ChildOf, Option<&Equipment>),
        With<DebugCharacterModelRoot>,
    >,
    equipment_items: Query<(Entity, &EquipmentItem, &ChildOf, Option<&Name>)>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
) {
    // Wait until equipment items have been spawned for all expected runtime slots.
    for (model_root, _request, _root_parent, equipment) in &model_roots {
        let Some(equipment) = equipment else { continue };
        for &slot in equipment.slots.keys() {
            if find_equipment_item_for_slot(model_root, slot, &equipment_items, &parents).is_none()
            {
                return;
            }
        }
    }

    let mut children = Vec::new();
    for (model_root, request, root_parent, _) in &model_roots {
        let label = names
            .get(root_parent.parent())
            .map(|name| name.as_str().to_string())
            .unwrap_or_else(|_| format!("Character:{model_root:?}"));
        children.push(debug_character_scene_node(
            model_root,
            label,
            request,
            &equipment_items,
            &parents,
            &names,
        ));
    }
    children.sort_by(|a, b| a.label.cmp(&b.label));
    commands.insert_resource(SceneTree {
        root: SceneNode {
            label: "DebugCharacterScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children,
        },
    });
}

fn debug_character_scene_node(
    model_root: Entity,
    label: String,
    request: &CharacterRenderRequest,
    equipment_items: &Query<(Entity, &EquipmentItem, &ChildOf, Option<&Name>)>,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
) -> SceneNode {
    let slot_defs: &[(Option<EquipmentSlot>, EquipmentVisualSlot, &str)] = &[
        (Some(EquipmentSlot::Head), EquipmentVisualSlot::Head, "Head"),
        (Some(EquipmentSlot::ShoulderLeft), EquipmentVisualSlot::Shoulder, "ShoulderLeft"),
        (Some(EquipmentSlot::ShoulderRight), EquipmentVisualSlot::Shoulder, "ShoulderRight"),
        (Some(EquipmentSlot::Back), EquipmentVisualSlot::Back, "Back"),
        (Some(EquipmentSlot::Chest), EquipmentVisualSlot::Chest, "Chest"),
        (Some(EquipmentSlot::Hands), EquipmentVisualSlot::Hands, "Hands"),
        (Some(EquipmentSlot::Waist), EquipmentVisualSlot::Waist, "Waist"),
        (Some(EquipmentSlot::Legs), EquipmentVisualSlot::Legs, "Legs"),
        (Some(EquipmentSlot::Feet), EquipmentVisualSlot::Feet, "Feet"),
    ];
    let children = slot_defs
        .iter()
        .map(|(eq_slot, vis_slot, name)| {
            equipment_slot_scene_node(
                model_root, *eq_slot, *vis_slot, name, request, equipment_items, parents, names,
            )
        })
        .collect();
    SceneNode {
        label,
        entity: Some(model_root),
        props: NodeProps::Character {
            model: "humanmale_hd".into(),
            race: "Human".into(),
            gender: "Male".into(),
        },
        children,
    }
}

fn equipment_slot_scene_node(
    model_root: Entity,
    slot: Option<EquipmentSlot>,
    visual_slot: EquipmentVisualSlot,
    slot_name: &str,
    request: &CharacterRenderRequest,
    equipment_items: &Query<(Entity, &EquipmentItem, &ChildOf, Option<&Name>)>,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
) -> SceneNode {
    let item_entity =
        slot.and_then(|s| find_equipment_item_for_slot(model_root, s, equipment_items, parents));
    let (anchor, attachment, attachment_anchor) =
        equipment_item_details(item_entity, equipment_items, names);
    let model = request
        .equipment_appearance
        .entries
        .iter()
        .find(|entry| entry.slot == visual_slot)
        .and_then(|entry| entry.display_info_id)
        .map(|display| format!("display:{display}"));
    SceneNode {
        label: format!("Slot:{slot_name}"),
        entity: item_entity,
        props: NodeProps::EquipmentSlot {
            slot: slot_name.into(),
            model,
            anchor,
            attachment,
            attachment_anchor,
        },
        children: vec![],
    }
}

fn find_equipment_item_for_slot(
    model_root: Entity,
    slot: EquipmentSlot,
    equipment_items: &Query<(Entity, &EquipmentItem, &ChildOf, Option<&Name>)>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    equipment_items
        .iter()
        .find(|(entity, item, _, _)| {
            item._slot == slot && belongs_to_model_root(*entity, model_root, parents)
        })
        .map(|(entity, _, _, _)| entity)
}

fn belongs_to_model_root(entity: Entity, model_root: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        current = parent.parent();
        if current == model_root {
            return true;
        }
    }
    false
}

fn equipment_item_details(
    entity: Option<Entity>,
    equipment_items: &Query<(Entity, &EquipmentItem, &ChildOf, Option<&Name>)>,
    names: &Query<&Name>,
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(entity) = entity else {
        return (None, None, None);
    };
    let Ok((_, _, parent, name)) = equipment_items.get(entity) else {
        return (None, None, None);
    };
    let anchor = names
        .get(parent.parent())
        .ok()
        .map(|name| name.as_str().to_string());
    let attachment = name.map(|name| name.as_str().to_string());
    (anchor.clone(), attachment, anchor)
}
