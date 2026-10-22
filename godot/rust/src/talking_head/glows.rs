//! Axis-specific source animation stays local to TalkingHead, not a toolkit API.
use crate::ui::RegistryUi;
use game_engine_ui_model::talking_head::{TalkingHeadState, glows};
use godot::prelude::*;

pub(crate) fn sync(ui: &Gd<RegistryUi>, state: &TalkingHeadState) -> Result<(), String> {
    let Some(paint) = state.presentation() else {
        return Ok(());
    };
    for glow in glows::sample(paint.main_elapsed_ms) {
        let mut region = ui
            .bind()
            .frame_control(glow.name)
            .ok_or_else(|| format!("TalkingHead source glow {} is absent", glow.name))?;
        let origin = Vector2::new(glow.origin[0], glow.origin[1]);
        let size = region.get_size();
        region.set_pivot_offset(size * origin);
        region.set_scale(Vector2::new(glow.scale[0], glow.scale[1]));
    }
    Ok(())
}
