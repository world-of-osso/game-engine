use game_engine_core::client_options_data::{MAX_UI_SCALE, MIN_UI_SCALE};

/// Match the legacy camera's fit rule without coupling the native host to Bevy.
pub(crate) fn effective_ui_scale(viewport: [f32; 2], user_scale: f32, in_world: bool) -> f32 {
    let user_scale = user_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE);
    if !in_world {
        return user_scale;
    }
    let fit = (viewport[0] / 1920.0)
        .min(viewport[1] / 1080.0)
        .max(2.0 / 3.0);
    fit * user_scale
}

#[cfg(test)]
mod tests {
    use super::effective_ui_scale;

    #[test]
    fn native_scale_matches_in_world_fit_and_non_world_user_scale() {
        for (viewport, user, in_world, expected) in [
            ([1280.0, 720.0], 0.75, true, 0.5),
            ([1280.0, 720.0], 1.25, true, 5.0 / 6.0),
            ([2560.0, 1440.0], 0.75, true, 1.0),
            ([2560.0, 1440.0], 1.25, true, 5.0 / 3.0),
            ([1024.0, 600.0], 1.0, true, 2.0 / 3.0),
            ([2560.0, 1440.0], 0.75, false, 0.75),
            ([2560.0, 1440.0], 1.25, false, 1.25),
            ([1280.0, 720.0], 0.1, false, 0.75),
            ([1280.0, 720.0], 5.0, false, 1.5),
        ] {
            let actual = effective_ui_scale(viewport, user, in_world);
            assert!(
                (actual - expected).abs() < 0.0001,
                "{viewport:?} {user} {in_world}: {actual} != {expected}"
            );
        }
    }
}
