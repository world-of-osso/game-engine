use bevy::prelude::*;

use crate::camera::Player;
use crate::game_state::GameState;

use super::{WorldObjectInteraction, WorldObjectInteractionKind};

#[derive(Resource, Default)]
pub(super) struct ZoneTransitionContactState {
    pub active_portal: Option<Entity>,
}

pub(super) fn reset_zone_transition_contact(mut contact: ResMut<ZoneTransitionContactState>) {
    contact.active_portal = None;
}

pub(super) fn trigger_zone_transition_on_collision(
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    portal_q: Query<(
        Entity,
        &WorldObjectInteraction,
        &GlobalTransform,
        Option<&Visibility>,
        Option<&game_engine::culling::DoodadCollider>,
        Option<&game_engine::culling::WmoGroup>,
    )>,
    mut contact: ResMut<ZoneTransitionContactState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    let Ok(player_transform) = player_q.single() else {
        contact.active_portal = None;
        return;
    };

    let player_position = player_transform.translation();
    let colliding_portal = nearest_colliding_zone_transition(player_position, &portal_q);
    if update_zone_transition_contact(&mut contact.active_portal, colliding_portal) {
        info!("Entered zone transition portal, entering Loading");
        next_state.set(GameState::Loading);
    }
}

fn nearest_colliding_zone_transition(
    player_position: Vec3,
    portal_q: &Query<(
        Entity,
        &WorldObjectInteraction,
        &GlobalTransform,
        Option<&Visibility>,
        Option<&game_engine::culling::DoodadCollider>,
        Option<&game_engine::culling::WmoGroup>,
    )>,
) -> Option<Entity> {
    let mut best_portal = None;

    for (entity, interaction, transform, visibility, doodad_collider, wmo_group) in portal_q.iter()
    {
        if interaction.kind != WorldObjectInteractionKind::ZoneTransition {
            continue;
        }
        if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden) {
            continue;
        }
        if !player_inside_zone_transition(player_position, transform, doodad_collider, wmo_group) {
            continue;
        }

        let distance_sq =
            zone_transition_distance_sq(player_position, transform, doodad_collider, wmo_group);
        match best_portal {
            Some((best_distance_sq, _)) if distance_sq >= best_distance_sq => {}
            _ => best_portal = Some((distance_sq, entity)),
        }
    }

    best_portal.map(|(_, entity)| entity)
}

pub(super) fn player_inside_zone_transition(
    player_position: Vec3,
    transform: &GlobalTransform,
    doodad_collider: Option<&game_engine::culling::DoodadCollider>,
    wmo_group: Option<&game_engine::culling::WmoGroup>,
) -> bool {
    if let Some(collider) = doodad_collider {
        return point_inside_aabb(player_position, collider.world_min, collider.world_max);
    }
    let Some(group) = wmo_group else {
        return false;
    };
    let local_player = transform
        .affine()
        .inverse()
        .transform_point3(player_position);
    point_inside_wmo_group_bounds(local_player, group)
}

fn point_inside_aabb(point: Vec3, min: Vec3, max: Vec3) -> bool {
    point.x >= min.x
        && point.y >= min.y
        && point.z >= min.z
        && point.x <= max.x
        && point.y <= max.y
        && point.z <= max.z
}

fn point_inside_wmo_group_bounds(
    local_point: Vec3,
    group: &game_engine::culling::WmoGroup,
) -> bool {
    local_point.x >= group.bbox_min.x
        && local_point.y >= group.bbox_min.y
        && local_point.z >= group.bbox_min.z
        && local_point.x <= group.bbox_max.x
        && local_point.y <= group.bbox_max.y
        && local_point.z <= group.bbox_max.z
}

fn zone_transition_distance_sq(
    player_position: Vec3,
    transform: &GlobalTransform,
    doodad_collider: Option<&game_engine::culling::DoodadCollider>,
    wmo_group: Option<&game_engine::culling::WmoGroup>,
) -> f32 {
    let center = if let Some(collider) = doodad_collider {
        (collider.world_min + collider.world_max) * 0.5
    } else if let Some(group) = wmo_group {
        transform
            .affine()
            .transform_point3((group.bbox_min + group.bbox_max) * 0.5)
    } else {
        transform.translation()
    };
    center.distance_squared(player_position)
}

pub(super) fn update_zone_transition_contact(
    active_portal: &mut Option<Entity>,
    colliding_portal: Option<Entity>,
) -> bool {
    let entered_new_portal = colliding_portal.is_some() && *active_portal != colliding_portal;
    *active_portal = colliding_portal;
    entered_new_portal
}
