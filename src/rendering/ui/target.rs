use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::gossip_data::GossipIntentQueue;
use game_engine::targeting::CurrentTarget;
use shared::components::Npc;

use crate::camera::Player;
use crate::game_state::GameState;
use crate::networking::RemoteEntity;
use game_engine::input_bindings::{InputAction, InputBindings};

type RemoteTargetQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, Option<&'static Visibility>),
    (With<RemoteEntity>, With<Npc>, Without<Player>),
>;

#[path = "target_visuals.rs"]
mod target_visuals;

use target_visuals::{spawn_target_circle, update_target_circle};

/// Marker on the selection circle entity.
#[derive(Component)]
pub struct TargetMarker;

#[derive(Component, Clone, Copy)]
struct TargetMarkerScaleFactor(f32);

/// Which visual style the target selection circle uses.
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub enum TargetCircleStyle {
    /// Procedural yellow ring + fill (no texture).
    Procedural,
    /// BLP texture by FDID pair: (base, optional glow).
    Blp {
        name: String,
        base_fdid: u32,
        glow_fdid: Option<u32>,
        emissive: [u8; 3],
    },
}

impl Default for TargetCircleStyle {
    fn default() -> Self {
        blp_style("Fat Ring", 167207, None, [255, 220, 50])
    }
}

impl TargetCircleStyle {
    pub fn label(&self) -> &str {
        match self {
            Self::Procedural => "Procedural",
            Self::Blp { name, .. } => name,
        }
    }
}

fn blp_style(name: &str, base: u32, glow: Option<u32>, rgb: [u8; 3]) -> TargetCircleStyle {
    TargetCircleStyle::Blp {
        name: name.into(),
        base_fdid: base,
        glow_fdid: glow,
        emissive: rgb,
    }
}

/// All available circle styles for the debug picker.
pub fn available_circle_styles() -> Vec<TargetCircleStyle> {
    let mut styles = vec![TargetCircleStyle::Procedural];
    styles.extend(white_ring_styles());
    styles.extend(spell_area_styles());
    styles
}

fn white_ring_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Thin Ring (Hostile)", 167208, None, [255, 40, 40]),
        blp_style("Thin Ring (Friendly)", 167208, None, [40, 255, 40]),
        blp_style("Thin Ring (Neutral)", 167208, None, [255, 220, 50]),
        blp_style("Fat Ring", 167207, None, [255, 220, 50]),
        blp_style("Ring Glow", 651522, None, [255, 220, 50]),
        blp_style("Double Ring", 623667, None, [255, 220, 50]),
        blp_style("Reticle", 166706, None, [255, 255, 255]),
    ]
}

fn spell_area_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Holy", 1001694, None, [255, 240, 150]),
        blp_style("Fire", 1001600, None, [255, 120, 30]),
        blp_style("Arcane", 1001690, None, [180, 130, 255]),
        blp_style("Frost", 1001693, None, [100, 200, 255]),
        blp_style("Nature", 1001695, None, [100, 220, 80]),
        blp_style("Shadow", 1001697, None, [160, 80, 220]),
    ]
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.init_resource::<TargetCircleStyle>();
        app.init_resource::<GossipIntentQueue>();
        app.add_systems(
            Update,
            (
                click_to_target,
                tab_target,
                self_target,
                clear_target,
                right_click_interact,
                spawn_target_circle,
                update_target_circle,
            )
                .run_if(targeting_state_active),
        );
    }
}

fn targeting_state_active(state: Res<State<GameState>>) -> bool {
    matches!(
        *state.get(),
        GameState::InWorld | GameState::InWorldSelectionDebug
    )
}

/// Raycast from camera through mouse cursor on left-click. Target the hit RemoteEntity.
fn click_to_target(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut ray_cast: MeshRayCast,
    parent_query: Query<&ChildOf>,
    remote_q: Query<Entity, (With<RemoteEntity>, Without<Player>)>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Some(ray) = camera.viewport_to_world(cam_tf, cursor).ok() else {
        return;
    };

    let hits = ray_cast.cast_ray(ray, &default());
    for &(entity, _) in hits {
        if let Some(target) = resolve_targetable_ancestor(entity, &parent_query, &remote_q) {
            current.0 = Some(target);
            return;
        }
    }
}

/// On Tab, cycle through nearby RemoteEntity sorted by distance from local player.
fn tab_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<&Transform, With<Player>>,
    remote_q: RemoteTargetQuery<'_, '_>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetNearest, &keys, &mouse_buttons) {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let sorted = sorted_targets_by_distance(player_tf, &remote_q);
    current.0 = pick_next_target(&sorted, current.0);
}

/// Sort remote entities by distance from player, return entity list.
fn sorted_targets_by_distance(
    player_tf: &Transform,
    remote_q: &RemoteTargetQuery<'_, '_>,
) -> Vec<Entity> {
    let mut entities: Vec<(Entity, f32)> = remote_q
        .iter()
        .filter(|(_, _, visibility)| visibility.is_none_or(|value| *value != Visibility::Hidden))
        .map(|(entity, transform, _)| {
            (
                entity,
                transform
                    .translation
                    .distance_squared(player_tf.translation),
            )
        })
        .collect();
    entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// Pick the next target after the current one in the sorted list, wrapping around.
fn pick_next_target(sorted: &[Entity], current: Option<Entity>) -> Option<Entity> {
    if sorted.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return Some(sorted[0]);
    };
    let idx = sorted.iter().position(|&e| e == cur);
    match idx {
        Some(i) => Some(sorted[(i + 1) % sorted.len()]),
        None => Some(sorted[0]),
    }
}

pub(crate) fn resolve_targetable_ancestor(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    remote_q: &Query<Entity, (With<RemoteEntity>, Without<Player>)>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if remote_q.get(current).is_ok() {
            return Some(current);
        }
        let Ok(parent) = parent_query.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

/// On F1, set the current target to the local player entity.
fn self_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<Entity, With<Player>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetSelf, &keys, &mouse_buttons) {
        return;
    }
    let Ok(player) = player_q.single() else {
        return;
    };
    current.0 = Some(player);
}

/// On Escape, clear the current target.
fn clear_target(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        current.0 = None;
    }
}

/// Maximum distance (world units) at which NPC interaction is allowed.
const INTERACT_RANGE: f32 = 5.0;

/// On right-click, interact with the targeted NPC if within range.
fn right_click_interact(
    mouse: Res<ButtonInput<MouseButton>>,
    current: Res<CurrentTarget>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    npc_q: Query<&GlobalTransform, With<Npc>>,
    mut gossip_queue: ResMut<GossipIntentQueue>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(target_entity) = current.0 else {
        return;
    };
    let Ok(npc_tf) = npc_q.get(target_entity) else {
        return;
    };
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let distance = player_tf.translation().distance(npc_tf.translation());
    if distance > INTERACT_RANGE {
        return;
    }
    gossip_queue.interact(target_entity.to_bits());
}

/// When CurrentTarget changes, spawn or move the selection circle.
#[cfg(test)]
#[path = "../../../tests/unit/target_tests.rs"]
mod tests;
