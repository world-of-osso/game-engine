//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, ground, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::f32::consts::PI;
use std::path::{Path, PathBuf};

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use shared::protocol::CharacterListEntry;

use crate::asset;
use crate::char_select::SelectedCharIndex;
use crate::creature_display;
use crate::game_state::GameState;
use crate::ground;
use crate::m2_scene;
use crate::networking_auth::CharacterList;
use crate::scene_setup::DEFAULT_M2;

/// Marker component for all entities belonging to the char-select 3D scene.
#[derive(Component)]
pub struct CharSelectScene;

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
        app.add_systems(OnEnter(GameState::CharSelect), setup_char_select_scene);
        app.add_systems(
            Update,
            sync_char_select_model.run_if(in_state(GameState::CharSelect)),
        );
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_scene);
    }
}

/// Camera settings for the char select scene: fixed angle, no player control.
fn spawn_char_select_camera(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("CharSelectCamera"),
            CharSelectScene,
            Camera3d::default(),
            Transform::from_xyz(0.0, 1.8, 6.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ))
        .id()
}

fn spawn_char_select_lighting(commands: &mut Commands) {
    // Warm ambient for campfire mood
    commands.spawn((
        CharSelectScene,
        AmbientLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            brightness: 80.0,
            ..default()
        },
    ));
    // Key light from upper-left
    commands.spawn((
        CharSelectScene,
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.92, 0.8),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 6.0, 0.0)),
    ));
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
) -> bool {
    // Spawn as static (no Player component, no movement controls)
    let entity = m2_scene::spawn_static_m2(
        commands,
        meshes,
        materials,
        images,
        inv_bp,
        m2_path,
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        creature_display_map,
    );
    if let Some(e) = entity {
        commands
            .entity(e)
            .insert((CharSelectScene, CharSelectModelRoot));
        true
    } else {
        false
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

fn race_model_wow_path(race: u8, sex: u8) -> Option<&'static str> {
    match (race, sex) {
        (1, 0) => Some("character/human/male/humanmale_hd.m2"),
        (1, 1) => Some("character/human/female/humanfemale_hd.m2"),
        (2, 0) => Some("character/orc/male/orcmale_hd.m2"),
        (2, 1) => Some("character/orc/female/orcfemale_hd.m2"),
        (3, 0) => Some("character/dwarf/male/dwarfmale_hd.m2"),
        (3, 1) => Some("character/dwarf/female/dwarffemale_hd.m2"),
        (4, 0) => Some("character/nightelf/male/nightelfmale_hd.m2"),
        (4, 1) => Some("character/nightelf/female/nightelffemale_hd.m2"),
        (5, 0) => Some("character/scourge/male/scourgemale_hd.m2"),
        (5, 1) => Some("character/scourge/female/scourgefemale_hd.m2"),
        (6, 0) => Some("character/tauren/male/taurenmale_hd.m2"),
        (6, 1) => Some("character/tauren/female/taurenfemale_hd.m2"),
        (7, 0) => Some("character/gnome/male/gnomemale_hd.m2"),
        (7, 1) => Some("character/gnome/female/gnomefemale_hd.m2"),
        (8, 0) => Some("character/troll/male/trollmale_hd.m2"),
        (8, 1) => Some("character/troll/female/trollfemale_hd.m2"),
        (10, 0) => Some("character/bloodelf/male/bloodelfmale_hd.m2"),
        (10, 1) => Some("character/bloodelf/female/bloodelffemale_hd.m2"),
        (11, 0) => Some("character/draenei/male/draeneimale_hd.m2"),
        (11, 1) => Some("character/draenei/female/draeneifemale_hd.m2"),
        _ => None,
    }
}

fn ensure_named_model_bundle(wow_model_path: &str) -> Option<PathBuf> {
    let model_path = ensure_named_model_asset(wow_model_path)?;
    let Some(parent) = Path::new(wow_model_path).parent() else {
        return Some(model_path);
    };
    let Some(stem) = Path::new(wow_model_path)
        .file_stem()
        .and_then(|s| s.to_str())
    else {
        return Some(model_path);
    };

    let skin_path = parent.join(format!("{stem}00.skin"));
    if let Some(skin_path) = skin_path.to_str() {
        let _ = ensure_named_model_asset(skin_path);
    }

    let skel_path = parent.join(format!("{stem}.skel"));
    if let Some(skel_path) = skel_path.to_str() {
        let _ = ensure_named_model_asset(skel_path);
    }

    Some(model_path)
}

fn ensure_named_model_asset(wow_path: &str) -> Option<PathBuf> {
    let file_name = Path::new(wow_path).file_name()?;
    let out_path = Path::new("data/models").join(file_name);
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    asset::casc_resolver::ensure_file_at_path(fdid, &out_path)
}

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
) {
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
    commands.spawn((
        CharSelectScene,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
    ));
}

#[allow(clippy::too_many_arguments)]
fn setup_char_select_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    char_list: Res<CharacterList>,
    selected: Res<SelectedCharIndex>,
    mut displayed: ResMut<DisplayedCharacterId>,
) {
    spawn_char_select_camera(&mut commands);
    spawn_char_select_lighting(&mut commands);
    spawn_tagged_ground(&mut commands, &mut meshes, &mut materials, &mut images);
    displayed.0 = spawn_selected_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
    );
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
) {
    let desired = selected_scene_character_id(&char_list, selected.0);
    if displayed.0 == desired {
        return;
    }
    for entity in current_model.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = spawn_selected_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        &char_list,
        selected.0,
    );
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
) -> Option<u64> {
    let model_path = resolve_char_select_model_path(char_list, selected)?;
    if !model_path.exists() {
        return None;
    }
    if spawn_char_select_model(
        commands,
        meshes,
        materials,
        images,
        inv_bp,
        &model_path,
        creature_display_map,
    ) {
        selected_scene_character_id(char_list, selected)
    } else {
        None
    }
}

fn teardown_char_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharSelectScene>>,
    mut displayed: ResMut<DisplayedCharacterId>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = None;
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
