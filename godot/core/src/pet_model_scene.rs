//! Retail UIModelSceneCamera/Actor data and ModelSceneActorMixin normalization.
use glam::{Quat, Vec3};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PetModelScene {
    pub id: u32,
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub zoom_distance: f32,
    pub actor_position: [f32; 3],
    pub actor_orientation: [f32; 3],
    pub origin_flags: u32,
    pub normalized_scale: f32,
    pub scale: f32,
}
impl PetModelScene {
    pub fn camera_vectors(&self) -> (Vec3, Vec3, Vec3) {
        // Vector3D_CalculateNormalFromYawPitch; OrbitCamera places eye at target-normal*zoom.
        let rotation = engine_rotation([self.yaw, self.pitch, self.roll]);
        let target = engine_vector(self.target);
        let eye = target - rotation * Vec3::X * self.zoom_distance;
        (eye, target, rotation * Vec3::Y)
    }
    pub fn actor_scale(&self, size: Vec3) -> f32 {
        if self.normalized_scale <= 0.0 {
            return self.scale;
        }
        // ModelSceneActorMixin MODEL_SCENE_ACTOR_DIMENSIONS_FOR_NORMALIZATION (WoW XYZ).
        let max_scale = (size.x / 0.9984).max(size.z / 1.121).max(size.y / 2.118);
        if max_scale == 0.0 {
            return self.scale;
        }
        self.scale / (1.0 + (max_scale - 1.0) * self.normalized_scale)
    }
    pub fn actor_rotation(&self) -> Quat {
        engine_rotation(self.actor_orientation)
    }
    pub fn actor_origin(&self, center: Vec3, scale: f32) -> Vec3 {
        // UIModelSceneActorFlag UseCenterForOrigin X=2,Y=4,Z=8, mapped into engine axes.
        let centered = Vec3::new(
            if self.origin_flags & 2 != 0 {
                center.x
            } else {
                0.0
            },
            if self.origin_flags & 8 != 0 {
                center.y
            } else {
                0.0
            },
            if self.origin_flags & 4 != 0 {
                center.z
            } else {
                0.0
            },
        );
        engine_vector(self.actor_position) - self.actor_rotation() * centered * scale
    }
}
fn engine_vector([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}
fn engine_rotation([yaw, pitch, roll]: [f32; 3]) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-pitch) * Quat::from_rotation_x(roll)
}
