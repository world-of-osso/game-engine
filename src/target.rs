use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::targeting::CurrentTarget;

use crate::camera::Player;
use crate::game_state::GameState;
use crate::networking::RemoteEntity;
use game_engine::input_bindings::{InputAction, InputBindings};

/// Marker on the selection circle entity.
#[derive(Component)]
pub struct TargetMarker;

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.add_systems(
            Update,
            (
                click_to_target,
                tab_target,
                self_target,
                clear_target,
                spawn_target_circle,
                update_target_circle,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
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
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
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
#[allow(clippy::type_complexity)]
fn tab_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<&Transform, With<Player>>,
    remote_q: Query<(Entity, &Transform), (With<RemoteEntity>, Without<Player>)>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
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
#[allow(clippy::type_complexity)]
fn sorted_targets_by_distance(
    player_tf: &Transform,
    remote_q: &Query<(Entity, &Transform), (With<RemoteEntity>, Without<Player>)>,
) -> Vec<Entity> {
    let mut entities: Vec<(Entity, f32)> = remote_q
        .iter()
        .map(|(e, tf)| (e, tf.translation.distance_squared(player_tf.translation)))
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

fn target_circle_transform(target_translation: Vec3) -> Transform {
    Transform::from_translation(target_translation + Vec3::Y * 0.08)
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(1.0))
}

/// On F1, set the current target to the local player entity.
fn self_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<Entity, With<Player>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
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

/// When CurrentTarget changes, spawn or move the selection circle.
fn spawn_target_circle(
    current: Res<CurrentTarget>,
    mut commands: Commands,
    existing: Query<Entity, With<TargetMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    target_tf: Query<&Transform>,
) {
    if !current.is_changed() {
        return;
    }
    // Remove old circle
    for e in existing.iter() {
        commands.entity(e).despawn();
    }
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };

    let ring = meshes.add(Annulus::new(0.7, 0.95).mesh().resolution(64));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.2, 0.92),
        emissive: LinearRgba::rgb(8.0, 7.0, 1.5),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(mat),
        target_circle_transform(tf.translation),
        TargetMarker,
    ));
}

/// Keep the selection circle positioned under the current target each frame.
fn update_target_circle(
    current: Res<CurrentTarget>,
    target_tf: Query<&Transform, Without<TargetMarker>>,
    mut circle_q: Query<&mut Transform, With<TargetMarker>>,
) {
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };
    for mut circle_tf in circle_q.iter_mut() {
        circle_tf.translation = tf.translation + Vec3::Y * 0.05;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct TargetResolutionResult(Option<Entity>);

    #[test]
    fn test_target_circle_transform_stays_flat_on_ground() {
        let transform = target_circle_transform(Vec3::new(10.0, 2.0, 5.0));
        assert_eq!(transform.translation, Vec3::new(10.0, 2.08, 5.0));
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        );
    }

    #[test]
    fn test_tab_cycles_targets() {
        // 3 entities at different distances from origin
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();

        // Spawn player at origin
        let _player = app
            .world_mut()
            .spawn((Transform::from_xyz(0.0, 0.0, 0.0), Player))
            .id();

        // Spawn 3 remote entities at increasing distances
        let e1 = app
            .world_mut()
            .spawn((Transform::from_xyz(5.0, 0.0, 0.0), RemoteEntity))
            .id();
        let e2 = app
            .world_mut()
            .spawn((Transform::from_xyz(10.0, 0.0, 0.0), RemoteEntity))
            .id();
        let e3 = app
            .world_mut()
            .spawn((Transform::from_xyz(15.0, 0.0, 0.0), RemoteEntity))
            .id();

        // Simulate tab cycling by calling pick_next_target directly
        let sorted = vec![e1, e2, e3];

        // First tab: pick closest
        let t1 = pick_next_target(&sorted, None);
        assert_eq!(t1, Some(e1));

        // Second tab: pick next
        let t2 = pick_next_target(&sorted, t1);
        assert_eq!(t2, Some(e2));

        // Third tab: pick next
        let t3 = pick_next_target(&sorted, t2);
        assert_eq!(t3, Some(e3));

        // Fourth tab: wrap around
        let t4 = pick_next_target(&sorted, t3);
        assert_eq!(t4, Some(e1));
    }

    #[test]
    fn test_escape_clears_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, clear_target);

        // Set a target
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(entity);

        // Press Escape
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let target = app.world().resource::<CurrentTarget>();
        assert_eq!(target.0, None, "Escape should clear the target");
    }

    #[test]
    fn test_target_circle_follows_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();

        // Spawn a target entity
        let target = app
            .world_mut()
            .spawn(Transform::from_xyz(10.0, 0.0, 5.0))
            .id();

        // Spawn a circle tracking it
        let circle = app
            .world_mut()
            .spawn((Transform::from_xyz(0.0, 0.0, 0.0), TargetMarker))
            .id();

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
        app.add_systems(Update, update_target_circle);
        app.update();

        let circle_pos = app
            .world()
            .entity(circle)
            .get::<Transform>()
            .unwrap()
            .translation;
        assert!(
            (circle_pos.x - 10.0).abs() < 0.01,
            "circle x should follow target, got {}",
            circle_pos.x
        );
        assert!(
            (circle_pos.z - 5.0).abs() < 0.01,
            "circle z should follow target, got {}",
            circle_pos.z
        );
    }

    #[test]
    fn test_resolve_targetable_ancestor_finds_remote_root_from_child_mesh() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<TargetResolutionResult>();

        let root = app
            .world_mut()
            .spawn((Transform::default(), RemoteEntity))
            .id();
        let child = app.world_mut().spawn(Transform::default()).id();
        app.world_mut().entity_mut(child).insert(ChildOf(root));
        app.add_systems(
            Update,
            move |parent_query: Query<&ChildOf>,
                  remote_query: Query<Entity, (With<RemoteEntity>, Without<Player>)>,
                  mut result: ResMut<TargetResolutionResult>| {
                result.0 = resolve_targetable_ancestor(child, &parent_query, &remote_query);
            },
        );
        app.update();

        assert_eq!(
            app.world().resource::<TargetResolutionResult>().0,
            Some(root)
        );
    }
}
