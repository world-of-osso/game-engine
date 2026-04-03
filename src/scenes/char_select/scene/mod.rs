//! 3D scene behind the character select screen.
//!
//! Spawns camera, lighting, warband terrain, and the selected character's M2 model.
//! All entities are tagged with [`CharSelectScene`] for bulk despawn on exit.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, ModelPresentation};
use game_engine::outfit_data::OutfitData;
use game_engine::scene_tree::SceneNode;
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
use crate::scenes::char_select::warband::{
    SelectedWarbandScene, WarbandSceneEntry, WarbandScenePlacement, WarbandScenes,
};
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

struct CharSelectModelSpawnContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    assets: &'a mut CharSelectRenderAssets<'w>,
    creature_display_map: &'a creature_display::CreatureDisplayMap,
}

struct SceneSetupSelection {
    scene_entry: Option<WarbandSceneEntry>,
    placement: Option<WarbandScenePlacement>,
    presentation: ModelPresentation,
}

struct ModelSyncSelection {
    desired_id: Option<u64>,
    scene_entry: Option<WarbandSceneEntry>,
    placement: Option<WarbandScenePlacement>,
    presentation: ModelPresentation,
    char_tf: Transform,
}

struct SceneSetupLighting {
    camera_entity: Entity,
    fov: f32,
    dir: Entity,
}

struct SceneSetupTimings {
    background_elapsed: std::time::Duration,
    camera_elapsed: std::time::Duration,
    sky_light_elapsed: std::time::Duration,
    model_elapsed: std::time::Duration,
}

#[derive(SystemParam)]
struct CharSelectSceneSetupParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    heightmap: ResMut<'w, TerrainHeightmap>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    customization_db: Res<'w, CustomizationDb>,
    char_list: Res<'w, CharacterList>,
    selected: Res<'w, SelectedCharIndex>,
    displayed: ResMut<'w, DisplayedCharacterId>,
    active_scene: ResMut<'w, ActiveWarbandSceneId>,
    pending_supplemental: ResMut<'w, PendingSupplementalWarbandScene>,
    warband: Option<Res<'w, WarbandScenes>>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
}

#[derive(SystemParam)]
struct CharSelectModelSyncParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    customization_db: Res<'w, CustomizationDb>,
    heightmap: Res<'w, TerrainHeightmap>,
    char_list: Res<'w, CharacterList>,
    selected: Res<'w, SelectedCharIndex>,
    current_model: Query<'w, 's, Entity, With<CharSelectModelWrapper>>,
    displayed: ResMut<'w, DisplayedCharacterId>,
    warband: Option<Res<'w, WarbandScenes>>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    camera_query: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut camera::CharSelectOrbit,
            &'static mut Projection,
        ),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
}

#[derive(SystemParam)]
struct CharSelectAppearanceSyncParams<'w, 's> {
    customization_db: Res<'w, CustomizationDb>,
    char_tex: Res<'w, CharTextureData>,
    outfit_data: Res<'w, OutfitData>,
    char_list: Res<'w, CharacterList>,
    selected: Res<'w, SelectedCharIndex>,
    displayed_appearance: ResMut<'w, DisplayedCharacterAppearance>,
    root_query:
        Query<'w, 's, (Entity, &'static CharSelectModelCharacter), With<CharSelectModelRoot>>,
    parent_query: Query<'w, 's, &'static ChildOf>,
    geoset_query: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::m2_spawn::GeosetMesh,
            &'static ChildOf,
        ),
    >,
    visibility_query: Query<'w, 's, &'static mut Visibility>,
    equipment_item_query: Query<'w, 's, (), With<EquipmentItem>>,
    material_query: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static crate::m2_spawn::GeosetMesh>,
            Option<&'static crate::m2_spawn::BatchTextureType>,
            &'static ChildOf,
        ),
    >,
    equipment_query: Query<'w, 's, &'static mut crate::equipment::Equipment>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
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

fn setup_char_select_scene(mut params: CharSelectSceneSetupParams) {
    let total_start = Instant::now();
    let selection = resolve_scene_setup_selection(&params);
    let (mut bg_node, background_elapsed) = spawn_scene_background(&mut params, &selection);
    let (lighting, camera_elapsed, sky_light_elapsed) =
        spawn_scene_camera_and_lighting(&mut params, &selection, &mut bg_node);
    let char_tf = resolve_char_transform(
        &params.warband,
        &params.selected_scene,
        Some(&params.heightmap),
        selection.presentation,
    );
    let (result, model_elapsed) = spawn_scene_model(&mut params, char_tf);
    finalize_scene_setup(&mut params, &selection, bg_node, lighting, result);
    let timings = SceneSetupTimings {
        background_elapsed,
        camera_elapsed,
        sky_light_elapsed,
        model_elapsed,
    };
    log_scene_setup_timings(total_start, timings);
}

fn resolve_scene_setup_selection(
    params: &CharSelectSceneSetupParams<'_, '_>,
) -> SceneSetupSelection {
    let scene_entry =
        background::find_scene_entry(&params.warband, &params.selected_scene).cloned();
    let placement = params
        .warband
        .as_ref()
        .zip(scene_entry.as_ref())
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let presentation = selected_character_presentation(
        &params.customization_db,
        &params.char_list,
        params.selected.0,
    );
    SceneSetupSelection {
        scene_entry,
        placement,
        presentation,
    }
}

fn spawn_scene_background(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
) -> (SceneNode, std::time::Duration) {
    let start = Instant::now();
    let mut background_ctx = background::WarbandBackgroundSpawnContext {
        commands: &mut params.commands,
        meshes: &mut params.assets.meshes,
        materials: &mut params.assets.materials,
        effect_materials: &mut params.assets.effect_materials,
        terrain_materials: &mut params.assets.terrain_materials,
        water_materials: &mut params.assets.water_materials,
        images: &mut params.assets.images,
        inv_bp: &mut params.assets.inv_bp,
        heightmap: &mut params.heightmap,
    };
    (
        background::spawn(
            &mut background_ctx,
            selection.scene_entry.as_ref(),
            selection
                .placement
                .as_ref()
                .map(|placement| placement.bevy_position()),
            &mut params.active_scene,
        ),
        start.elapsed(),
    )
}

fn spawn_scene_camera_and_lighting(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
    bg_node: &mut SceneNode,
) -> (SceneSetupLighting, std::time::Duration, std::time::Duration) {
    let camera_start = Instant::now();
    let camera_entity = camera::spawn_char_select_camera(
        &mut params.commands,
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        Some(&params.heightmap),
        selection.presentation,
    );
    let camera_elapsed = camera_start.elapsed();
    let sky_light_start = Instant::now();
    let camera_params = camera::camera_params(
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        selection.presentation,
    );
    attach_scene_skybox(
        params,
        selection.scene_entry.as_ref(),
        camera_params.0,
        bg_node,
    );
    let dir = lighting::spawn(
        &mut params.commands,
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        selection.presentation,
    );
    (
        SceneSetupLighting {
            camera_entity,
            fov: camera_params.2,
            dir,
        },
        camera_elapsed,
        sky_light_start.elapsed(),
    )
}

fn attach_scene_skybox(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    scene_entry: Option<&WarbandSceneEntry>,
    camera_translation: Vec3,
    bg_node: &mut SceneNode,
) {
    let skybox_entity = {
        let mut skybox_ctx = background::WarbandSkyboxSpawnContext {
            commands: &mut params.commands,
            meshes: &mut params.assets.meshes,
            materials: &mut params.assets.materials,
            effect_materials: &mut params.assets.effect_materials,
            skybox_materials: &mut params.assets.skybox_materials,
            images: &mut params.assets.images,
            inv_bp: &mut params.assets.inv_bp,
            creature_display_map: &params.creature_display_map,
        };
        background::spawn_skybox(&mut skybox_ctx, scene_entry, camera_translation)
    };
    if let Some((entity, path)) = skybox_entity
        .zip(scene_entry.and_then(crate::scenes::char_select::warband::ensure_warband_skybox))
    {
        bg_node.children.push(scene_tree::skybox_scene_node(
            entity,
            path.display().to_string(),
        ));
    }
}

fn spawn_scene_model(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    char_tf: Transform,
) -> (Option<(u64, Entity)>, std::time::Duration) {
    let start = Instant::now();
    let mut spawn_ctx = CharSelectModelSpawnContext {
        commands: &mut params.commands,
        assets: &mut params.assets,
        creature_display_map: &params.creature_display_map,
    };
    (
        spawn_selected_model(
            &mut spawn_ctx,
            &params.char_list,
            params.selected.0,
            char_tf,
        ),
        start.elapsed(),
    )
}

fn finalize_scene_setup(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
    bg_node: SceneNode,
    lighting: SceneSetupLighting,
    result: Option<(u64, Entity)>,
) {
    params.displayed.0 = result.as_ref().map(|(id, _)| *id);
    let children = build_scene_setup_children(
        params,
        bg_node,
        &lighting,
        result.as_ref().map(|(_, entity)| *entity),
    );
    params
        .commands
        .insert_resource(scene_tree::build_scene_tree(children));
    params.pending_supplemental.scene_id = selection
        .scene_entry
        .as_ref()
        .filter(|scene| {
            !crate::scenes::char_select::warband::supplemental_terrain_tile_coords(scene).is_empty()
        })
        .map(|scene| scene.id);
    params.pending_supplemental.wait_for_next_frame =
        params.pending_supplemental.scene_id.is_some();
}

fn build_scene_setup_children(
    params: &CharSelectSceneSetupParams<'_, '_>,
    bg_node: SceneNode,
    lighting: &SceneSetupLighting,
    model_entity: Option<Entity>,
) -> Vec<SceneNode> {
    let mut children = vec![bg_node];
    if let Some(entity) = model_entity {
        let (race, gender, model) =
            scene_systems::char_info_strings(&params.char_list, params.selected.0);
        children.push(scene_tree::character_scene_node(
            entity, model, race, gender,
        ));
    }
    children.extend(scene_tree::light_scene_nodes(
        lighting.camera_entity,
        lighting.fov,
        None,
        lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS,
        lighting.dir,
    ));
    children
}

fn log_scene_setup_timings(total_start: Instant, timings: SceneSetupTimings) {
    info!(
        "setup_char_select_scene finished in {:.3}s (background={:.3}s camera={:.3}s sky+light={:.3}s model={:.3}s)",
        total_start.elapsed().as_secs_f32(),
        timings.background_elapsed.as_secs_f32(),
        timings.camera_elapsed.as_secs_f32(),
        timings.sky_light_elapsed.as_secs_f32(),
        timings.model_elapsed.as_secs_f32(),
    );
}

fn sync_char_select_model(mut params: CharSelectModelSyncParams) {
    let selection = resolve_model_sync_selection(&params);
    if params.displayed.0 == selection.desired_id {
        return;
    }
    despawn_current_char_select_model(&mut params);
    params.displayed.0 = spawn_synced_char_select_model(&mut params, &selection);
    sync_char_select_camera_after_model(&mut params, &selection);
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
) -> Option<u64> {
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
    .map(|(id, _)| id)
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
    let Some(character) = selected_scene_character(&params.char_list, params.selected.0) else {
        params.displayed_appearance.0 = None;
        return;
    };
    let Ok((root, root_character)) = params.root_query.single() else {
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
    if params.displayed_appearance.0 == Some(desired.clone()) {
        return;
    }
    let resolved_equipment = resolve_equipment_appearance(
        &character.equipment_appearance,
        &params.outfit_data,
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
        &params.customization_db,
        &params.char_tex,
        Some(&resolved_equipment),
        root,
        &mut params.images,
        &mut params.materials,
        &params.parent_query,
        &params.geoset_query,
        &mut params.visibility_query,
        &params.equipment_item_query,
        &params.material_query,
    );
    if let Ok(mut equipment) = params.equipment_query.get_mut(root) {
        apply_runtime_equipment(&mut equipment, &resolved_equipment);
    }
    params.displayed_appearance.0 = Some(desired);
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
