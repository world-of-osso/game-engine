//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

mod scene_types;

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use game_engine::customization_data::{CustomizationDb, ModelPresentation};
use game_engine::scene_tree::SceneNode;
use shared::protocol::CharacterListEntry;

use crate::character_customization::{
    CharacterCustomizationSelection, apply_character_customization,
};
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display;
use crate::equipment_appearance::{apply_runtime_equipment, resolve_equipment_appearance};
use crate::game_state::GameState;
use crate::m2_scene;
use crate::networking_auth::CharacterList;
use crate::scenes::char_select::scene_tree::{self as scene_tree, ActiveWarbandSceneId};
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandSceneEntry, WarbandScenes};
use crate::scenes::setup::DEFAULT_M2;
use crate::terrain_heightmap::TerrainHeightmap;
use scene_types::{
    AppearanceSyncSelection, AppliedCharacterAppearance, CharSelectAppearanceSyncParams,
    CharSelectModelSyncParams, CharSelectRenderAssets, CharSelectSceneSetupParams,
    DisplayedCharacterAppearance, DisplayedCharacterId, ModelSyncSelection,
    PendingSupplementalWarbandScene, SceneSetupLighting, SceneSetupSelection, SceneSetupTimings,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelSyncDebugState {
    displayed_id: Option<u64>,
    desired_id: Option<u64>,
}

struct CharSelectModelSpawnContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    assets: &'a mut CharSelectRenderAssets<'w>,
    creature_display_map: &'a creature_display::CreatureDisplayMap,
}

pub struct CharSelectScenePlugin;

impl Plugin for CharSelectScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayedCharacterId>();
        app.init_resource::<DisplayedCharacterAppearance>();
        app.init_resource::<ActiveWarbandSceneId>();
        app.init_resource::<PendingSupplementalWarbandScene>();
        app.add_systems(
            OnEnter(GameState::CharSelect),
            setup::setup_char_select_scene,
        );
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
pub(crate) mod lighting;
mod scene_systems;
mod setup;
mod skybox;

fn spawn_char_select_model(
    ctx: &mut CharSelectModelSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    char_transform: Transform,
) -> Option<(Entity, Entity)> {
    let mut spawn_ctx = m2_scene::M2SceneSpawnContext {
        commands: ctx.commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: &mut ctx.assets.meshes,
            materials: &mut ctx.assets.materials,
            effect_materials: &mut ctx.assets.effect_materials,
            skybox_materials: None,
            images: &mut ctx.assets.images,
            inverse_bindposes: &mut ctx.assets.inv_bp,
        },
        creature_display_map: ctx.creature_display_map,
    };
    let spawned = m2_scene::spawn_animated_static_m2_parts(&mut spawn_ctx, m2_path, char_transform);
    let spawned = spawned?;
    ctx.commands
        .entity(spawned.root)
        .insert((CharSelectScene, CharSelectModelWrapper));
    ctx.commands
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

fn selected_scene_character_identity(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> (Option<String>, Option<u64>) {
    selected_scene_character(char_list, selected)
        .map(|character| (Some(character.name.clone()), Some(character.character_id)))
        .unwrap_or((None, None))
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

fn sync_char_select_model(mut params: CharSelectModelSyncParams) {
    let selection = resolve_model_sync_selection(&params);
    let debug_state = model_sync_debug_state(params.displayed.0, selection.desired_id);
    info!(
        displayed_id = ?debug_state.displayed_id,
        desired_id = ?debug_state.desired_id,
        should_respawn = debug_state.should_respawn(),
        "char-select model sync comparison"
    );
    if !debug_state.should_respawn() {
        return;
    }
    despawn_current_char_select_model(&mut params);
    let spawned_model = spawn_synced_char_select_model(&mut params, &selection);
    params.displayed.0 = spawned_model
        .as_ref()
        .map(|(character_id, _)| *character_id);
    sync_char_select_camera_after_model(&mut params, &selection);
    sync_scene_tree_character_identity(&mut params, spawned_model);
}

fn model_sync_debug_state(
    displayed_id: Option<u64>,
    desired_id: Option<u64>,
) -> ModelSyncDebugState {
    ModelSyncDebugState {
        displayed_id,
        desired_id,
    }
}

impl ModelSyncDebugState {
    fn should_respawn(self) -> bool {
        self.displayed_id != self.desired_id
    }
}

fn resolve_model_sync_selection(params: &CharSelectModelSyncParams<'_, '_>) -> ModelSyncSelection {
    let desired_id = selected_scene_character_id(&params.char_list, params.selected.0);
    let presentation = selected_character_presentation(
        &params.customization_db,
        &params.char_list,
        params.selected.0,
    );
    let scene_entry =
        background::find_scene_entry(&params.warband, &params.selected_scene).cloned();
    let placement = params
        .warband
        .as_ref()
        .zip(scene_entry.as_ref())
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let char_tf = resolve_char_transform(
        &params.warband,
        &params.selected_scene,
        Some(&params.heightmap),
        presentation,
    );
    ModelSyncSelection {
        desired_id,
        scene_entry,
        placement,
        presentation,
        char_tf,
    }
}

fn despawn_current_char_select_model(params: &mut CharSelectModelSyncParams<'_, '_>) {
    for entity in params.current_model.iter() {
        params.commands.entity(entity).despawn();
    }
}

fn spawn_synced_char_select_model(
    params: &mut CharSelectModelSyncParams<'_, '_>,
    selection: &ModelSyncSelection,
) -> Option<(u64, Entity)> {
    let mut spawn_ctx = CharSelectModelSpawnContext {
        commands: &mut params.commands,
        assets: &mut params.assets,
        creature_display_map: &params.creature_display_map,
    };
    spawn_selected_model(
        &mut spawn_ctx,
        &params.char_list,
        params.selected.0,
        selection.char_tf,
    )
}

fn sync_scene_tree_character_identity(
    params: &mut CharSelectModelSyncParams<'_, '_>,
    spawned_model: Option<(u64, Entity)>,
) {
    let Some(scene_tree) = params.scene_tree.as_deref_mut() else {
        return;
    };
    let Some((character_id, model_entity)) = spawned_model else {
        return;
    };
    let (race, gender, model) =
        scene_systems::char_info_strings(&params.char_list, params.selected.0);
    let (name, _) = selected_scene_character_identity(&params.char_list, params.selected.0);
    let character_node = scene_tree::character_scene_node(
        model_entity,
        model,
        race,
        gender,
        name,
        Some(character_id),
    );
    replace_scene_tree_character_node(scene_tree, character_node);
}

fn replace_scene_tree_character_node(
    scene_tree: &mut game_engine::scene_tree::SceneTree,
    node: SceneNode,
) {
    if let Some(existing) = scene_tree.root.children.iter_mut().find(|child| {
        matches!(
            child.props,
            game_engine::scene_tree::NodeProps::Character { .. }
        )
    }) {
        *existing = node;
        return;
    }
    let insert_at = scene_tree
        .root
        .children
        .iter()
        .position(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Background { .. }
            )
        })
        .map(|index| index + 1)
        .unwrap_or(0);
    scene_tree.root.children.insert(insert_at, node);
}

fn sync_char_select_camera_after_model(
    params: &mut CharSelectModelSyncParams<'_, '_>,
    selection: &ModelSyncSelection,
) {
    if let Some(scene) = selection.scene_entry.as_ref() {
        camera::update_camera_for_scene(
            scene,
            selection.placement.as_ref(),
            Some(&params.heightmap),
            selection.presentation,
            &mut params.camera_query,
        );
    }
}

fn sync_selected_character_appearance(mut params: CharSelectAppearanceSyncParams) {
    let Some(selection) = resolve_appearance_sync_selection(&mut params) else {
        params.displayed_appearance.0 = None;
        return;
    };
    if params.displayed_appearance.0 == Some(selection.desired.clone()) {
        return;
    }
    apply_selected_character_appearance(&mut params, &selection);
    params.displayed_appearance.0 = Some(selection.desired);
}

fn resolve_appearance_sync_selection(
    params: &mut CharSelectAppearanceSyncParams<'_, '_>,
) -> Option<AppearanceSyncSelection> {
    let character = selected_scene_character(&params.char_list, params.selected.0)?.clone();
    let (root, root_character) = params.root_query.single().ok()?;
    if root_character.0 != character.character_id {
        return None;
    }
    if !character_root_ready_for_appearance_sync(
        root,
        &params.parent_query,
        &params.geoset_query,
        &params.material_query,
    ) {
        return None;
    }
    Some(AppearanceSyncSelection {
        root,
        desired: desired_character_appearance(&character),
        character,
    })
}

fn character_root_ready_for_appearance_sync(
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::GeosetMesh>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
) -> bool {
    let has_geosets = geoset_query
        .iter()
        .any(|(entity, _, _)| is_descendant_of(entity, root, parent_query));
    let has_materials = material_query
        .iter()
        .any(|(entity, _, _, _, _)| is_descendant_of(entity, root, parent_query));
    has_geosets && has_materials
}

fn is_descendant_of(entity: Entity, root: Entity, parent_query: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    loop {
        let Ok(parent) = parent_query.get(current) else {
            return false;
        };
        let parent = parent.parent();
        if parent == root {
            return true;
        }
        current = parent;
    }
}

fn desired_character_appearance(character: &CharacterListEntry) -> AppliedCharacterAppearance {
    AppliedCharacterAppearance {
        character_id: character.character_id,
        race: character.race,
        class: character.class,
        appearance: character.appearance,
        equipment_appearance: character.equipment_appearance.clone(),
    }
}

fn apply_selected_character_appearance(
    params: &mut CharSelectAppearanceSyncParams<'_, '_>,
    selection: &AppearanceSyncSelection,
) {
    let resolved_equipment = resolve_equipment_appearance(
        &selection.character.equipment_appearance,
        &params.outfit_data,
        selection.character.race,
        selection.character.appearance.sex,
    );
    apply_character_customization(
        CharacterCustomizationSelection {
            race: selection.character.race,
            class: selection.character.class,
            sex: selection.character.appearance.sex,
            appearance: selection.character.appearance,
        },
        &params.customization_db,
        &params.char_tex,
        Some(&resolved_equipment),
        selection.root,
        &mut params.images,
        &mut params.materials,
        &params.parent_query,
        &params.geoset_query,
        &mut params.visibility_query,
        &params.equipment_item_query,
        &params.material_query,
    );
    if let Ok(mut equipment) = params.equipment_query.get_mut(selection.root) {
        apply_runtime_equipment(&mut equipment, &resolved_equipment);
    }
}

fn spawn_selected_model(
    ctx: &mut CharSelectModelSpawnContext<'_, '_, '_>,
    char_list: &CharacterList,
    selected: Option<usize>,
    char_transform: Transform,
) -> Option<(u64, Entity)> {
    let model_path = resolve_char_select_model_path(char_list, selected)?;
    if !model_path.exists() {
        return None;
    }
    let (_, model_entity) = spawn_char_select_model(ctx, &model_path, char_transform)?;
    let char_id = selected_scene_character_id(char_list, selected)?;
    ctx.commands
        .entity(model_entity)
        .insert(CharSelectModelCharacter(char_id));
    Some((char_id, model_entity))
}

#[cfg(test)]
mod tests;
