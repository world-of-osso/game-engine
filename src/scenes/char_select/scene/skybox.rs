use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::path::PathBuf;

use crate::creature_display;
use crate::scenes::char_select::scene::CharSelectSkybox;
use crate::scenes::char_select::scene::background;
use crate::scenes::char_select::scene::camera::CharSelectOrbit;
use crate::scenes::char_select::scene::scene_types::CharSelectRenderAssets;
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};

#[derive(SystemParam)]
pub(super) struct CharSelectSkyboxSyncParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    warband: Option<Res<'w, WarbandScenes>>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    camera_query: Query<'w, 's, &'static CharSelectOrbit>,
    skybox_query: Query<'w, 's, (Entity, &'static CharSelectSkybox, &'static Transform)>,
}

pub(super) fn sync_char_select_skybox(mut params: CharSelectSkyboxSyncParams) {
    let Ok(skybox_translation) = params.camera_query.single().map(|orbit| orbit.focus) else {
        return;
    };
    let desired_path = background::find_scene_entry(&params.warband, &params.selected_scene)
        .and_then(crate::scenes::char_select::warband::ensure_warband_skybox);
    let current = current_skybox_state(&params);

    despawn_replaced_skybox(&mut params, &current, desired_path.as_ref());

    if should_spawn_skybox(&current, desired_path.as_ref()) {
        let scene = background::find_scene_entry(&params.warband, &params.selected_scene);
        let mut spawn_ctx = background::WarbandSkyboxSpawnContext {
            commands: &mut params.commands,
            meshes: &mut params.assets.meshes,
            materials: &mut params.assets.materials,
            effect_materials: &mut params.assets.effect_materials,
            skybox_materials: &mut params.assets.skybox_materials,
            images: &mut params.assets.images,
            inv_bp: &mut params.assets.inv_bp,
            creature_display_map: &params.creature_display_map,
        };
        background::spawn_skybox(&mut spawn_ctx, scene, skybox_translation);
        return;
    }

    sync_existing_skybox_translation(&mut params, skybox_translation);
}

fn current_skybox_state(
    params: &CharSelectSkyboxSyncParams<'_, '_>,
) -> Option<(Entity, PathBuf, Vec3)> {
    params
        .skybox_query
        .iter()
        .next()
        .map(|(entity, skybox, transform)| (entity, skybox.path.clone(), transform.translation))
}

fn despawn_replaced_skybox(
    params: &mut CharSelectSkyboxSyncParams<'_, '_>,
    current: &Option<(Entity, PathBuf, Vec3)>,
    desired_path: Option<&PathBuf>,
) {
    if let Some((entity, current_path, _)) = current
        && desired_path != Some(current_path)
    {
        params.commands.entity(*entity).despawn();
    }
}

fn should_spawn_skybox(
    current: &Option<(Entity, PathBuf, Vec3)>,
    desired_path: Option<&PathBuf>,
) -> bool {
    current.as_ref().map(|(_, path, _)| path) != desired_path && desired_path.is_some()
}

fn sync_existing_skybox_translation(
    params: &mut CharSelectSkyboxSyncParams<'_, '_>,
    skybox_translation: Vec3,
) {
    for (entity, _, transform) in params.skybox_query.iter() {
        if transform.translation != skybox_translation {
            params
                .commands
                .entity(entity)
                .insert(Transform::from_translation(skybox_translation));
        }
    }
}
