//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
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
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};
use crate::scenes::setup::DEFAULT_M2;
use crate::skybox_m2_material::SkyboxM2Material;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

/// Marker component for all entities belonging to the char-select 3D scene.
#[derive(Component)]
pub struct CharSelectScene;

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
                camera::char_select_orbit_camera,
            )
                .run_if(in_state(GameState::CharSelect)),
        );
        app.add_systems(
            OnExit(GameState::CharSelect),
            scene_systems::teardown_char_select_scene,
        );
    }
}

mod background;
mod camera;
mod lighting;
mod scene_systems;
mod skybox;

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
    scene: &crate::scenes::char_select::warband::WarbandSceneEntry,
    placement: &crate::scenes::char_select::warband::WarbandScenePlacement,
    presentation: ModelPresentation,
) -> Quat {
    let (eye, _, _) = camera::camera_params(Some(scene), Some(placement), presentation);
    let to_camera = eye - placement.bevy_position();
    let horizontal = Vec3::new(to_camera.x, 0.0, to_camera.z).normalize_or_zero();
    if horizontal == Vec3::ZERO {
        placement.bevy_rotation()
    } else {
        Quat::from_rotation_arc(Vec3::X, horizontal)
    }
}

fn character_transform(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::char_select::warband::WarbandScenePlacement>,
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
    scene: &crate::scenes::char_select::warband::WarbandSceneEntry,
) -> Option<crate::scenes::char_select::warband::WarbandScenePlacement> {
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
    let camera_entity = camera::spawn_char_select_camera(
        &mut commands,
        scene_entry,
        placement.as_ref(),
        Some(&heightmap),
        presentation,
    );
    let camera_elapsed = camera_start.elapsed();
    let sky_light_start = Instant::now();
    let camera_translation = camera::camera_params(scene_entry, placement.as_ref(), presentation).0;
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
    if let Some((entity, path)) = skybox_entity
        .zip(scene_entry.and_then(crate::scenes::char_select::warband::ensure_warband_skybox))
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
    let fov = camera::camera_params(scene_entry, placement.as_ref(), presentation).2;
    children.extend(scene_tree::light_scene_nodes(
        camera_entity,
        fov,
        None,
        lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS,
        dir,
    ));
    commands.insert_resource(scene_tree::build_scene_tree(children));
    pending_supplemental.scene_id = scene_entry
        .filter(|scene| {
            !crate::scenes::char_select::warband::supplemental_terrain_tile_coords(scene).is_empty()
        })
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
        (
            &mut Transform,
            &mut camera::CharSelectOrbit,
            &mut Projection,
        ),
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
        camera::update_camera_for_scene(
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

#[cfg(test)]
mod tests;
