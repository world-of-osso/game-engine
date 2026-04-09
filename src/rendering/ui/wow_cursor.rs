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
    pub interact: Handle<Image>,
    pub attack: Handle<Image>,
    pub quest: Handle<Image>,
    pub loot: Handle<Image>,
    pub mail: Handle<Image>,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveWowCursor {
    Default,
    Interact,
    Attack,
    Quest,
    Loot,
    Mail,
}

impl WowCursorAssets {
    fn handle_for(&self, cursor: ActiveWowCursor) -> Handle<Image> {
        match cursor {
            ActiveWowCursor::Default => self.default_point.clone(),
            ActiveWowCursor::Interact => self.interact.clone(),
            ActiveWowCursor::Attack => self.attack.clone(),
            ActiveWowCursor::Quest => self.quest.clone(),
            ActiveWowCursor::Loot => self.loot.clone(),
            ActiveWowCursor::Mail => self.mail.clone(),
        }
    }
}

fn load_cursor_image(images: &mut Assets<Image>, path: &str) -> Option<Handle<Image>> {
    use std::path::Path;
    let path = Path::new(path);
    let image = match asset::blp::load_blp_gpu_image(path) {
        Ok(image) => image,
        Err(error) => {
            warn!("failed to load WoW cursor {}: {error}", path.display());
            return None;
        }
    };
    Some(images.add(image))
}

fn load_cursor_assets(images: &mut Assets<Image>) -> Option<WowCursorAssets> {
    Some(WowCursorAssets {
        default_point: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Point.blp",
        )?,
        interact: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Interact.blp",
        )?,
        attack: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Attack.blp",
        )?,
        quest: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/QuestInteract.blp",
        )?,
        loot: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/LootAll.blp",
        )?,
        mail: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Mail.blp",
        )?,
    })
}

pub fn install_wow_cursor(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window_entity) = primary_window.single() else {
        return;
    };
    let Some(assets) = load_cursor_assets(&mut images) else {
        return;
    };
    let default_cursor = assets.default_point.clone();
    commands.insert_resource(assets);
    commands.insert_resource(ActiveWowCursor::Default);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: default_cursor,
            hotspot: (0, 0),
            ..default()
        })));
}

fn cursor_for_interaction(target: crate::target::InteractionTarget) -> ActiveWowCursor {
    match target {
        crate::target::InteractionTarget::Npc(_) => ActiveWowCursor::Attack,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Mailbox,
        ) => ActiveWowCursor::Mail,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::GatherNode(_),
        ) => ActiveWowCursor::Loot,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::QuestObject,
        ) => ActiveWowCursor::Quest,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Forge,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Anvil,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Chair,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::ZoneTransition,
        ) => ActiveWowCursor::Interact,
    }
}

/// Raycast to determine which cursor mode should be shown.
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
    for &(entity, _) in ray_cast.cast_ray(ray, &default()).iter() {
        if let Some(target) = crate::target::resolve_interaction_ancestor(
            entity,
            parent_query,
            npc_q,
            object_q,
            quest_q,
            visibility_q,
        ) {
            return Some(cursor_for_interaction(target));
        }
    }
    Some(ActiveWowCursor::Default)
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
    let handle = assets.handle_for(desired);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            hotspot: (0, 0),
            ..default()
        })));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{GatherNodeKind, InteractionTarget, WorldObjectInteractionKind};

    #[test]
    fn cursor_modes_map_npc_to_attack() {
        assert_eq!(
            cursor_for_interaction(InteractionTarget::Npc(Entity::PLACEHOLDER)),
            ActiveWowCursor::Attack
        );
    }

    #[test]
    fn cursor_modes_map_mailbox_to_mail() {
        assert_eq!(
            cursor_for_interaction(InteractionTarget::Object(
                Entity::PLACEHOLDER,
                WorldObjectInteractionKind::Mailbox,
            )),
            ActiveWowCursor::Mail
        );
    }

    #[test]
    fn cursor_modes_map_gather_node_to_loot() {
        assert_eq!(
            cursor_for_interaction(InteractionTarget::Object(
                Entity::PLACEHOLDER,
                WorldObjectInteractionKind::GatherNode(GatherNodeKind::CopperVein),
            )),
            ActiveWowCursor::Loot
        );
    }

    #[test]
    fn cursor_modes_map_quest_object_to_quest() {
        assert_eq!(
            cursor_for_interaction(InteractionTarget::Object(
                Entity::PLACEHOLDER,
                WorldObjectInteractionKind::QuestObject,
            )),
            ActiveWowCursor::Quest
        );
    }

    #[test]
    fn cursor_modes_map_generic_world_interactions_to_interact() {
        for kind in [
            WorldObjectInteractionKind::Forge,
            WorldObjectInteractionKind::Anvil,
            WorldObjectInteractionKind::Chair,
            WorldObjectInteractionKind::ZoneTransition,
        ] {
            assert_eq!(
                cursor_for_interaction(InteractionTarget::Object(Entity::PLACEHOLDER, kind)),
                ActiveWowCursor::Interact
            );
        }
    }
}
