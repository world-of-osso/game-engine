//! Native display settings shared by startup and live Options edits.

use game_engine_core::client_options_data::{
    GraphicsOptionsFile, MAX_FRAME_RATE_LIMIT, MIN_FRAME_RATE_LIMIT,
};
use godot::classes::{DisplayServer, Engine, display_server::VSyncMode};
use godot::prelude::*;

pub(crate) fn apply_graphics_display_options(graphics: &GraphicsOptionsFile) {
    let vsync_mode = if graphics.vsync_enabled {
        VSyncMode::MAILBOX
    } else {
        VSyncMode::DISABLED
    };
    DisplayServer::singleton().window_set_vsync_mode(vsync_mode);

    let max_fps = if graphics.frame_rate_limit_enabled {
        i32::from(
            graphics
                .frame_rate_limit
                .clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT),
        )
    } else {
        0
    };
    Engine::singleton().set_max_fps(max_fps);
}
