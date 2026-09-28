use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility};
use bevy::prelude::*;

use crate::collision::WmoCollisionMesh;
use crate::sky::SkyDome;
use crate::terrain_heightmap::TerrainHeightmap;
use game_engine::camera_follow_data::{COLLISION_OFFSET, follow_camera as follow_camera_data};
use game_engine::culling::WmoInteriors;

use super::{GROUND_Y, Player, WowCamera};

type FollowPlayerQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform), (With<Player>, Without<WowCamera>)>;
type FollowCameraQuery<'w, 's> =
    Query<'w, 's, (&'static mut WowCamera, &'static mut Transform), Without<Player>>;

/// Recursively collect all descendant entities into the set.
fn collect_descendants(entity: Entity, children_q: &Query<&Children>, out: &mut HashSet<Entity>) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            out.insert(child);
            collect_descendants(child, children_q, out);
        }
    }
}

/// Build the set of entities excluded from camera collision (player + children + sky).
fn build_collision_excluded_set(
    player_entity: Entity,
    children_q: &Query<&Children>,
    sky_q: &Query<Entity, With<SkyDome>>,
) -> HashSet<Entity> {
    let mut excluded = HashSet::new();
    excluded.insert(player_entity);
    collect_descendants(player_entity, children_q, &mut excluded);
    for entity in sky_q.iter() {
        excluded.insert(entity);
    }
    excluded
}

/// What the camera collides with: visible meshes and WMO walls even when portal culling
/// hides their group. Portal culling controls drawing, not solidity.
#[derive(SystemParam)]
pub(crate) struct CameraBlockers<'w, 's> {
    visibility: Query<'w, 's, &'static InheritedVisibility>,
    wmo_walls: Query<'w, 's, (), With<WmoCollisionMesh>>,
}

impl CameraBlockers<'_, '_> {
    fn blocks(&self, entity: Entity) -> bool {
        self.visibility.get(entity).is_ok_and(|v| v.get()) || self.wmo_walls.contains(entity)
    }
}

fn collision_adjusted_distance(intended_distance: f32, hit_distance: Option<f32>) -> f32 {
    match hit_distance {
        Some(hit) if hit < intended_distance => (hit - COLLISION_OFFSET).max(0.5),
        _ => intended_distance,
    }
}

/// Cast against world meshes selected by the WMO-aware collision predicate.
fn mesh_hit_distance(
    ray_cast: &mut MeshRayCast,
    blocks: &dyn Fn(Entity) -> bool,
    eye_target: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    let ray = Ray3d::new(eye_target, Dir3::new(ray_dir).unwrap());
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)
        .with_filter(&blocks);
    ray_cast
        .cast_ray(ray, &settings)
        .first()
        .map(|(_, hit)| hit.distance)
}

/// Pull a smoothed camera position back in front of the first blocker between it and the eye.
/// The collision ray validates only the target pose; smoothing moves the camera along a
/// straight line from its last pose, which can cut through a stair nose or a wall corner.
fn keep_in_sight(
    eye_target: Vec3,
    camera: Vec3,
    ray_cast: &mut MeshRayCast,
    blocks: &dyn Fn(Entity) -> bool,
) -> Vec3 {
    let offset = camera - eye_target;
    let distance = offset.length();
    let Ok(direction) = Dir3::new(offset) else {
        return camera;
    };
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)
        .with_filter(&blocks);
    match ray_cast
        .cast_ray(Ray3d::new(eye_target, direction), &settings)
        .first()
    {
        Some((_, hit)) if hit.distance < distance => {
            eye_target + direction * collision_adjusted_distance(distance, Some(hit.distance))
        }
        _ => camera,
    }
}

/// The height the camera stays above: the terrain, `GROUND_Y` before its tile loads, and
/// none on a WMO-only map, whose WMO walls (the mesh ray cast) are its only bounds.
fn camera_ground(terrain: Option<&TerrainHeightmap>, pos: Vec3) -> Option<f32> {
    match terrain {
        Some(heightmap) if heightmap.is_wmo_only() => None,
        Some(heightmap) => Some(heightmap.height_at(pos.x, pos.z).unwrap_or(GROUND_Y)),
        None => Some(GROUND_Y),
    }
}

fn follow_target(player_q: &FollowPlayerQuery<'_, '_>) -> Option<(Entity, Vec3)> {
    let Ok((entity, transform)) = player_q.single() else {
        return None;
    };
    Some((entity, transform.translation))
}

pub(crate) fn camera_follow(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    player_q: FollowPlayerQuery<'_, '_>,
    mut camera_q: FollowCameraQuery<'_, '_>,
    mut ray_cast: MeshRayCast,
    sky_q: Query<Entity, With<SkyDome>>,
    children_q: Query<&Children>,
    blockers: CameraBlockers,
    interiors: WmoInteriors,
) {
    let Some((player_entity, target_translation)) = follow_target(&player_q) else {
        return;
    };
    let Ok((mut cam, mut cam_tf)) = camera_q.single_mut() else {
        return;
    };

    let eye_target = target_translation + Vec3::Y * game_engine::camera_follow_data::EYE_HEIGHT;
    // WMO interiors use their walls, not terrain above or around the interior, as bounds.
    let in_interior = interiors.contain(eye_target);
    let excluded = build_collision_excluded_set(player_entity, &children_q, &sky_q);
    let blocks = |entity: Entity| !excluded.contains(&entity) && blockers.blocks(entity);
    let mut height_at = |x, z| terrain.as_deref().and_then(|map| map.height_at(x, z));
    let terrain_samples: Option<&mut dyn FnMut(f32, f32) -> Option<f32>> = terrain
        .as_ref()
        .filter(|_| !in_interior)
        .map(|_| &mut height_at as &mut _);
    let pose = follow_camera_data(
        &mut cam,
        cam_tf.translation,
        target_translation,
        time.delta_secs(),
        terrain_samples,
        |eye, dir| mesh_hit_distance(&mut ray_cast, &blocks, eye, dir),
        |pos| camera_ground(terrain.as_deref(), pos).filter(|_| !in_interior),
    );
    let mut next_transform = *cam_tf;
    next_transform.translation =
        keep_in_sight(pose.eye_target, pose.position, &mut ray_cast, &blocks);
    next_transform.look_at(pose.eye_target, Vec3::Y);
    cam_tf.set_if_neq(next_transform);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::primitives::Aabb;
    use game_engine::camera_follow_data::EYE_HEIGHT;
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct CameraTransformChanges(Vec<Transform>);

    fn record_camera_changes(
        cameras: Query<&Transform, (With<WowCamera>, Changed<Transform>)>,
        mut changes: ResMut<CameraTransformChanges>,
    ) {
        changes.0.extend(cameras.iter().copied());
    }

    fn follow_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default())
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<CameraTransformChanges>()
            .add_systems(Update, (camera_follow, record_camera_changes).chain());
        let target = Vec3::new(1.0, 0.0, 2.0);
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_translation(target)))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                WowCamera(game_engine::camera_control_data::CameraState {
                    pitch: 0.0,
                    distance: 10.0,
                    target_distance: 10.0,
                    ..default()
                }),
                Transform::from_xyz(1.0, EYE_HEIGHT, 12.0)
                    .looking_at(target + Vec3::Y * EYE_HEIGHT, Vec3::Y)
                    .with_scale(Vec3::new(1.25, 0.75, 1.5)),
            ))
            .id();
        advance_follow(&mut app, 0.1);
        app.world_mut()
            .resource_mut::<CameraTransformChanges>()
            .0
            .clear();
        (app, player, camera)
    }

    fn advance_follow(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(seconds));
        app.update();
    }

    #[test]
    fn camera_direction_changes_rendered_view_and_persists() {
        let (mut app, player, camera) = follow_app();
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .y = 100.0;
        app.world_mut()
            .get_mut::<WowCamera>(camera)
            .unwrap()
            .set_direction_degrees(Some(90.0), Some(60.0))
            .unwrap();
        let expected = Quat::from_euler(
            EulerRot::YXZ,
            90.0_f32.to_radians(),
            60.0_f32.to_radians(),
            0.0,
        ) * Vec3::NEG_Z;
        for _ in 0..3 {
            advance_follow(&mut app, 0.1);
            let transform = app.world().get::<Transform>(camera).unwrap();
            assert!(transform.forward().as_vec3().abs_diff_eq(expected, 1e-5));
            assert!(transform.forward().y > 0.8, "positive pitch looks upward");
        }
    }

    #[test]
    fn camera_stays_below_the_world_origin_on_a_wmo_only_map() {
        let (mut app, player, camera) = follow_app();
        // The Stockade floor lies near Bevy y -19.3, below GROUND_Y.
        let mut heightmap = TerrainHeightmap::default();
        heightmap.set_wmo_only();
        app.insert_resource(heightmap);
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .y = -19.3;
        for _ in 0..40 {
            advance_follow(&mut app, 0.1);
        }
        let camera_y = app.world().get::<Transform>(camera).unwrap().translation.y;
        assert!(
            (camera_y - (-19.3 + EYE_HEIGHT)).abs() < 0.1,
            "camera at y {camera_y}"
        );
    }

    #[test]
    fn camera_follow_settled_pose_does_not_notify_transform_changes() {
        let (mut app, _, camera) = follow_app();
        let settled = *app.world().get::<Transform>(camera).unwrap();
        for _ in 0..3 {
            advance_follow(&mut app, 0.1);
            assert_eq!(*app.world().get::<Transform>(camera).unwrap(), settled);
        }
        assert!(
            app.world()
                .resource::<CameraTransformChanges>()
                .0
                .is_empty(),
            "settled camera frames must not notify downstream transform consumers",
        );
    }

    #[test]
    fn camera_follow_movement_preserves_smoothing_rotation_and_scale() {
        let (mut app, player, camera) = follow_app();
        let scale = app.world().get::<Transform>(camera).unwrap().scale;
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .x = 9.0;
        advance_follow(&mut app, 0.025);
        let expected = Transform::from_xyz(3.0, EYE_HEIGHT, 12.0)
            .looking_at(Vec3::new(9.0, EYE_HEIGHT, 2.0), Vec3::Y)
            .with_scale(scale);
        assert_eq!(*app.world().get::<Transform>(camera).unwrap(), expected);
        assert_eq!(
            app.world().resource::<CameraTransformChanges>().0,
            vec![expected]
        );
    }

    #[test]
    fn camera_follow_retains_mesh_collision_and_recovery() {
        let (mut app, _, camera) = follow_app();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(2.0, 4.0, 1.0));
        let visible = ViewVisibility::VISIBLE;
        let wall = app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                Transform::from_xyz(1.0, EYE_HEIGHT, 7.0),
                GlobalTransform::from_xyz(1.0, EYE_HEIGHT, 7.0),
                Aabb::from_min_max(Vec3::new(-1.0, -2.0, -0.5), Vec3::new(1.0, 2.0, 0.5)),
                InheritedVisibility::VISIBLE,
                visible,
            ))
            .id();
        advance_follow(&mut app, 0.1);
        assert!(
            app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .abs_diff_eq(Vec3::new(1.0, EYE_HEIGHT, 6.2), 0.0001,)
        );
        assert!(
            app.world()
                .get::<WowCamera>(camera)
                .unwrap()
                .collision_distance
                .is_some()
        );
        assert_eq!(app.world().resource::<CameraTransformChanges>().0.len(), 1);

        app.world_mut().despawn(wall);
        advance_follow(&mut app, 0.1);
        assert!(
            app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .abs_diff_eq(Vec3::new(1.0, EYE_HEIGHT, 9.1), 0.0001,)
        );
        assert!(
            app.world()
                .get::<WowCamera>(camera)
                .unwrap()
                .collision_distance
                .is_some()
        );
        advance_follow(&mut app, 0.2);
        assert!(
            app.world()
                .get::<WowCamera>(camera)
                .unwrap()
                .collision_distance
                .is_none()
        );
        assert!(
            app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .abs_diff_eq(Vec3::new(1.0, EYE_HEIGHT, 12.0), 0.0001,)
        );
    }

    #[test]
    fn camera_collision_survives_wall_leaving_view_and_honors_hidden_parent() {
        use bevy::camera::visibility::{VisibilityPlugin, VisibilitySystems, update_frusta};

        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            TransformPlugin,
            VisibilityPlugin,
        ))
        .init_resource::<Time>()
        .add_systems(
            PostUpdate,
            update_frusta.in_set(VisibilitySystems::UpdateFrusta),
        );
        let eye = Vec3::Y * EYE_HEIGHT;
        app.world_mut().spawn((Player, Transform::default()));
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                WowCamera(game_engine::camera_control_data::CameraState {
                    pitch: 0.0,
                    distance: 10.0,
                    target_distance: 10.0,
                    ..default()
                }),
                Transform::from_translation(eye + Vec3::Z * 10.0).looking_at(eye, Vec3::Y),
            ))
            .id();
        let parent = app
            .world_mut()
            .spawn((Transform::default(), Visibility::Inherited))
            .id();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(2.0, 4.0, 1.0));
        let wall = app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                Transform::from_translation(eye + Vec3::Z * 5.0),
                ChildOf(parent),
            ))
            .id();

        // Let real transform propagation and frustum culling establish the first view.
        advance_follow(&mut app, 0.1);
        assert!(app.world().get::<ViewVisibility>(wall).unwrap().get());
        app.add_systems(Update, camera_follow);
        advance_follow(&mut app, 0.1);
        let collision_pose = *app.world().get::<Transform>(camera).unwrap();
        assert!((collision_pose.translation.z - 4.2).abs() < 0.0001);
        assert!(
            !app.world().get::<ViewVisibility>(wall).unwrap().get(),
            "the wall is now behind the collision-adjusted camera"
        );
        assert!(app.world().get::<InheritedVisibility>(wall).unwrap().get());

        for _ in 0..3 {
            advance_follow(&mut app, 0.1);
            assert!(
                app.world()
                    .get::<Transform>(camera)
                    .unwrap()
                    .translation
                    .abs_diff_eq(collision_pose.translation, 0.0001),
                "view culling must not let the camera recover through the wall"
            );
        }

        *app.world_mut().get_mut::<Visibility>(parent).unwrap() = Visibility::Hidden;
        advance_follow(&mut app, 0.1);
        assert!(!app.world().get::<InheritedVisibility>(wall).unwrap().get());
        advance_follow(&mut app, 0.1);
        assert!(
            (app.world().get::<Transform>(camera).unwrap().translation.z - 7.1).abs() < 0.0001,
            "a hierarchically hidden wall must permit normal collision recovery"
        );
    }
}
