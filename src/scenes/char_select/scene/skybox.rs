use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::creature_display;
use crate::scenes::char_select::scene::background;
use crate::scenes::char_select::scene::camera::CharSelectOrbit;
use crate::scenes::char_select::scene::{CharSelectRenderAssets, CharSelectSkybox};
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};

#[derive(SystemParam)]
pub(super) struct CharSelectSkyboxSyncParams<'w, 's> {
    commands: Commands<'w, 's>,
    assets: CharSelectRenderAssets<'w>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    warband: Option<Res<'w, WarbandScenes>>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    camera_query: Query<'w, 's, &'static Transform, With<CharSelectOrbit>>,
    skybox_query: Query<'w, 's, (Entity, &'static CharSelectSkybox, &'static Transform)>,
}

pub(super) fn sync_char_select_skybox(mut params: CharSelectSkyboxSyncParams) {
    let Ok(camera_transform) = params.camera_query.single() else {
        return;
    };
    let scene = background::find_scene_entry(&params.warband, &params.selected_scene);
    let desired_path = scene.and_then(crate::scenes::char_select::warband::ensure_warband_skybox);
    let current = params
        .skybox_query
        .iter()
        .next()
        .map(|(entity, skybox, transform)| (entity, skybox.path.clone(), transform.translation));

    if let Some((entity, current_path, _)) = &current
        && desired_path.as_ref() != Some(current_path)
    {
        params.commands.entity(*entity).despawn();
    }

    if current.as_ref().map(|(_, path, _)| path) != desired_path.as_ref() && desired_path.is_some()
    {
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
        background::spawn_skybox(&mut spawn_ctx, scene, camera_transform.translation);
        return;
    }

    for (entity, _, transform) in params.skybox_query.iter() {
        if transform.translation != camera_transform.translation {
            params
                .commands
                .entity(entity)
                .insert(Transform::from_translation(camera_transform.translation));
        }
    }
}
