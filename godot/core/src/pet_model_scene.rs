//! Retail UIModelSceneCamera/Actor data and ModelSceneActorMixin normalization.
use crate::model_scene_actor::{
    actor_origin, engine_rotation, engine_vector, normalized_actor_scale,
};
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
        normalized_actor_scale(self.scale, self.normalized_scale, size)
    }
    pub fn actor_rotation(&self) -> Quat {
        engine_rotation(self.actor_orientation)
    }
    pub fn actor_origin(&self, center: Vec3, scale: f32) -> Vec3 {
        actor_origin(
            self.origin_flags,
            self.actor_position,
            self.actor_orientation,
            center,
            scale,
        )
    }
}
