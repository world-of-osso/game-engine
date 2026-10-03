//! Shared local-authority correction and remote interpolation math.

use glam::{Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionPose {
    pub position: Vec3,
    pub rotation: Quat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionTarget {
    pub position: Vec3,
    pub yaw: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ServerMotionUpdate {
    pub pose: MotionPose,
    pub adopted_epoch: u32,
    pub facing_yaw: Option<f32>,
}

/// Adopt the first epoch without teleporting; later epoch changes are authoritative.
pub fn follow_server_motion(
    mut pose: MotionPose,
    target: MotionTarget,
    adopted_epoch: Option<u32>,
    epoch: u32,
    controlled: bool,
    dt: f32,
) -> ServerMotionUpdate {
    if adopted_epoch.is_some_and(|adopted| adopted != epoch) {
        pose.position = target.position;
    }
    if !controlled {
        return ServerMotionUpdate {
            pose,
            adopted_epoch: epoch,
            facing_yaw: None,
        };
    }
    pose.position = pose.position.lerp(target.position, (10.0 * dt).min(1.0));
    if let Some(yaw) = target.yaw {
        pose.rotation = Quat::from_rotation_y(yaw);
    }
    ServerMotionUpdate {
        pose,
        adopted_epoch: epoch,
        facing_yaw: target.yaw,
    }
}

/// Follow the newest remote snapshot with the original capped interpolation rate.
pub fn interpolate_remote_motion(pose: MotionPose, target: MotionTarget, dt: f32) -> MotionPose {
    let t = (10.0 * dt).min(1.0);
    MotionPose {
        position: pose.position.lerp(target.position, t),
        rotation: target.yaw.map_or(pose.rotation, |yaw| {
            pose.rotation.slerp(Quat::from_rotation_y(yaw), t)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    fn pose(x: f32, yaw: f32) -> MotionPose {
        MotionPose {
            position: Vec3::new(x, 2.0, 3.0),
            rotation: Quat::from_rotation_y(yaw),
        }
    }

    fn target(x: f32, yaw: Option<f32>) -> MotionTarget {
        MotionTarget {
            position: Vec3::new(x, 2.0, 3.0),
            yaw,
        }
    }

    #[test]
    fn uncontrolled_player_keeps_prediction_and_adopts_first_epoch_without_snapping() {
        for adopted in [None, Some(4)] {
            let result = follow_server_motion(
                pose(2.0, 0.25),
                target(10.0, Some(1.0)),
                adopted,
                4,
                false,
                0.05,
            );
            assert_eq!(result.pose, pose(2.0, 0.25));
            assert_eq!(result.adopted_epoch, 4);
            assert_eq!(result.facing_yaw, None);
        }
    }

    #[test]
    fn changed_epoch_snaps_uncontrolled_player_without_changing_facing() {
        let result = follow_server_motion(
            pose(2.0, 0.25),
            target(10.0, Some(1.0)),
            Some(3),
            4,
            false,
            0.05,
        );
        assert_eq!(result.pose, pose(10.0, 0.25));
        assert_eq!(result.adopted_epoch, 4);
        assert_eq!(result.facing_yaw, None);
    }

    #[test]
    fn changed_epoch_snaps_controlled_player_before_following() {
        let result = follow_server_motion(
            pose(2.0, 0.25),
            target(10.0, Some(1.0)),
            Some(3),
            4,
            true,
            0.05,
        );
        assert_eq!(result.pose, pose(10.0, 1.0));
        assert_eq!(result.facing_yaw, Some(1.0));
    }

    #[test]
    fn controlled_player_follows_halfway_and_sets_yaw_directly() {
        let result = follow_server_motion(
            pose(2.0, 0.25),
            target(10.0, Some(1.0)),
            Some(4),
            4,
            true,
            0.05,
        );
        assert_eq!(result.pose, pose(6.0, 1.0));
        assert_eq!(result.facing_yaw, Some(1.0));
    }

    #[test]
    fn missing_yaw_preserves_rotation_and_does_not_update_facing() {
        let result =
            follow_server_motion(pose(2.0, 0.25), target(10.0, None), Some(4), 4, true, 0.05);
        assert_eq!(result.pose, pose(6.0, 0.25));
        assert_eq!(result.facing_yaw, None);
    }

    #[test]
    fn remote_interpolation_uses_half_step_and_shortest_quaternion_path() {
        let original = pose(2.0, 170.0_f32.to_radians());
        let result = interpolate_remote_motion(
            original,
            target(10.0, Some((-170.0_f32).to_radians())),
            0.05,
        );
        assert_eq!(result.position, Vec3::new(6.0, 2.0, 3.0));
        let expected = original
            .rotation
            .slerp(Quat::from_rotation_y((-170.0_f32).to_radians()), 0.5);
        assert!(result.rotation.dot(expected).abs() > 0.99999);
    }

    #[test]
    fn interpolation_caps_delta_at_one() {
        let result = interpolate_remote_motion(pose(2.0, 0.25), target(10.0, Some(1.0)), 0.2);
        assert_eq!(result.position, Vec3::new(10.0, 2.0, 3.0));
        assert!(result.rotation.dot(Quat::from_rotation_y(1.0)).abs() > 0.99999);
    }

    #[test]
    fn missing_remote_yaw_preserves_rotation() {
        let result = interpolate_remote_motion(pose(2.0, 0.25), target(10.0, None), 0.05);
        assert_eq!(result, pose(6.0, 0.25));
    }
}
