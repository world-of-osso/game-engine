//! EncounterJournal ModelScene initial pose; no Products.TranslateDisplayInfo rounding.
use crate::{
    csv_util::read_db2_rows,
    model_scene_actor::{ModelSceneActor, read_scene_actors},
    model_scene_camera::ModelSceneCamera,
};
use std::path::Path;

pub fn read_model_scene(
    root: &Path,
    scene_id: u32,
) -> Result<(ModelSceneActor, ModelSceneCamera), String> {
    Ok((
        read_creature_actor(root, scene_id)?,
        read_camera(root, scene_id)?,
    ))
}

/// Bounded Journal support: plain source clips25/26/27/213 without override metadata.
/// Other authored animation configurations retain their existing explicit coverage gaps.
pub fn read_plain_animation(root: &Path, actor_id: u32) -> Result<Option<u16>, String> {
    let db2 = root.join("db2/active");
    let mut display_id = None;
    read_db2_rows(&db2.join("UiModelSceneActor.csv"), |row| {
        if row.parse::<u32>("ID")? == actor_id {
            display_id = Some(row.parse::<u32>("UiModelSceneActorDisplayID")?);
        }
        Ok(())
    })?;
    let display_id = display_id.ok_or_else(|| format!("Journal actor {actor_id} missing"))?;
    if display_id == 0 {
        return Ok(None);
    }
    let mut animation = None;
    read_db2_rows(&db2.join("UiModelSceneActorDisplay.csv"), |row| {
        if row.parse::<u32>("ID")? == display_id {
            let clip = row.parse::<u16>("AnimationID")?;
            let plain = matches!(clip, 25 | 26 | 27 | 213)
                && row.parse::<u32>("AnimKitID")? == 0
                && row.parse::<u32>("SequenceVariation")? == 0
                && row.parse::<f32>("AnimSpeed")? == 1.
                && row.parse::<u32>("SpellVisualKitID")? == 0;
            animation = Some(plain.then_some(clip));
        }
        Ok(())
    })?;
    animation.ok_or_else(|| format!("Journal actor display {display_id} missing"))
}

fn read_creature_actor(root: &Path, scene_id: u32) -> Result<ModelSceneActor, String> {
    let mut default_actor = None;
    read_db2_rows(&root.join("db2/active/UiModelSceneActor.csv"), |row| {
        if row.parse::<u32>("UiModelSceneID")? != scene_id || row.field("ScriptTag")? != "creature"
        {
            return Ok(());
        }
        if row.parse::<u32>("UiModelSceneActorDisplayID")? != 0 {
            return Ok(());
        }
        // ModelSceneActorMixin ApplyFromModelSceneActorInfo's no-display arm:
        // requestedScale1, alpha1, animation0; authored actor transform survives.
        // This Journal-only branch leaves read_scene_actors behavior unchanged.
        default_actor = Some(ModelSceneActor {
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
            scale: 1.,
            alpha: 1.,
            animation: 0,
        });
        Ok(())
    })?;
    if let Some(actor) = default_actor {
        return Ok(actor);
    }
    read_scene_actors(root, scene_id)?
        .remove("creature")
        .ok_or_else(|| format!("Journal UiModelScene {scene_id} has no creature actor"))
}

fn read_camera(root: &Path, scene_id: u32) -> Result<ModelSceneCamera, String> {
    let mut camera = None;
    read_db2_rows(&root.join("db2/active/UiModelSceneCamera.csv"), |row| {
        if row.parse::<u32>("UiModelSceneID")? != scene_id || camera.is_some() {
            return Ok(());
        }
        let camera_type = row.parse::<u32>("CameraType")?;
        if camera_type != 0 {
            return Err(format!(
                "Journal UiModelScene {scene_id} camera type {camera_type} is not OrbitCamera"
            ));
        }
        let mut initial = ModelSceneCamera {
            id: row.parse("ID")?,
            scene_id,
            target: [
                row.parse("Target_0")?,
                row.parse("Target_1")?,
                row.parse("Target_2")?,
            ],
            yaw: row.parse("Yaw")?,
            pitch: row.parse("Pitch")?,
            roll: row.parse("Roll")?,
            zoom_distance: row.parse("ZoomDistance")?,
            min_zoom_distance: row.parse("MinZoomDistance")?,
            max_zoom_distance: row.parse("MaxZoomDistance")?,
        };
        // OrbitCameraMixin GetDerivedTarget/Orientation evaluates its two-point
        // zoom splines at 1-zoomPercent. PercentageBetween(min==max) returns0.
        // Journal installs the immediate initial pose; it has no zoom interaction.
        let endpoint = 1. - initial.zoom_percent();
        for (target, column) in initial.target.iter_mut().zip([
            "ZoomedTargetOffset_0",
            "ZoomedTargetOffset_1",
            "ZoomedTargetOffset_2",
        ]) {
            *target += row.parse::<f32>(column)? * endpoint;
        }
        initial.yaw += row.parse::<f32>("ZoomedYawOffset")? * endpoint;
        initial.pitch += row.parse::<f32>("ZoomedPitchOffset")? * endpoint;
        initial.roll += row.parse::<f32>("ZoomedRollOffset")? * endpoint;
        camera = Some(initial);
        Ok(())
    })?;
    camera.ok_or_else(|| format!("Journal UiModelScene {scene_id} has no authored camera"))
}
