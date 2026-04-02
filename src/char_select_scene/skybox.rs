use bevy::prelude::*;

use crate::char_select_scene::background;
use crate::char_select_scene::{CharSelectOrbit, CharSelectRenderAssets, CharSelectSkybox};
use crate::creature_display;
use crate::warband_scene::{SelectedWarbandScene, WarbandScenes};

#[allow(clippy::too_many_arguments)]
pub(super) fn sync_char_select_skybox(
    mut commands: Commands,
    mut assets: CharSelectRenderAssets,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    warband: Option<Res<WarbandScenes>>,
    selected_scene: Option<Res<SelectedWarbandScene>>,
    camera_query: Query<&Transform, With<CharSelectOrbit>>,
    skybox_query: Query<(Entity, &CharSelectSkybox, &Transform)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let scene = background::find_scene_entry(&warband, &selected_scene);
    let desired_path = scene.and_then(crate::warband_scene::ensure_warband_skybox);
    let current = skybox_query
        .iter()
        .next()
        .map(|(entity, skybox, transform)| (entity, skybox.path.clone(), transform.translation));

    if let Some((entity, current_path, _)) = &current
        && desired_path.as_ref() != Some(current_path)
    {
        commands.entity(*entity).despawn();
    }

    if current.as_ref().map(|(_, path, _)| path) != desired_path.as_ref() && desired_path.is_some()
    {
        background::spawn_skybox(
            &mut commands,
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.effect_materials,
            &mut assets.images,
            &mut assets.inv_bp,
            &creature_display_map,
            scene,
            camera_transform.translation,
        );
        return;
    }

    for (entity, _, transform) in skybox_query.iter() {
        if transform.translation != camera_transform.translation {
            commands
                .entity(entity)
                .insert(Transform::from_translation(camera_transform.translation));
        }
    }
}
