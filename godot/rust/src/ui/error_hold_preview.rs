//! Supplied elapsed time through UIErrorsData and production screen; no wall-clock claim.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::{
    ui_errors_data::UiErrorsData, ui_errors_frame_component::ui_errors_frame_screen,
};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_error_hold_preview(&mut self, forever: bool, elapsed: f32, repeat: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let mut errors = UiErrorsData::default();
            errors.add("Out of range.");
            errors.add_info("Blackrock Worg slain: 1/6");
            errors.tick(elapsed);
            if repeat {
                errors.add("Out of range.");
                errors.add_info("Blackrock Worg slain: 1/6");
            }
            self.show_viewport_screen(errors, ui_errors_frame_screen, ScreenPostsetup::None)?;
            Ok::<(), String>(())
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn tick_error_hold_preview(&mut self, elapsed: f32) -> GString {
        self.tick_errors(elapsed)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn clear_error_hold_preview(&mut self) -> GString {
        self.clear_errors()
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn add_error_hold_preview(&mut self) -> GString {
        self.add_error("Out of range.")
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
}
