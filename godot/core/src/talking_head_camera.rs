//! Source model transform and fixed camera from70448 ModelFrames.lua.
//! Native PlayerModel projection and flags→useModelCenter remain unsourced.
use std::path::Path;

pub const MODEL_PROJECTION_GAP: &str = "TALKING_HEAD_MODEL_PROJECTION_UNSOURCED";

#[derive(Clone, Debug, PartialEq)]
pub struct TalkingHeadCamera {
    pub id: u32,
    pub model_position: [f32; 3],
    pub yaw_pitch_roll: [f32; 3],
    pub camera_position: [f32; 3],
    pub camera_target: [f32; 3],
    pub flags: u32,
}

pub fn read_camera(root: &Path, id: u32) -> Result<TalkingHeadCamera, String> {
    let mut camera = None;
    crate::csv_util::read_db2_rows(&root.join("db2/active/UiCamera.csv"), |row| {
        if row.parse::<u32>("ID")? != id {
            return Ok(());
        }
        camera = Some(TalkingHeadCamera {
            id,
            model_position: [
                row.parse("Pos_0")?,
                row.parse("Pos_1")?,
                row.parse("Pos_2")?,
            ],
            yaw_pitch_roll: [
                row.parse("LookAt_0")?,
                row.parse("LookAt_1")?,
                row.parse("LookAt_2")?,
            ],
            // ModelFrames.lua70–71: camera space, not a fitted model-space eye.
            camera_position: [4.0, 0.0, 0.0],
            camera_target: [0.0, 0.0, 0.0],
            flags: row.parse("Flags")?,
        });
        Ok(())
    })?;
    camera.ok_or_else(|| format!("UiCamera{id} has no source transform"))
}
