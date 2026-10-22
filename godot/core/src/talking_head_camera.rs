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

pub fn read_camera(_root: &Path, id: u32) -> Result<TalkingHeadCamera, String> {
    Err(format!("UiCamera{id} source transform is not implemented"))
}
