//! Host the built-in Keys screen with the same flat-frame art in either skin.
use super::{RegistryUi, ScreenPostsetup, register_metal_frame_style};
use game_engine_ui_model::{
    mythic_mentor::{MentorView, mentor_screen},
    panel_style_data::MetalTopLeft,
};
impl RegistryUi {
    pub(crate) fn show_mythic_mentor(&mut self, view: MentorView) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(&mut registry, MetalTopLeft::Plain)?;
        self.toplevel = true;
        self.show_viewport_screen_in(view, mentor_screen, ScreenPostsetup::None, registry, parent)
    }
}
