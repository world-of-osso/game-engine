//! Preserve the authored PlayerModel transform without inventing its projection.
//! No model/camera render nodes are created while the named source gap is open.
use std::path::Path;

use game_engine_core::talking_head_camera::{MODEL_PROJECTION_GAP, read_camera};
use game_engine_ui_model::talking_head::TalkingHeadView;
use godot::prelude::*;

use crate::ui::RegistryUi;

pub(crate) fn sync_source_camera(
    ui: &Gd<RegistryUi>,
    view: Option<&TalkingHeadView>,
    data_root: &Path,
) -> Result<(), String> {
    let Some(view) = view else {
        return Ok(());
    };
    let mut host = ui
        .bind()
        .frame_control("TalkingHeadModel")
        .ok_or("TalkingHead source model host is absent")?;
    if host.has_meta("source_ui_camera")
        && host.get_meta("source_ui_camera").to::<i64>() == i64::from(view.camera_id)
    {
        return Ok(());
    }
    let camera = read_camera(data_root, view.camera_id)?;
    host.set_meta("source_ui_camera", &i64::from(camera.id).to_variant());
    host.set_meta(
        "model_position",
        &vector(camera.model_position).to_variant(),
    );
    host.set_meta(
        "yaw_pitch_roll",
        &vector(camera.yaw_pitch_roll).to_variant(),
    );
    host.set_meta(
        "camera_position",
        &vector(camera.camera_position).to_variant(),
    );
    host.set_meta("camera_target", &vector(camera.camera_target).to_variant());
    host.set_meta("source_flags", &i64::from(camera.flags).to_variant());
    host.set_meta("model_gap", &MODEL_PROJECTION_GAP.to_variant());
    godot_warn!(
        "{MODEL_PROJECTION_GAP}: UiCamera{} transform retained; no model rendered",
        camera.id
    );
    Ok(())
}

fn vector([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, y, z)
}

#[cfg(debug_assertions)]
pub(crate) fn camera_state(ui: Option<&Gd<RegistryUi>>) -> VarDictionary {
    let mut state = VarDictionary::new();
    let Some(host) = ui.and_then(|ui| ui.bind().frame_control("TalkingHeadModel")) else {
        return state;
    };
    for key in [
        "source_ui_camera",
        "model_position",
        "yaw_pitch_roll",
        "camera_position",
        "camera_target",
        "source_flags",
        "model_gap",
    ] {
        if host.has_meta(key) {
            state.set(key, &host.get_meta(key));
        }
    }
    state
}
