//! Warband scene switch, supplemental terrain, teardown, and utility systems.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::customization_data::CustomizationDb;

use crate::networking_auth::CharacterList;
use crate::scenes::char_select::SelectedCharIndex;
use crate::scenes::char_select::scene_tree::{
    self as scene_tree, ActiveWarbandSceneId, CharSelectTerrain,
};
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandSceneEntry, WarbandScenes};
use crate::terrain_heightmap::TerrainHeightmap;

use super::camera::{CharSelectOrbit, update_camera_for_scene};
use super::scene_types::{
    CharSelectRenderAssets, DisplayedCharacterAppearance, DisplayedCharacterId,
    PendingSupplementalWarbandScene,
};
use super::{
    CharSelectModelRoot, CharSelectScene, ModelPresentation, resolve_char_select_model_path,
    selected_character_presentation, selected_scene_character, selected_scene_placement,
};

fn spawn_scene_warband_terrain(
    commands: &mut Commands,
    assets: &mut CharSelectRenderAssets,
    heightmap: &mut TerrainHeightmap,
    scene: &WarbandSceneEntry,
    focus_pos: Vec3,
) {
    let _ = scene_tree::spawn_warband_terrain(
        &mut scene_tree::WarbandTerrainSpawnContext {
            commands,
            meshes: &mut assets.meshes,
            materials: &mut assets.materials,
            effect_materials: &mut assets.effect_materials,
            terrain_materials: &mut assets.terrain_materials,
            water_materials: &mut assets.water_materials,
            images: &mut assets.images,
            inv_bp: &mut assets.inv_bp,
            heightmap,
        },
        scene,
        focus_pos,
    );
}

fn update_pending_scene(
    pending: &mut PendingSupplementalWarbandScene,
    scene_id: u32,
    has_supplemental: bool,
) {
    pending.scene_id = if has_supplemental {
        Some(scene_id)
    } else {
        None
    };
    pending.wait_for_next_frame = pending.scene_id.is_some();
}

#[derive(SystemParam)]
pub(super) struct CharSelectSceneSwitchParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    active_scene: ResMut<'w, ActiveWarbandSceneId>,
    heightmap: ResMut<'w, TerrainHeightmap>,
    customization_db: Res<'w, CustomizationDb>,
    char_list: Res<'w, CharacterList>,
    selected: Res<'w, SelectedCharIndex>,
    warband: Option<Res<'w, WarbandScenes>>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    terrain_query: Query<'w, 's, Entity, With<CharSelectTerrain>>,
    camera_query: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut CharSelectOrbit,
            &'static mut Projection,
        ),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
    pending_supplemental: ResMut<'w, PendingSupplementalWarbandScene>,
}

#[derive(SystemParam)]
pub(super) struct CharSelectSupplementalTerrainParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    pending: ResMut<'w, PendingSupplementalWarbandScene>,
    active_scene: Res<'w, ActiveWarbandSceneId>,
    heightmap: ResMut<'w, TerrainHeightmap>,
    warband: Option<Res<'w, WarbandScenes>>,
    terrain_query: Query<'w, 's, Entity, With<CharSelectTerrain>>,
}

pub(super) fn sync_warband_scene_switch(mut params: CharSelectSceneSwitchParams) {
    let Some(scene_switch) = resolve_scene_switch(&params) else {
        return;
    };
    for entity in params.terrain_query.iter() {
        params.commands.entity(entity).despawn();
    }
    let focus_pos = scene_switch
        .placement
        .as_ref()
        .map(|p| p.bevy_position())
        .unwrap_or_else(|| scene_switch.scene.bevy_look_at());
    spawn_scene_warband_terrain(
        &mut params.commands,
        &mut params.assets,
        &mut params.heightmap,
        &scene_switch.scene,
        focus_pos,
    );
    update_camera_for_scene(
        &scene_switch.scene,
        scene_switch.placement.as_ref(),
        Some(&params.heightmap),
        scene_switch.presentation,
        &mut params.camera_query,
    );
    params.active_scene.0 = Some(scene_switch.scene_id);
    let has_supplemental =
        !crate::scenes::char_select::warband::supplemental_terrain_tile_coords(&scene_switch.scene)
            .is_empty();
    update_pending_scene(
        &mut params.pending_supplemental,
        scene_switch.scene_id,
        has_supplemental,
    );
}

struct SceneSwitchData {
    scene_id: u32,
    scene: WarbandSceneEntry,
    placement: Option<crate::scenes::char_select::warband::WarbandScenePlacement>,
    presentation: ModelPresentation,
}

fn resolve_scene_switch(params: &CharSelectSceneSwitchParams<'_, '_>) -> Option<SceneSwitchData> {
    let warband = params.warband.as_ref()?;
    let sel = params.selected_scene.as_ref()?;
    if params.active_scene.0 == Some(sel.scene_id) {
        return None;
    }
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == sel.scene_id)?;
    let placement = selected_scene_placement(warband, scene);
    let presentation = selected_character_presentation(
        &params.customization_db,
        &params.char_list,
        params.selected.0,
    );
    Some(SceneSwitchData {
        scene_id: sel.scene_id,
        scene: scene.clone(),
        placement,
        presentation,
    })
}

fn is_pending_scene_valid(
    pending: &PendingSupplementalWarbandScene,
    active_scene: &ActiveWarbandSceneId,
) -> Option<u32> {
    let scene_id = pending.scene_id?;
    if active_scene.0 != Some(scene_id) {
        return None;
    }
    Some(scene_id)
}

fn do_spawn_supplemental(
    commands: &mut Commands,
    assets: &mut CharSelectRenderAssets,
    heightmap: &mut TerrainHeightmap,
    scene: &WarbandSceneEntry,
    root_entity: Entity,
) {
    scene_tree::spawn_warband_supplemental_terrain(
        &mut scene_tree::WarbandTerrainSpawnContext {
            commands,
            meshes: &mut assets.meshes,
            materials: &mut assets.materials,
            effect_materials: &mut assets.effect_materials,
            terrain_materials: &mut assets.terrain_materials,
            water_materials: &mut assets.water_materials,
            images: &mut assets.images,
            inv_bp: &mut assets.inv_bp,
            heightmap,
        },
        scene,
        root_entity,
    );
}

pub(super) fn spawn_pending_warband_supplemental_terrain(
    mut params: CharSelectSupplementalTerrainParams,
) {
    let Some(scene_id) = params.pending.scene_id else {
        return;
    };
    if params.pending.wait_for_next_frame {
        params.pending.wait_for_next_frame = false;
        return;
    }
    if is_pending_scene_valid(&params.pending, &params.active_scene).is_none() {
        params.pending.scene_id = None;
        params.pending.wait_for_next_frame = false;
        return;
    }
    let Some(warband) = params.warband.as_ref() else {
        return;
    };
    let Some(scene) = warband.scenes.iter().find(|scene| scene.id == scene_id) else {
        params.pending.scene_id = None;
        params.pending.wait_for_next_frame = false;
        return;
    };
    let Ok(root_entity) = params.terrain_query.single() else {
        return;
    };
    do_spawn_supplemental(
        &mut params.commands,
        &mut params.assets,
        &mut params.heightmap,
        scene,
        root_entity,
    );
    params.pending.scene_id = None;
    params.pending.wait_for_next_frame = false;
}

pub(super) fn teardown_char_select_scene(
    mut commands: Commands,
    query: Query<Entity, With<CharSelectScene>>,
    mut displayed: ResMut<DisplayedCharacterId>,
    mut displayed_appearance: ResMut<DisplayedCharacterAppearance>,
    mut active_scene: ResMut<ActiveWarbandSceneId>,
    mut pending_supplemental: ResMut<PendingSupplementalWarbandScene>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    displayed.0 = None;
    displayed_appearance.0 = None;
    active_scene.0 = None;
    pending_supplemental.scene_id = None;
    pending_supplemental.wait_for_next_frame = false;
    commands.remove_resource::<game_engine::scene_tree::SceneTree>();
}

pub(super) fn char_info_strings(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> (String, String, String) {
    use crate::character_models::race_name;
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
