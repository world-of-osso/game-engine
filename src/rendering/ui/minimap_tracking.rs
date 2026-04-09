use bevy::prelude::*;

use crate::minimap_render::{TrackingIconKind, draw_tracking_icon};
use crate::target::{WorldObjectInteraction, WorldObjectInteractionKind};
use game_engine::{
    minimap_data::{MinimapHerbNode, MinimapUIState, TrackingType},
    quest_data::QuestLogState,
    quest_tracking::{QuestTrackedItem, should_sparkle},
};

use super::{MAX_TRACKING_ICONS, MINIMAP_COMPOSITE_SIZE, world_pixel_in_composite};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TrackingPoint {
    pub(super) kind: TrackingIconKind,
    pub(super) bx: f32,
    pub(super) bz: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrackingFilter {
    show_herbs: bool,
    show_minerals: bool,
}

pub(super) fn collect_tracking_points(
    tracking_state: Option<&MinimapUIState>,
    world_object_q: &Query<(
        &GlobalTransform,
        &WorldObjectInteraction,
        Option<&Visibility>,
    )>,
    herb_q: &Query<(&GlobalTransform, Option<&Visibility>), With<MinimapHerbNode>>,
    quest_q: &Query<(&GlobalTransform, &QuestTrackedItem, Option<&Visibility>)>,
    quest_log: Option<&QuestLogState>,
) -> Vec<TrackingPoint> {
    limit_tracking_points(gather_tracking_points(
        tracking_state,
        world_object_q,
        herb_q,
        quest_q,
        quest_log,
    ))
}

pub(super) fn draw_tracking_icons(
    data: &mut [u8],
    ds: usize,
    px_x: usize,
    px_y: usize,
    player_row: u32,
    player_col: u32,
    tracking_points: &[TrackingPoint],
    mask: &[bool],
) {
    let half = ds as i32 / 2;
    for point in tracking_points {
        let (icon_px_x, icon_px_y) = world_pixel_in_composite(
            point.bx,
            point.bz,
            player_row,
            player_col,
            MINIMAP_COMPOSITE_SIZE as usize,
        );
        let crop_x = icon_px_x as i32 - px_x as i32 + half;
        let crop_y = icon_px_y as i32 - px_y as i32 + half;
        if crop_x < 0 || crop_y < 0 || crop_x as usize >= ds || crop_y as usize >= ds {
            continue;
        }
        if !mask[crop_y as usize * ds + crop_x as usize] {
            continue;
        }
        draw_tracking_icon(data, ds, crop_x, crop_y, point.kind);
    }
}

fn gather_tracking_points(
    tracking_state: Option<&MinimapUIState>,
    world_object_q: &Query<(
        &GlobalTransform,
        &WorldObjectInteraction,
        Option<&Visibility>,
    )>,
    herb_q: &Query<(&GlobalTransform, Option<&Visibility>), With<MinimapHerbNode>>,
    quest_q: &Query<(&GlobalTransform, &QuestTrackedItem, Option<&Visibility>)>,
    quest_log: Option<&QuestLogState>,
) -> Vec<TrackingPoint> {
    let mut points = Vec::new();
    let tracking_filter = tracking_filter(tracking_state);

    append_world_object_tracking_points(&mut points, world_object_q, tracking_filter.show_minerals);
    if tracking_filter.show_herbs {
        append_herb_tracking_points(&mut points, herb_q);
    }
    append_quest_tracking_points(&mut points, quest_q, quest_log);
    points
}

fn limit_tracking_points(mut points: Vec<TrackingPoint>) -> Vec<TrackingPoint> {
    points.truncate(MAX_TRACKING_ICONS);
    points
}

fn tracking_filter(tracking_state: Option<&MinimapUIState>) -> TrackingFilter {
    TrackingFilter {
        show_herbs: tracking_state
            .is_none_or(|state| matches!(state.tracking, TrackingType::None | TrackingType::Herbs)),
        show_minerals: tracking_state.is_none_or(|state| {
            matches!(state.tracking, TrackingType::None | TrackingType::Minerals)
        }),
    }
}

fn append_world_object_tracking_points(
    points: &mut Vec<TrackingPoint>,
    world_object_q: &Query<(
        &GlobalTransform,
        &WorldObjectInteraction,
        Option<&Visibility>,
    )>,
    show_minerals: bool,
) {
    for (transform, interaction, visibility) in world_object_q.iter() {
        if is_hidden(visibility) {
            continue;
        }
        let Some(kind) = tracking_icon_kind(interaction.kind, show_minerals) else {
            continue;
        };
        push_tracking_point(points, transform, kind);
    }
}

fn append_herb_tracking_points(
    points: &mut Vec<TrackingPoint>,
    herb_q: &Query<(&GlobalTransform, Option<&Visibility>), With<MinimapHerbNode>>,
) {
    for (transform, visibility) in herb_q.iter() {
        if is_hidden(visibility) {
            continue;
        }
        push_tracking_point(points, transform, TrackingIconKind::Herb);
    }
}

fn append_quest_tracking_points(
    points: &mut Vec<TrackingPoint>,
    quest_q: &Query<(&GlobalTransform, &QuestTrackedItem, Option<&Visibility>)>,
    quest_log: Option<&QuestLogState>,
) {
    let Some(quest_log) = quest_log else { return };
    for (transform, tracked, visibility) in quest_q.iter() {
        if is_hidden(visibility) || !should_sparkle(tracked, quest_log) {
            continue;
        }
        push_tracking_point(points, transform, TrackingIconKind::QuestObjective);
    }
}

fn push_tracking_point(
    points: &mut Vec<TrackingPoint>,
    transform: &GlobalTransform,
    kind: TrackingIconKind,
) {
    let position = GlobalTransform::translation(transform);
    points.push(TrackingPoint {
        kind,
        bx: position.x,
        bz: position.z,
    });
}

fn tracking_icon_kind(
    interaction: WorldObjectInteractionKind,
    show_minerals: bool,
) -> Option<TrackingIconKind> {
    match interaction {
        WorldObjectInteractionKind::Mailbox => Some(TrackingIconKind::Mailbox),
        WorldObjectInteractionKind::GatherNode(_) if show_minerals => {
            Some(TrackingIconKind::Mineral)
        }
        _ => None,
    }
}

fn is_hidden(visibility: Option<&Visibility>) -> bool {
    visibility.is_some_and(|value| *value == Visibility::Hidden)
}
