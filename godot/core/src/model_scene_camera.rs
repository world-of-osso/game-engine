//! Trading Post's authored base camera (Products.TranslateDisplayInfo).
//! Inherited native zoom splines and ModelScene default FOV remain explicit gaps.
use crate::{
    csv_util::read_db2_rows,
    model_scene_actor::{engine_rotation, engine_vector},
};
use glam::{Quat, Vec3};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelSceneCamera {
    pub id: u32,
    pub scene_id: u32,
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub zoom_distance: f32,
    pub min_zoom_distance: f32,
    pub max_zoom_distance: f32,
}

impl ModelSceneCamera {
    pub fn zoom_percent(&self) -> f32 {
        if self.min_zoom_distance == self.max_zoom_distance {
            return 0.;
        }
        ((self.zoom_distance - self.min_zoom_distance)
            / (self.max_zoom_distance - self.min_zoom_distance))
            .clamp(0., 1.)
    }

    pub fn zoom_distance_at(&self, percent: f32) -> f32 {
        self.min_zoom_distance
            + (self.max_zoom_distance - self.min_zoom_distance) * percent.clamp(0., 1.)
    }

    pub fn orientation(&self, yaw_offset: f32) -> Quat {
        engine_rotation([self.yaw + yaw_offset, self.pitch, self.roll])
    }

    pub fn vectors(&self, yaw_offset: f32, zoom_percent: f32) -> (Vec3, Vec3, Vec3) {
        // OrbitCameraMixin: eye = target - forward * zoom distance.
        let rotation = self.orientation(yaw_offset);
        let target = engine_vector(self.target);
        let eye = target - rotation * Vec3::X * self.zoom_distance_at(zoom_percent);
        (eye, target, rotation * Vec3::Y)
    }
}

fn round_decimal(value: f32) -> f32 {
    (value * 10.).round() / 10.
}

/// TranslateDisplayInfo uses the first camera, including a product's `override` tag.
/// MathUtil.RoundToSignificantDigits(value, 1) rounds decimal places; angles round
/// in degrees before Model.lua converts them back to radians.
pub fn read_perks_camera(root: &Path, scene_id: u32) -> Result<ModelSceneCamera, String> {
    let mut camera = None;
    read_db2_rows(&root.join("db2/active/UiModelSceneCamera.csv"), |row| {
        if row.parse::<u32>("UiModelSceneID")? != scene_id || camera.is_some() {
            return Ok(());
        }
        let camera_type = row.parse::<u32>("CameraType")?;
        if camera_type != 0 {
            return Err(format!(
                "UiModelScene {scene_id} camera type {camera_type} is not OrbitCamera"
            ));
        }
        camera = Some(ModelSceneCamera {
            id: row.parse("ID")?,
            scene_id,
            target: [
                round_decimal(row.parse("Target_0")?),
                round_decimal(row.parse("Target_1")?),
                round_decimal(row.parse("Target_2")?),
            ],
            yaw: round_decimal(row.parse::<f32>("Yaw")?.to_degrees()).to_radians(),
            pitch: round_decimal(row.parse::<f32>("Pitch")?.to_degrees()).to_radians(),
            roll: round_decimal(row.parse::<f32>("Roll")?.to_degrees()).to_radians(),
            zoom_distance: row.parse("ZoomDistance")?,
            min_zoom_distance: row.parse("MinZoomDistance")?,
            max_zoom_distance: row.parse("MaxZoomDistance")?,
        });
        Ok(())
    })?;
    camera.ok_or_else(|| format!("UiModelScene {scene_id} has no authored camera"))
}
