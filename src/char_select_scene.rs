//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::f32::consts::{FRAC_PI_8, PI};
use std::path::{Path, PathBuf};

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, ModelPresentation};
use game_engine::outfit_data::OutfitData;
use shared::protocol::CharacterListEntry;

use crate::asset;
use crate::char_select::SelectedCharIndex;
use crate::char_select_scene_tree::{self as scene_tree, ActiveWarbandSceneId, CharSelectTerrain};
use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::creature_display;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_scene;
use crate::networking_auth::CharacterList;
use crate::scene_setup::DEFAULT_M2;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::warband_scene::{SelectedWarbandScene, WarbandScenes};
use crate::water_material::WaterMaterial;

/// Marker component for all entities belonging to the char-select 3D scene.
#[derive(Component)]
pub struct CharSelectScene;

#[derive(Component, Clone)]
struct CharSelectOrbit {
    /// Current yaw offset in radians (horizontal rotation).
    yaw: f32,
    /// Starting yaw from focus to eye in radians.
    base_yaw: f32,
    /// Current pitch offset in radians (vertical rotation).
    pitch: f32,
    /// Point the camera orbits around.
    focus: Vec3,
    /// Distance from the focus point.
    distance: f32,
    /// Base pitch (the initial vertical angle).
    base_pitch: f32,
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8; // ±22.5°
const ORBIT_PITCH_LIMIT: f32 = 0.15; // ±~8.6°
const SOLO_CHARACTER_CAMERA_DISTANCE: f32 = 6.5;
const SOLO_CHARACTER_MAX_FOV_DEGREES: f32 = 55.0;

/// Marker for the currently displayed character model root.
#[derive(Component)]
struct CharSelectModelRoot;

#[derive(Component)]
struct CharSelectModelCharacter(u64);

/// Tracks which character is currently displayed as a 3D model.
#[derive(Resource, Default)]
struct DisplayedCharacterId(Option<u64>);

#[derive(Resource, Default)]
struct DisplayedCharacterAppearance(Option<AppliedCharacterAppearance>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AppliedCharacterAppearance {
    character_id: u64,
    race: u8,
    class: u8,
    appearance: shared::components::CharacterAppearance,
}

pub struct CharSelectScenePlugin;

impl Plugin for CharSelectScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedCharacterId>();
        app.init_resource::<DisplayedCharacterAppearance>();
        app.init_resource::<ActiveWarbandSceneId>();
        app.add_systems(OnEnter(GameState::CharSelect), setup_char_select_scene);
        app.add_systems(
            Update,
            (
                sync_char_select_model,
                sync_selected_character_appearance,
                sync_warband_scene_switch,
                char_select_orbit_camera,
            )
                .run_if(in_state(GameState::CharSelect)),
        );
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_scene);
    }
}

fn single_character_focus(
    _scene: &crate::warband_scene::WarbandSceneEntry,
    placement: &crate::warband_scene::WarbandScenePlacement,
    presentation: ModelPresentation,
) -> Vec3 {
    let char_pos = placement.bevy_position();
    let focus_y = char_pos.y + presentation.customize_scale.max(0.01);
    Vec3::new(char_pos.x, focus_y, char_pos.z)
}

fn camera_params(
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> (Vec3, Vec3, f32) {
    if let Some(s) = scene {
        let scene_eye = s.bevy_position();
        let scene_focus = s.bevy_look_at();
        let focus = placement
            .map(|placement| single_character_focus(s, placement, presentation))
            .unwrap_or(scene_focus);
        let eye = if placement.is_some() {
            let distance = (SOLO_CHARACTER_CAMERA_DISTANCE + presentation.camera_distance_offset)
                .clamp(3.5, (scene_eye - scene_focus).length());
            solo_camera_eye(scene_eye, scene_focus, focus, distance)
        } else {
            scene_eye
        };
        let fov = if placement.is_some() {
            s.fov.min(SOLO_CHARACTER_MAX_FOV_DEGREES)
        } else {
            s.fov
        };
        (eye, focus, fov)
    } else {
        (Vec3::new(0.0, 1.8, 6.0), Vec3::new(0.0, 1.0, 0.0), 45.0)
    }
}

fn solo_camera_eye(scene_eye: Vec3, scene_focus: Vec3, focus: Vec3, distance: f32) -> Vec3 {
    let scene_offset = scene_eye - scene_focus;
    let vertical = scene_offset.y;
    let horizontal = Vec3::new(scene_offset.x, 0.0, scene_offset.z);
    let horizontal_dir = horizontal.normalize_or_zero();
    let horizontal_distance = (distance * distance - vertical * vertical).max(0.0).sqrt();
    focus + horizontal_dir * horizontal_distance + Vec3::Y * vertical
}

fn orbit_from_eye_focus(eye: Vec3, focus: Vec3) -> CharSelectOrbit {
    let offset = eye - focus;
    let distance = offset.length();
    let base_yaw = offset.x.atan2(offset.z);
    let base_pitch = if distance > 0.0 {
        (offset.y / distance).asin()
    } else {
        0.0
    };
    CharSelectOrbit {
        yaw: 0.0,
        base_yaw,
        pitch: 0.0,
        focus,
        distance,
        base_pitch,
    }
}

fn orbit_eye(orbit: &CharSelectOrbit) -> Vec3 {
    let yaw = orbit.base_yaw + orbit.yaw;
    let pitch = orbit.base_pitch + orbit.pitch;
    orbit.focus
        + Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ) * orbit.distance
}

fn spawn_char_select_camera(
    commands: &mut Commands,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> Entity {
    let (eye, focus, fov) = camera_params(scene, placement, presentation);
    commands
        .spawn((
            Name::new("CharSelectCamera"),
            CharSelectScene,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: fov.to_radians(),
                ..default()
            }),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            orbit_from_eye_focus(eye, focus),
        ))
        .id()
}

fn char_select_orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut CharSelectOrbit, &mut Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    for (mut orbit, mut transform) in &mut query {
        orbit.yaw =
            (orbit.yaw - delta.x * ORBIT_SENSITIVITY).clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + delta.y * ORBIT_SENSITIVITY)
            .clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        let eye = orbit_eye(&orbit);
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
    }
}

fn spawn_char_select_lighting(commands: &mut Commands) -> Entity {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.85),
        brightness: 80.0,
        ..default()
    });
    // Keep the moody upper-left key, but bias it closer to the camera for clearer facial read.
    let directional = commands
        .spawn((
            Name::new("DirectionalLight"),
            CharSelectScene,
            DirectionalLight {
                illuminance: 8000.0,
                shadows_enabled: true,
                color: Color::srgb(1.0, 0.92, 0.8),
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.8, PI / 10.0, 0.0)),
        ))
        .id();
    directional
}

#[allow(clippy::too_many_arguments)]
fn spawn_char_select_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
    char_transform: Transform,
) -> Option<Entity> {
    let entity = m2_scene::spawn_animated_static_m2(
        commands,
        meshes,
        materials,
        images,
        inv_bp,
        m2_path,
        char_transform,
        creature_display_map,
    );
    if let Some(e) = entity {
        commands
            .entity(e)
            .insert((CharSelectScene, CharSelectModelRoot));
        Some(e)
    } else {
        None
    }
}

fn single_character_rotation(
    scene: &crate::warband_scene::WarbandSceneEntry,
    placement: &crate::warband_scene::WarbandScenePlacement,
    presentation: ModelPresentation,
) -> Quat {
    let (eye, _, _) = camera_params(Some(scene), Some(placement), presentation);
    let to_camera = eye - placement.bevy_position();
    let horizontal = Vec3::new(to_camera.x, 0.0, to_camera.z).normalize_or_zero();
    if horizontal == Vec3::ZERO {
        placement.bevy_rotation()
    } else {
        Quat::from_rotation_arc(Vec3::X, horizontal)
    }
}

fn character_transform(
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
) -> Transform {
    if let Some(placement) = placement {
        let rotation = scene
            .map(|scene| single_character_rotation(scene, placement, presentation))
            .unwrap_or_else(|| placement.bevy_rotation());
        let mut translation = placement.bevy_position();
        if let Some(terrain_y) =
            heightmap.and_then(|heightmap| heightmap.height_at(translation.x, translation.z))
        {
            translation.y = translation.y.max(terrain_y);
        }
        Transform::from_translation(translation)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(presentation.customize_scale.max(0.01)))
    } else {
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(presentation.customize_scale.max(0.01)))
    }
}

fn default_char_transform() -> Transform {
    Transform::from_xyz(0.0, 0.0, 0.0)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
}

fn selected_character_presentation(
    customization_db: &CustomizationDb,
    char_list: &CharacterList,
    selected: Option<usize>,
) -> ModelPresentation {
    selected_scene_character(char_list, selected)
        .map(|character| {
            customization_db.presentation_for(character.race, character.appearance.sex)
        })
        .unwrap_or_default()
}

fn selected_scene_placement<'a>(
    warband: &'a WarbandScenes,
    scene: &crate::warband_scene::WarbandSceneEntry,
) -> Option<crate::warband_scene::WarbandScenePlacement> {
    warband
        .solo_character_placement(scene)
        .or_else(|| warband.first_character_placement(scene.id).cloned())
        .or_else(|| warband.first_placement(scene.id).cloned())
}

fn resolve_char_transform(
    warband: &Option<Res<WarbandScenes>>,
    selected_scene: &Option<Res<SelectedWarbandScene>>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
) -> Transform {
    let scene = find_scene_entry(warband, selected_scene);
    let placement = warband
        .as_ref()
        .zip(scene)
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    if scene.is_some() || placement.is_some() {
        character_transform(scene, placement.as_ref(), heightmap, presentation)
    } else {
        default_char_transform()
    }
}

fn selected_scene_character(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> Option<&CharacterListEntry> {
    selected
        .and_then(|index| char_list.0.get(index))
        .or_else(|| char_list.0.first())
}

fn selected_scene_character_id(char_list: &CharacterList, selected: Option<usize>) -> Option<u64> {
    selected_scene_character(char_list, selected).map(|character| character.character_id)
}

use crate::character_models::{ensure_named_model_bundle, race_model_wow_path, race_name};

fn fallback_model_path() -> Option<PathBuf> {
    let default_path = PathBuf::from(DEFAULT_M2);
    default_path.exists().then_some(default_path)
}

fn resolve_char_select_model_path(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> Option<PathBuf> {
    selected_scene_character(char_list, selected)
        .and_then(|character| race_model_wow_path(character.race, character.appearance.sex))
        .and_then(ensure_named_model_bundle)
        .or_else(fallback_model_path)
}

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

fn find_scene_entry<'a>(
    warband: &'a Option<Res<WarbandScenes>>,
    selected: &Option<Res<SelectedWarbandScene>>,
) -> Option<&'a crate::warband_scene::WarbandSceneEntry> {
    warband
        .as_ref()
        .zip(selected.as_ref())
        .and_then(|(w, sel)| w.scenes.iter().find(|s| s.id == sel.scene_id))
}

#[allow(clippy::too_many_arguments)]
fn spawn_background(
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
        && let Some(e) = scene_tree::spawn_warband_terrain(
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
        return scene_tree::background_scene_node(
            e,
            &format!("terrain:{}_{ty}_{tx}", s.map_name()),
        );
    }
    let ground = spawn_tagged_ground(commands, meshes, materials, images);
    scene_tree::ground_scene_node(ground)
}

#[allow(clippy::too_many_arguments)]
fn setup_char_select_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut heightmap: ResMut<TerrainHeightmap>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    customization_db: Res<CustomizationDb>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
) {
    let scene_entry = find_scene_entry(&warband, &selected_scene);
    let placement = warband
        .as_ref()
        .zip(scene_entry)
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let presentation = selected_character_presentation(&customization_db, &char_list, selected.0);
    let camera_entity =
        spawn_char_select_camera(&mut commands, scene_entry, placement.as_ref(), presentation);
    let dir = spawn_char_select_lighting(&mut commands);
    let bg_node = spawn_background(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut terrain_materials,
        &mut water_materials,
        &mut images,
        &mut inv_bp,
        &mut heightmap,
        scene_entry,
        &mut active_scene,
    );
    let char_tf = resolve_char_transform(&warband, &selected_scene, Some(&heightmap), presentation);
    let result = spawn_selected_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
        char_tf,
    );
    let mut children = vec![bg_node];
    if let Some((_, entity)) = &result {
        let (race, gender, model) = char_info_strings(&char_list, selected.0);
        children.push(scene_tree::character_scene_node(
            *entity, model, race, gender,
        ));
    }
    displayed.0 = result.map(|(id, _)| id);
    let fov = camera_params(scene_entry, placement.as_ref(), presentation).2;
    children.extend(scene_tree::light_scene_nodes(camera_entity, fov, None, dir));
    commands.insert_resource(scene_tree::build_scene_tree(children));
}

#[allow(clippy::too_many_arguments)]
fn sync_char_select_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    customization_db: Res<CustomizationDb>,
    heightmap: Res<TerrainHeightmap>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    current_model: Query<Entity, With<CharSelectModelRoot>>,
    mut displayed: ResMut<DisplayedCharacterId>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
    mut camera_query: Query<
        (&mut Transform, &mut CharSelectOrbit, &mut Projection),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
) {
    let desired = selected_scene_character_id(&char_list, selected.0);
    if displayed.0 == desired {
        return;
    }
    for entity in current_model.iter() {
        commands.entity(entity).despawn();
    }
    let presentation = selected_character_presentation(&customization_db, &char_list, selected.0);
    let char_tf = resolve_char_transform(&warband, &selected_scene, Some(&heightmap), presentation);
    displayed.0 = spawn_selected_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
        char_tf,
    )
    .map(|(id, _)| id);
    if let Some(scene) = find_scene_entry(&warband, &selected_scene) {
        let placement = warband
            .as_ref()
            .and_then(|warband| selected_scene_placement(warband, scene));
        update_camera_for_scene(scene, placement.as_ref(), presentation, &mut camera_query);
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_selected_character_appearance(
    customization_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    outfit_data: Res<OutfitData>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    mut displayed_appearance: ResMut<DisplayedCharacterAppearance>,
    root_query: Query<(Entity, &CharSelectModelCharacter), With<CharSelectModelRoot>>,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(character) = selected_scene_character(&char_list, selected.0) else {
        displayed_appearance.0 = None;
        return;
    };
    let Ok((root, root_character)) = root_query.single() else {
        return;
    };
    if root_character.0 != character.character_id {
        return;
    }
    let desired = AppliedCharacterAppearance {
        character_id: character.character_id,
        race: character.race,
        class: character.class,
        appearance: character.appearance,
    };
    if displayed_appearance.0 == Some(desired) {
        return;
    }
    apply_character_customization(
        CharacterCustomizationSelection {
            race: character.race,
            class: character.class,
            sex: character.appearance.sex,
            appearance: character.appearance,
        },
        &customization_db,
        &char_tex,
        &outfit_data,
        root,
        &mut images,
        &mut materials,
        &parent_query,
        &geoset_query,
        &mut visibility_query,
        &material_query,
    );
    displayed_appearance.0 = Some(desired);
}

#[allow(clippy::too_many_arguments)]
fn spawn_selected_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    char_list: &CharacterList,
    selected: Option<usize>,
    char_transform: Transform,
) -> Option<(u64, Entity)> {
    let model_path = resolve_char_select_model_path(char_list, selected)?;
    if !model_path.exists() {
        return None;
    }
    let model_entity = spawn_char_select_model(
        commands,
        meshes,
        materials,
        images,
        inv_bp,
        &model_path,
        creature_display_map,
        char_transform,
    )?;
    let char_id = selected_scene_character_id(char_list, selected)?;
    commands
        .entity(model_entity)
        .insert(CharSelectModelCharacter(char_id));
    Some((char_id, model_entity))
}

fn update_camera_for_scene(
    scene: &crate::warband_scene::WarbandSceneEntry,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    presentation: ModelPresentation,
    camera_query: &mut Query<
        (&mut Transform, &mut CharSelectOrbit, &mut Projection),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
) {
    let (eye, focus, fov) = camera_params(Some(scene), placement, presentation);
    let orbit = orbit_from_eye_focus(eye, focus);
    for (mut tf, mut orb, mut proj) in camera_query.iter_mut() {
        *tf = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
        *orb = orbit.clone();
        if let Projection::Perspective(ref mut p) = *proj {
            p.fov = fov.to_radians();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_warband_scene_switch(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
    mut heightmap: ResMut<TerrainHeightmap>,
    customization_db: Res<CustomizationDb>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
    terrain_query: Query<Entity, With<CharSelectTerrain>>,
    mut camera_query: Query<
        (&mut Transform, &mut CharSelectOrbit, &mut Projection),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
) {
    let Some(warband) = warband else { return };
    let Some(sel) = selected_scene else { return };
    if active_scene.0 == Some(sel.scene_id) {
        return;
    }
    let Some(scene) = warband.scenes.iter().find(|s| s.id == sel.scene_id) else {
        return;
    };
    let placement = selected_scene_placement(&warband, scene);
    let presentation = selected_character_presentation(&customization_db, &char_list, selected.0);
    for entity in terrain_query.iter() {
        commands.entity(entity).despawn();
    }
    let _ = scene_tree::spawn_warband_terrain(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut terrain_materials,
        &mut water_materials,
        &mut images,
        &mut inv_bp,
        &mut heightmap,
        scene,
    );
    update_camera_for_scene(scene, placement.as_ref(), presentation, &mut camera_query);
    active_scene.0 = Some(sel.scene_id);
}

fn teardown_char_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharSelectScene>>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut displayed_appearance: ResMut<DisplayedCharacterAppearance>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = None;
    displayed_appearance.0 = None;
    active_scene.0 = None;
    commands.remove_resource::<game_engine::scene_tree::SceneTree>();
}

fn char_info_strings(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> (String, String, String) {
    let character = selected_scene_character(char_list, selected);
    let race = character
        .map(|c| race_name(c.race).to_string())
        .unwrap_or_else(|| "Unknown".into());
    let gender = character
        .map(|c| {
            if c.appearance.sex == 0 {
                "Male"
            } else {
                "Female"
            }
        })
        .unwrap_or("Unknown")
        .to_string();
    let model = resolve_char_select_model_path(char_list, selected)
        .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".into());
    (race, gender, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking_auth::CharacterList;
    use shared::components::CharacterAppearance;
    use shared::protocol::CharacterListEntry;

    fn character(character_id: u64, race: u8, sex: u8, name: &str) -> CharacterListEntry {
        CharacterListEntry {
            character_id,
            name: name.to_string(),
            level: 1,
            race,
            class: 1,
            appearance: CharacterAppearance {
                sex,
                ..Default::default()
            },
        }
    }

    #[test]
    fn selected_scene_character_id_uses_selected_index() {
        let char_list = CharacterList(vec![
            character(10, 1, 0, "First"),
            character(20, 2, 0, "Second"),
        ]);

        assert_eq!(selected_scene_character_id(&char_list, Some(1)), Some(20));
    }

    #[test]
    fn selected_scene_character_id_falls_back_to_first_character() {
        let char_list = CharacterList(vec![
            character(10, 1, 0, "First"),
            character(20, 2, 0, "Second"),
        ]);

        assert_eq!(selected_scene_character_id(&char_list, None), Some(10));
        assert_eq!(selected_scene_character_id(&char_list, Some(99)), Some(10));
    }

    #[test]
    fn race_model_wow_path_covers_known_playable_races_and_sex() {
        assert_eq!(
            race_model_wow_path(1, 0),
            Some("character/human/male/humanmale_hd.m2")
        );
        assert_eq!(
            race_model_wow_path(2, 0),
            Some("character/orc/male/orcmale_hd.m2")
        );
        assert_eq!(
            race_model_wow_path(10, 1),
            Some("character/bloodelf/female/bloodelffemale_hd.m2")
        );
        assert_eq!(
            race_model_wow_path(10, 0),
            Some("character/bloodelf/male/bloodelfmale_hd.m2")
        );
        assert_eq!(race_model_wow_path(99, 0), None);
    }

    #[test]
    fn camera_params_center_focused_placement_horizontally() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let (eye, focus, _) =
            camera_params(Some(scene), Some(&placement), ModelPresentation::default());
        let forward = (focus - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let rel = placement.bevy_position() - eye;

        assert!(
            rel.dot(right).abs() < 0.001,
            "focused placement should sit on the camera centerline"
        );
    }

    #[test]
    fn camera_params_use_tighter_single_character_framing() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let presentation = ModelPresentation::default();
        let (scene_eye, scene_focus, scene_fov) = camera_params(Some(scene), None, presentation);
        let (eye, focus, fov) = camera_params(Some(scene), Some(&placement), presentation);

        assert!(
            eye.distance(focus) < scene_eye.distance(scene_focus),
            "single-character framing should move the camera closer than the raw warband overview"
        );
        assert!(
            fov < scene_fov,
            "single-character framing should narrow the FOV from the warband overview"
        );
    }

    #[test]
    fn camera_params_use_model_center_height_for_single_character_focus() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let presentation = ModelPresentation {
            customize_scale: 1.1,
            camera_distance_offset: -0.34,
        };

        let (_, focus, _) = camera_params(Some(scene), Some(&placement), presentation);

        assert!(
            (focus.y - (placement.bevy_position().y + presentation.customize_scale)).abs() < 0.001,
            "single-character focus should target model center height, got focus_y={} placement_y={} scale={}",
            focus.y,
            placement.bevy_position().y,
            presentation.customize_scale
        );
    }

    #[test]
    fn camera_params_preserve_authored_vertical_offset_for_single_character_eye() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let presentation = ModelPresentation {
            customize_scale: 1.1,
            camera_distance_offset: -0.34,
        };

        let (eye, focus, _) = camera_params(Some(scene), Some(&placement), presentation);
        let authored_vertical = scene.bevy_position().y - scene.bevy_look_at().y;

        assert!(
            ((eye.y - focus.y) - authored_vertical).abs() < 0.001,
            "single-character eye should preserve authored vertical lift, got {} expected {}",
            eye.y - focus.y,
            authored_vertical
        );
    }

    #[test]
    fn character_transform_snaps_character_up_to_warband_terrain() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let adt_path =
            crate::warband_scene::ensure_warband_terrain(scene).expect("expected warband terrain");
        let data = std::fs::read(&adt_path).expect("expected ADT data");
        let adt = crate::asset::adt::load_adt(&data).expect("expected ADT parse");
        let mut heightmap = TerrainHeightmap::default();
        let (tile_y, tile_x) = scene.tile_coords();
        heightmap.insert_tile(tile_y, tile_x, &adt);

        let transform = character_transform(
            Some(scene),
            Some(&placement),
            Some(&heightmap),
            ModelPresentation::default(),
        );
        let terrain_y = heightmap
            .height_at(transform.translation.x, transform.translation.z)
            .expect("expected terrain at placement");

        assert!(
            (transform.translation.y - terrain_y).abs() < 0.001,
            "character root should sit on terrain, got root_y={} terrain_y={terrain_y}",
            transform.translation.y
        );
    }

    #[test]
    fn focused_placement_rotation_faces_camera_reasonably() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
        let (eye, _, _) =
            camera_params(Some(scene), Some(&placement), ModelPresentation::default());
        let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
        let facing = rotation * Vec3::X;
        let angle = facing.angle_between(to_camera).to_degrees();

        assert!(
            angle < 25.0,
            "focused placement should face mostly toward the camera, got {angle:.2} degrees"
        );
    }

    #[test]
    fn camera_params_apply_model_distance_offset() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let default_presentation = ModelPresentation::default();
        let human_presentation = ModelPresentation {
            customize_scale: 1.1,
            camera_distance_offset: -0.34,
        };
        let (default_eye, default_focus, _) =
            camera_params(Some(scene), Some(&placement), default_presentation);
        let (eye, focus, _) = camera_params(Some(scene), Some(&placement), human_presentation);

        assert!(eye.distance(focus) < default_eye.distance(default_focus));
    }

    #[test]
    fn orbit_from_eye_focus_preserves_initial_yaw() {
        let eye = Vec3::new(4.0, 3.0, -2.0);
        let focus = Vec3::new(1.5, 1.0, 0.5);

        let orbit = orbit_from_eye_focus(eye, focus);

        assert!(
            orbit_eye(&orbit).distance(eye) < 1e-5,
            "reconstructed orbit eye should match the authored eye position"
        );
    }

    #[test]
    fn focused_placement_rotation_faces_camera_tightly() {
        let warband = crate::warband_scene::WarbandScenes::load();
        let scene = warband
            .scenes
            .iter()
            .find(|scene| scene.id == 1)
            .expect("Adventurer's Rest");
        let placement = selected_scene_placement(&warband, scene).expect("expected placement");
        let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
        let (eye, _, _) =
            camera_params(Some(scene), Some(&placement), ModelPresentation::default());
        let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
        let to_camera = Vec3::new(to_camera.x, 0.0, to_camera.z).normalize_or_zero();
        let facing = rotation * Vec3::X;
        let facing = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
        let angle = facing.angle_between(to_camera).to_degrees();

        assert!(
            angle < 1.0,
            "single-character rotation should face the camera horizontally, got {angle:.2} degrees"
        );
    }
}
