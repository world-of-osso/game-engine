use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::{CursorIcon, CursorOptions, CustomCursor, CustomCursorImage, PrimaryWindow};
use game_engine::quest_tracking::QuestTrackedItem;
use shared::components::Npc;

use crate::asset;
use crate::camera::{Player, WowCamera};
use crate::networking::RemoteEntity;
use crate::target::WorldObjectInteraction;

#[derive(Resource)]
pub struct WowCursorAssets {
    pub default_point: Handle<Image>,
    pub hover_point: Handle<Image>,
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWowCursor {
    Default,
    Hover,
}

fn load_cursor_images(images: &mut Assets<Image>) -> Option<(Handle<Image>, Handle<Image>)> {
    use std::path::Path;
    let default_path = Path::new("/syncthing/Sync/Projects/wow/Interface/CURSOR/Point.blp");
    let hover_path = Path::new("/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Point.blp");
    let default_image = match asset::blp::load_blp_gpu_image(default_path) {
        Ok(image) => image,
        Err(error) => {
            warn!(
                "failed to load WoW cursor {}: {error}",
                default_path.display()
            );
            return None;
        }
    };
    let hover_image = match asset::blp::load_blp_gpu_image(hover_path) {
        Ok(image) => image,
        Err(error) => {
            warn!(
                "failed to load WoW cursor {}: {error}",
                hover_path.display()
            );
            return None;
        }
    };
    Some((images.add(default_image), images.add(hover_image)))
}

pub fn install_wow_cursor(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window_entity) = primary_window.single() else {
        return;
    };
    let Some((default_cursor, hover_cursor)) = load_cursor_images(&mut images) else {
        return;
    };
    commands.insert_resource(WowCursorAssets {
        default_point: default_cursor.clone(),
        hover_point: hover_cursor,
    });
    commands.insert_resource(ActiveWowCursor::Default);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: default_cursor,
            hotspot: (0, 0),
            ..default()
        })));
}

/// Raycast to determine whether the cursor hovers over a remote entity.
fn pick_desired_cursor(
    window: &Window,
    camera: (&Camera, &GlobalTransform),
    parent_query: &Query<&ChildOf>,
    npc_q: &Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    object_q: &Query<&WorldObjectInteraction>,
    quest_q: &Query<(), With<QuestTrackedItem>>,
    visibility_q: &Query<&Visibility>,
    ray_cast: &mut MeshRayCast,
) -> Option<ActiveWowCursor> {
    let cursor = window.cursor_position()?;
    let (cam, cam_tf) = camera;
    let ray = cam.viewport_to_world(cam_tf, cursor).ok()?;
    let hover = ray_cast
        .cast_ray(ray, &default())
        .iter()
        .any(|(entity, _)| {
            crate::target::resolve_interaction_ancestor(
                *entity,
                parent_query,
                npc_q,
                object_q,
                quest_q,
                visibility_q,
            )
            .is_some()
        });
    Some(if hover {
        ActiveWowCursor::Hover
    } else {
        ActiveWowCursor::Default
    })
}

pub fn update_wow_cursor_style(
    windows: Query<(&Window, &CursorOptions, Entity), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WowCamera>>,
    parent_query: Query<&ChildOf>,
    npc_q: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    object_q: Query<&WorldObjectInteraction>,
    quest_q: Query<(), With<QuestTrackedItem>>,
    visibility_q: Query<&Visibility>,
    assets: Option<Res<WowCursorAssets>>,
    active: Option<ResMut<ActiveWowCursor>>,
    mut ray_cast: MeshRayCast,
    mut commands: Commands,
) {
    let (window, cursor_opts, window_entity) = match windows.single() {
        Ok(value) => value,
        Err(_) => return,
    };
    if !cursor_opts.visible {
        return;
    }
    let Ok(camera) = cameras.single() else { return };
    let Some(assets) = assets else { return };
    let Some(mut active) = active else { return };

    let desired = pick_desired_cursor(
        window,
        camera,
        &parent_query,
        &npc_q,
        &object_q,
        &quest_q,
        &visibility_q,
        &mut ray_cast,
    )
    .unwrap_or(ActiveWowCursor::Default);
    if *active == desired {
        return;
    }
    *active = desired;
    let handle = match desired {
        ActiveWowCursor::Default => assets.default_point.clone(),
        ActiveWowCursor::Hover => assets.hover_point.clone(),
    };
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            hotspot: (0, 0),
            ..default()
        })));
}
