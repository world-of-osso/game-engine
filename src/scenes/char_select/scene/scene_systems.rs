//! Warband scene switch, supplemental terrain, teardown, and utility systems.

use bevy::prelude::*;
use game_engine::customization_data::CustomizationDb;

use crate::networking_auth::CharacterList;
use crate::scenes::char_select::SelectedCharIndex;
use crate::scenes::char_select::scene_tree::{
    self as scene_tree, ActiveWarbandSceneId, CharSelectTerrain,
};
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandSceneEntry, WarbandScenes};
use crate::terrain_heightmap::TerrainHeightmap;

use super::{
    CharSelectModelRoot, CharSelectOrbit, CharSelectRenderAssets, CharSelectScene,
    DisplayedCharacterAppearance, DisplayedCharacterId, PendingSupplementalWarbandScene,
    resolve_char_select_model_path, selected_character_presentation, selected_scene_character,
    selected_scene_placement, update_camera_for_scene,
};

fn spawn_scene_warband_terrain(
    commands: &mut Commands,
    assets: &mut CharSelectRenderAssets,
    heightmap: &mut TerrainHeightmap,
    scene: &WarbandSceneEntry,
    focus_pos: Vec3,
) {
    let _ = scene_tree::spawn_warband_terrain(
        commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.terrain_materials,
        &mut assets.water_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        heightmap,
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

#[allow(clippy::too_many_arguments)]
pub(super) fn sync_warband_scene_switch(
    mut commands: Commands,
    mut assets: CharSelectRenderAssets,
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
    mut pending_supplemental: ResMut<PendingSupplementalWarbandScene>,
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
    let focus_pos = placement
        .as_ref()
        .map(|p| p.bevy_position())
        .unwrap_or_else(|| scene.bevy_look_at());
    spawn_scene_warband_terrain(&mut commands, &mut assets, &mut heightmap, scene, focus_pos);
    update_camera_for_scene(
        scene,
        placement.as_ref(),
        Some(&heightmap),
        presentation,
        &mut camera_query,
    );
    active_scene.0 = Some(sel.scene_id);
    let has_supplemental =
        !crate::scenes::char_select::warband::supplemental_terrain_tile_coords(scene).is_empty();
    update_pending_scene(&mut pending_supplemental, sel.scene_id, has_supplemental);
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
        commands,
        &mut assets.meshes,
        &mut assets.materials,
        &mut assets.effect_materials,
        &mut assets.terrain_materials,
        &mut assets.water_materials,
        &mut assets.images,
        &mut assets.inv_bp,
        heightmap,
        scene,
        root_entity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_pending_warband_supplemental_terrain(
    mut commands: Commands,
    mut assets: CharSelectRenderAssets,
    mut pending: ResMut<PendingSupplementalWarbandScene>,
    active_scene: Res<ActiveWarbandSceneId>,
    mut heightmap: ResMut<TerrainHeightmap>,
    warband: Option<Res<WarbandScenes>>,
    terrain_query: Query<Entity, With<CharSelectTerrain>>,
) {
    let Some(scene_id) = pending.scene_id else {
        return;
    };
    if pending.wait_for_next_frame {
        pending.wait_for_next_frame = false;
        return;
    }
    if is_pending_scene_valid(&pending, &active_scene).is_none() {
        pending.scene_id = None;
        pending.wait_for_next_frame = false;
        return;
    }
    let Some(warband) = warband else { return };
    let Some(scene) = warband.scenes.iter().find(|scene| scene.id == scene_id) else {
        pending.scene_id = None;
        pending.wait_for_next_frame = false;
        return;
    };
    let Ok(root_entity) = terrain_query.single() else {
        return;
    };
    do_spawn_supplemental(
        &mut commands,
        &mut assets,
        &mut heightmap,
        scene,
        root_entity,
    );
    pending.scene_id = None;
    pending.wait_for_next_frame = false;
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
