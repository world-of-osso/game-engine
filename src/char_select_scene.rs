//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::f32::consts::{FRAC_PI_8, PI};
use std::path::{Path, PathBuf};

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use shared::protocol::CharacterListEntry;

use crate::asset;
use crate::char_select::SelectedCharIndex;
use crate::char_select_scene_tree::{self as scene_tree, ActiveWarbandSceneId, CharSelectTerrain};
use crate::creature_display;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_scene;
use crate::networking_auth::CharacterList;
use crate::scene_setup::DEFAULT_M2;
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

/// Marker for the currently displayed character model root.
#[derive(Component)]
struct CharSelectModelRoot;

/// Tracks which character is currently displayed as a 3D model.
#[derive(Resource, Default)]
struct DisplayedCharacterId(Option<u64>);

pub struct CharSelectScenePlugin;

impl Plugin for CharSelectScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedCharacterId>();
        app.init_resource::<ActiveWarbandSceneId>();
        app.add_systems(OnEnter(GameState::CharSelect), setup_char_select_scene);
        app.add_systems(
            Update,
            (sync_char_select_model, sync_warband_scene_switch, char_select_orbit_camera)
                .run_if(in_state(GameState::CharSelect)),
        );
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_scene);
    }
}

fn camera_params(scene: Option<&crate::warband_scene::WarbandSceneEntry>) -> (Vec3, Vec3, f32) {
    if let Some(s) = scene {
        (s.bevy_position(), s.bevy_look_at(), s.fov)
    } else {
        (Vec3::new(0.0, 1.8, 6.0), Vec3::new(0.0, 1.0, 0.0), 45.0)
    }
}

fn orbit_from_eye_focus(eye: Vec3, focus: Vec3) -> CharSelectOrbit {
    let offset = eye - focus;
    let distance = offset.length();
    let base_pitch = if distance > 0.0 { (offset.y / distance).asin() } else { 0.0 };
    CharSelectOrbit { yaw: 0.0, pitch: 0.0, focus, distance, base_pitch }
}

fn spawn_char_select_camera(
    commands: &mut Commands,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
) -> Entity {
    let (eye, focus, fov) = camera_params(scene);
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
        orbit.yaw = (orbit.yaw - delta.x * ORBIT_SENSITIVITY).clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + delta.y * ORBIT_SENSITIVITY).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);

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

fn spawn_char_select_lighting(commands: &mut Commands) -> (Entity, Entity) {
    // Warm ambient for campfire mood
    let ambient = commands
        .spawn((
            Name::new("AmbientLight"),
            CharSelectScene,
            AmbientLight {
                color: Color::srgb(1.0, 0.95, 0.85),
                brightness: 80.0,
                ..default()
            },
        ))
        .id();
    // Key light from upper-left
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
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
        ))
        .id();
    (ambient, directional)
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
    let entity = m2_scene::spawn_static_m2(
        commands, meshes, materials, images, inv_bp,
        m2_path, char_transform, creature_display_map,
    );
    if let Some(e) = entity {
        commands.entity(e).insert((CharSelectScene, CharSelectModelRoot));
        Some(e)
    } else {
        None
    }
}

fn character_transform(warband: &WarbandScenes, scene_id: u32) -> Transform {
    if let Some(placement) = warband.first_placement(scene_id) {
        Transform::from_translation(placement.bevy_position())
            .with_rotation(placement.bevy_rotation())
    } else {
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
    }
}

fn default_char_transform() -> Transform {
    Transform::from_xyz(0.0, 0.0, 0.0)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
}

fn resolve_char_transform(
    warband: &Option<Res<WarbandScenes>>,
    selected_scene: &Option<Res<SelectedWarbandScene>>,
) -> Transform {
    warband.as_ref()
        .zip(selected_scene.as_ref())
        .map(|(w, sel)| character_transform(&w, sel.scene_id))
        .unwrap_or_else(default_char_transform)
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
    warband.as_ref()
        .zip(selected.as_ref())
        .and_then(|(w, sel)| w.scenes.iter().find(|s| s.id == sel.scene_id))
}

fn spawn_background(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    active: &mut ActiveWarbandSceneId,
) -> game_engine::scene_tree::SceneNode {
    if let Some(s) = scene {
        if let Some(e) = scene_tree::spawn_warband_terrain(
            commands, meshes, materials, terrain_materials,
            water_materials, images, inv_bp, s,
        ) {
            active.0 = Some(s.id);
            let (ty, tx) = s.tile_coords();
            return scene_tree::background_scene_node(e, &format!("terrain:{}_{ty}_{tx}", s.map_name()));
        }
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
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
) {
    let scene_entry = find_scene_entry(&warband, &selected_scene);
    let camera_entity = spawn_char_select_camera(&mut commands, scene_entry);
    let (ambient, dir) = spawn_char_select_lighting(&mut commands);
    let bg_node = spawn_background(
        &mut commands, &mut meshes, &mut materials, &mut terrain_materials,
        &mut water_materials, &mut images, &mut inv_bp, scene_entry, &mut active_scene,
    );
    let char_tf = resolve_char_transform(&warband, &selected_scene);
    let result = spawn_selected_model(
        &mut commands, &mut meshes, &mut materials, &mut images,
        &mut inv_bp, &creature_display_map, &char_list, selected.0, char_tf,
    );
    let mut children = vec![bg_node];
    if let Some((_, entity)) = &result {
        let (race, gender, model) = char_info_strings(&char_list, selected.0);
        children.push(scene_tree::character_scene_node(*entity, model, race, gender));
    }
    displayed.0 = result.map(|(id, _)| id);
    let fov = scene_entry.map(|s| s.fov).unwrap_or(45.0);
    children.extend(scene_tree::light_scene_nodes(camera_entity, fov, ambient, dir));
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
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    current_model: Query<Entity, With<CharSelectModelRoot>>,
    mut displayed: ResMut<DisplayedCharacterId>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
) {
    let desired = selected_scene_character_id(&char_list, selected.0);
    if displayed.0 == desired {
        return;
    }
    for entity in current_model.iter() {
        commands.entity(entity).despawn();
    }
    let char_tf = resolve_char_transform(&warband, &selected_scene);
    displayed.0 = spawn_selected_model(
        &mut commands, &mut meshes, &mut materials, &mut images,
        &mut inv_bp, &creature_display_map, &char_list, selected.0, char_tf,
    )
    .map(|(id, _)| id);
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
        commands, meshes, materials, images, inv_bp,
        &model_path, creature_display_map, char_transform,
    )?;
    let char_id = selected_scene_character_id(char_list, selected)?;
    Some((char_id, model_entity))
}

fn update_camera_for_scene(
    scene: &crate::warband_scene::WarbandSceneEntry,
    camera_query: &mut Query<(&mut Transform, &mut CharSelectOrbit, &mut Projection), With<CharSelectScene>>,
) {
    let (eye, focus, fov) = camera_params(Some(scene));
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
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
    terrain_query: Query<Entity, With<CharSelectTerrain>>,
    mut camera_query: Query<(&mut Transform, &mut CharSelectOrbit, &mut Projection), With<CharSelectScene>>,
) {
    let Some(warband) = warband else { return };
    let Some(sel) = selected_scene else { return };
    if active_scene.0 == Some(sel.scene_id) {
        return;
    }
    let Some(scene) = warband.scenes.iter().find(|s| s.id == sel.scene_id) else {
        return;
    };
    for entity in terrain_query.iter() {
        commands.entity(entity).despawn();
    }
    let _ = scene_tree::spawn_warband_terrain(
        &mut commands, &mut meshes, &mut materials, &mut terrain_materials,
        &mut water_materials, &mut images, &mut inv_bp, scene,
    );
    update_camera_for_scene(scene, &mut camera_query);
    active_scene.0 = Some(sel.scene_id);
}

fn teardown_char_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharSelectScene>>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = None;
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
}
