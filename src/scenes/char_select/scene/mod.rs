//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::f32::consts::FRAC_PI_8;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::camera::ClearColorConfig;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, ModelPresentation};
use game_engine::outfit_data::OutfitData;
use shared::protocol::CharacterListEntry;

use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::equipment::EquipmentItem;
use crate::equipment_appearance::{apply_runtime_equipment, resolve_equipment_appearance};
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::networking_auth::CharacterList;
use crate::scenes::char_select::SelectedCharIndex;
use crate::scenes::char_select::scene_tree::{self as scene_tree, ActiveWarbandSceneId};
use crate::scenes::setup::DEFAULT_M2;
use crate::scenes::warband::{SelectedWarbandScene, WarbandScenes};
use crate::skybox_m2_material::SkyboxM2Material;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

/// Marker component for all entities belonging to the char-select 3D scene.
#[derive(Component)]
pub struct CharSelectScene;

#[derive(Component, Clone)]
pub(super) struct CharSelectOrbit {
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
const CHAR_SELECT_CAMERA_GROUND_CLEARANCE: f32 = 0.5;
const CHAR_SELECT_FOG_START: f32 = 140.0;
const CHAR_SELECT_FOG_END: f32 = 220.0;
const CHAR_SELECT_CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
const CHAR_SELECT_FOG_COLOR: Color = Color::srgb(0.18, 0.2, 0.23);
const CHAR_SELECT_FOG_LIGHT_COLOR: Color = Color::srgb(0.35, 0.38, 0.42);

fn char_select_fog() -> DistanceFog {
    DistanceFog {
        color: CHAR_SELECT_FOG_COLOR,
        directional_light_color: CHAR_SELECT_FOG_LIGHT_COLOR,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::Linear {
            start: CHAR_SELECT_FOG_START,
            end: CHAR_SELECT_FOG_END,
        },
    }
}

/// Marker for the currently displayed character model root.
#[derive(Component)]
pub(super) struct CharSelectModelRoot;

#[derive(Component)]
struct CharSelectModelWrapper;
#[derive(Component)]
struct CharSelectModelCharacter(u64);
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(super) struct CharSelectSkybox {
    pub(super) path: PathBuf,
}

/// Tracks which character is currently displayed as a 3D model.
#[derive(Resource, Default)]
pub(super) struct DisplayedCharacterId(pub(super) Option<u64>);

#[derive(Resource, Default)]
pub(super) struct DisplayedCharacterAppearance(pub(super) Option<AppliedCharacterAppearance>);

#[derive(Resource, Default)]
pub(super) struct PendingSupplementalWarbandScene {
    pub(super) scene_id: Option<u32>,
    pub(super) wait_for_next_frame: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AppliedCharacterAppearance {
    character_id: u64,
    race: u8,
    class: u8,
    appearance: shared::components::CharacterAppearance,
    equipment_appearance: shared::components::EquipmentAppearance,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct CharSelectRenderAssets<'w> {
    pub(super) meshes: ResMut<'w, Assets<Mesh>>,
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
    pub(super) effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    pub(super) skybox_materials: ResMut<'w, Assets<SkyboxM2Material>>,
    pub(super) terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub(super) water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub(super) images: ResMut<'w, Assets<Image>>,
    pub(super) inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
}

pub struct CharSelectScenePlugin;

impl Plugin for CharSelectScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedCharacterId>();
        app.init_resource::<DisplayedCharacterAppearance>();
        app.init_resource::<ActiveWarbandSceneId>();
        app.init_resource::<PendingSupplementalWarbandScene>();
        app.add_systems(OnEnter(GameState::CharSelect), setup_char_select_scene);
        app.add_systems(
            Update,
            (
                sync_char_select_model,
                sync_selected_character_appearance,
                skybox::sync_char_select_skybox,
                scene_systems::sync_warband_scene_switch,
                scene_systems::spawn_pending_warband_supplemental_terrain,
                char_select_orbit_camera,
            )
                .run_if(in_state(GameState::CharSelect)),
        );
        app.add_systems(
            OnExit(GameState::CharSelect),
            scene_systems::teardown_char_select_scene,
        );
    }
}

fn single_character_focus(
    _scene: &crate::scenes::warband::WarbandSceneEntry,
    placement: &crate::scenes::warband::WarbandScenePlacement,
    presentation: ModelPresentation,
) -> Vec3 {
    let char_pos = placement.bevy_position();
    let focus_y = char_pos.y + presentation.customize_scale.max(0.01);
    Vec3::new(char_pos.x, focus_y, char_pos.z)
}

fn camera_params(
    scene: Option<&crate::scenes::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::warband::WarbandScenePlacement>,
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

fn clamp_char_select_eye(eye: Vec3, heightmap: Option<&TerrainHeightmap>) -> Vec3 {
    let mut clamped = eye;
    if let Some(terrain_y) = heightmap.and_then(|heightmap| heightmap.height_at(eye.x, eye.z)) {
        clamped.y = clamped
            .y
            .max(terrain_y + CHAR_SELECT_CAMERA_GROUND_CLEARANCE);
    }
    clamped
}

fn spawn_char_select_camera(
    commands: &mut Commands,
    scene: Option<&crate::scenes::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::warband::WarbandScenePlacement>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
) -> Entity {
    let (eye, focus, fov) = camera_params(scene, placement, presentation);
    let eye = clamp_char_select_eye(eye, heightmap);
    commands
        .spawn((
            Name::new("CharSelectCamera"),
            CharSelectScene,
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(CHAR_SELECT_CLEAR_COLOR),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: fov.to_radians(),
                ..default()
            }),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            orbit_from_eye_focus(eye, focus),
            char_select_fog(),
        ))
        .id()
}

mod background;
mod lighting;
mod scene_systems;
mod skybox;

fn char_select_orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    heightmap: Option<Res<TerrainHeightmap>>,
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
        let eye = clamp_char_select_eye(orbit_eye(&orbit), heightmap.as_deref());
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_char_select_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
    char_transform: Transform,
) -> Option<(Entity, Entity)> {
    let spawned = m2_scene::spawn_animated_static_m2_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        m2_path,
        char_transform,
        creature_display_map,
    );
    let spawned = spawned?;
    commands
        .entity(spawned.root)
        .insert((CharSelectScene, CharSelectModelWrapper));
    commands
        .entity(spawned.model_root)
        .insert(CharSelectModelRoot);
    Some((spawned.root, spawned.model_root))
}

fn single_character_rotation(
    scene: &crate::scenes::warband::WarbandSceneEntry,
    placement: &crate::scenes::warband::WarbandScenePlacement,
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
    scene: Option<&crate::scenes::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::warband::WarbandScenePlacement>,
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
    scene: &crate::scenes::warband::WarbandSceneEntry,
) -> Option<crate::scenes::warband::WarbandScenePlacement> {
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
    let scene = background::find_scene_entry(warband, selected_scene);
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

pub(super) fn selected_scene_character(
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

pub(super) fn resolve_char_select_model_path(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> Option<PathBuf> {
    let character = selected_scene_character(char_list, selected)?;
    race_model_wow_path(character.race, character.appearance.sex)
        .and_then(ensure_named_model_bundle)
        .or_else(|| {
            let p = PathBuf::from(DEFAULT_M2);
            p.exists().then_some(p)
        })
}

#[allow(clippy::too_many_arguments)]
fn setup_char_select_scene(
    mut commands: Commands,
    mut assets: CharSelectRenderAssets,
    mut heightmap: ResMut<TerrainHeightmap>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    customization_db: Res<CustomizationDb>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
    mut pending_supplemental: ResMut<PendingSupplementalWarbandScene>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
) {
    let total_start = Instant::now();
    let scene_entry = background::find_scene_entry(&warband, &selected_scene);
    let placement = warband
        .as_ref()
        .zip(scene_entry)
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let presentation = selected_character_presentation(&customization_db, &char_list, selected.0);
    let background_start = Instant::now();
    let mut bg_node = background::spawn(
        &mut commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.terrain_materials,
        &mut assets.water_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        &mut heightmap,
        scene_entry,
        placement
            .as_ref()
            .map(|placement| placement.bevy_position()),
        &mut active_scene,
    );
    let background_elapsed = background_start.elapsed();
    let camera_start = Instant::now();
    let camera_entity = spawn_char_select_camera(
        &mut commands,
        scene_entry,
        placement.as_ref(),
        Some(&heightmap),
        presentation,
    );
    let camera_elapsed = camera_start.elapsed();
    let sky_light_start = Instant::now();
    let camera_translation = camera_params(scene_entry, placement.as_ref(), presentation).0;
    let skybox_entity = background::spawn_skybox(
        &mut commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.skybox_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        &creature_display_map,
        scene_entry,
        camera_translation,
    );
    if let Some((entity, path)) =
        skybox_entity.zip(scene_entry.and_then(crate::scenes::warband::ensure_warband_skybox))
    {
        bg_node.children.push(scene_tree::skybox_scene_node(
            entity,
            path.display().to_string(),
        ));
    }
    let dir = lighting::spawn(&mut commands, scene_entry, placement.as_ref(), presentation);
    let sky_light_elapsed = sky_light_start.elapsed();
    let char_tf = resolve_char_transform(&warband, &selected_scene, Some(&heightmap), presentation);
    let model_start = Instant::now();
    let result = spawn_selected_model(
        &mut commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
        char_tf,
    );
    let model_elapsed = model_start.elapsed();
    let mut children = vec![bg_node];
    if let Some((_, entity)) = &result {
        let (race, gender, model) = scene_systems::char_info_strings(&char_list, selected.0);
        children.push(scene_tree::character_scene_node(
            *entity, model, race, gender,
        ));
    }
    displayed.0 = result.map(|(id, _)| id);
    let fov = camera_params(scene_entry, placement.as_ref(), presentation).2;
    children.extend(scene_tree::light_scene_nodes(
        camera_entity,
        fov,
        None,
        lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS,
        dir,
    ));
    commands.insert_resource(scene_tree::build_scene_tree(children));
    pending_supplemental.scene_id = scene_entry
        .filter(|scene| !crate::scenes::warband::supplemental_terrain_tile_coords(scene).is_empty())
        .map(|scene| scene.id);
    pending_supplemental.wait_for_next_frame = pending_supplemental.scene_id.is_some();
    info!(
        "setup_char_select_scene finished in {:.3}s (background={:.3}s camera={:.3}s sky+light={:.3}s model={:.3}s)",
        total_start.elapsed().as_secs_f32(),
        background_elapsed.as_secs_f32(),
        camera_elapsed.as_secs_f32(),
        sky_light_elapsed.as_secs_f32(),
        model_elapsed.as_secs_f32(),
    );
}

#[allow(clippy::too_many_arguments)]
fn sync_char_select_model(
    mut commands: Commands,
    mut assets: CharSelectRenderAssets,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    customization_db: Res<CustomizationDb>,
    heightmap: Res<TerrainHeightmap>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    current_model: Query<Entity, With<CharSelectModelWrapper>>,
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
    let scene = background::find_scene_entry(&warband, &selected_scene);
    let placement = warband
        .as_ref()
        .zip(scene)
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let char_tf = resolve_char_transform(&warband, &selected_scene, Some(&heightmap), presentation);
    displayed.0 = spawn_selected_model(
        &mut commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
        char_tf,
    )
    .map(|(id, _)| id);
    if let Some(scene) = scene {
        update_camera_for_scene(
            scene,
            placement.as_ref(),
            Some(&heightmap),
            presentation,
            &mut camera_query,
        );
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
    equipment_item_query: Query<(), With<EquipmentItem>>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::GeosetMesh>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
    mut equipment_query: Query<&mut crate::equipment::Equipment>,
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
        equipment_appearance: character.equipment_appearance.clone(),
    };
    if displayed_appearance.0 == Some(desired.clone()) {
        return;
    }
    let resolved_equipment = resolve_equipment_appearance(
        &character.equipment_appearance,
        &outfit_data,
        character.race,
        character.appearance.sex,
    );
    apply_character_customization(
        CharacterCustomizationSelection {
            race: character.race,
            class: character.class,
            sex: character.appearance.sex,
            appearance: character.appearance,
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
        &equipment_item_query,
        &material_query,
    );
    if let Ok(mut equipment) = equipment_query.get_mut(root) {
        apply_runtime_equipment(&mut equipment, &resolved_equipment);
    }
    displayed_appearance.0 = Some(desired);
}

#[allow(clippy::too_many_arguments)]
fn spawn_selected_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
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
    let (_, model_entity) = spawn_char_select_model(
        commands,
        meshes,
        materials,
        effect_materials,
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

pub(super) fn update_camera_for_scene(
    scene: &crate::scenes::warband::WarbandSceneEntry,
    placement: Option<&crate::scenes::warband::WarbandScenePlacement>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
    camera_query: &mut Query<
        (&mut Transform, &mut CharSelectOrbit, &mut Projection),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
) {
    let (eye, focus, fov) = camera_params(Some(scene), placement, presentation);
    let eye = clamp_char_select_eye(eye, heightmap);
    let orbit = orbit_from_eye_focus(eye, focus);
    for (mut tf, mut orb, mut proj) in camera_query.iter_mut() {
        *tf = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
        *orb = orbit.clone();
        if let Projection::Perspective(ref mut p) = *proj {
            p.fov = fov.to_radians();
        }
    }
}

#[cfg(test)]
mod tests;
