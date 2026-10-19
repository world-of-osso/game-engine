//! UiModelSceneActor/Display and SharedXML ModelSceneActorMixin actor transforms.
use crate::csv_util::read_db2_rows;
use glam::{Quat, Vec3};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug)]
pub struct ModelSceneActor {
    pub id: u32,
    pub flags: u32,
    pub position: [f32; 3],
    pub orientation: [f32; 3],
    pub normalized_scale: f32,
    pub scale: f32,
    pub alpha: f32,
    pub animation: u16,
}

impl ModelSceneActor {
    pub fn actor_scale(&self, size: Vec3) -> f32 {
        self.normalized_scale_of(self.scale, size)
    }
    /// This actor's normalization applied to another requested scale.
    pub fn normalized_scale_of(&self, requested: f32, size: Vec3) -> f32 {
        normalized_actor_scale(requested, self.normalized_scale, size)
    }
    pub fn actor_rotation(&self) -> Quat {
        engine_rotation(self.orientation)
    }
    pub fn actor_origin(&self, center: Vec3, scale: f32) -> Vec3 {
        actor_origin(self.flags, self.position, self.orientation, center, scale)
    }
}

pub fn read_scene_actors(
    root: &Path,
    scene_id: u32,
) -> Result<BTreeMap<String, ModelSceneActor>, String> {
    let db2 = root.join("db2/active");
    let mut displays = BTreeMap::new();
    read_db2_rows(&db2.join("UiModelSceneActorDisplay.csv"), |row| {
        displays.insert(
            row.parse::<u32>("ID")?,
            (
                row.parse::<f32>("Scale")?,
                row.parse::<f32>("Alpha")?,
                row.parse::<u16>("AnimationID")?,
            ),
        );
        Ok(())
    })?;
    let mut actors = BTreeMap::new();
    read_db2_rows(&db2.join("UiModelSceneActor.csv"), |row| {
        if row.parse::<u32>("UiModelSceneID")? != scene_id {
            return Ok(());
        }
        let display_id = row.parse::<u32>("UiModelSceneActorDisplayID")?;
        let &(scale, alpha, animation) = displays
            .get(&display_id)
            .ok_or_else(|| format!("UiModelSceneActorDisplay {display_id} missing"))?;
        let actor = ModelSceneActor {
            id: row.parse("ID")?,
            flags: row.parse("Flags")?,
            position: [
                row.parse("Position_0")?,
                row.parse("Position_1")?,
                row.parse("Position_2")?,
            ],
            orientation: [
                row.parse("OrientationYaw")?,
                row.parse("OrientationPitch")?,
                row.parse("OrientationRoll")?,
            ],
            normalized_scale: row.parse("NormalizedScale")?,
            scale,
            alpha,
            animation,
        };
        actors.insert(row.field("ScriptTag")?.to_owned(), actor);
        Ok(())
    })?;
    if actors.is_empty() {
        return Err(format!("UiModelScene {scene_id} has no actors"));
    }
    Ok(actors)
}

pub(crate) fn normalized_actor_scale(requested: f32, normalized: f32, size: Vec3) -> f32 {
    if normalized <= 0.0 {
        return requested;
    }
    // MODEL_SCENE_ACTOR_DIMENSIONS_FOR_NORMALIZATION, converted WoW XYZ -> engine XZY.
    let max_scale = (size.x / 0.9984).max(size.z / 1.121).max(size.y / 2.118);
    if max_scale == 0.0 {
        return requested;
    }
    requested / (1.0 + (max_scale - 1.0) * normalized)
}

pub(crate) fn actor_origin(
    flags: u32,
    position: [f32; 3],
    orientation: [f32; 3],
    center: Vec3,
    scale: f32,
) -> Vec3 {
    let centered = Vec3::new(
        if flags & 2 != 0 { center.x } else { 0.0 },
        if flags & 8 != 0 { center.y } else { 0.0 },
        if flags & 4 != 0 { center.z } else { 0.0 },
    );
    engine_vector(position) - engine_rotation(orientation) * centered * scale
}
pub(crate) fn engine_vector([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}
pub(crate) fn engine_rotation([yaw, pitch, roll]: [f32; 3]) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-pitch) * Quat::from_rotation_x(roll)
}
