//! Supplied ReadyCheck listener state through production projection/input, not party authority.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::ready_check_frame_component::{ReadyCheckFrameState, ready_check_frame};
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    screen::SharedContext,
    widget_def::Element,
};

fn ready_check_screen(shared: &SharedContext) -> Element {
    ready_check_frame(shared.get::<ReadyCheckFrameState>().unwrap())
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_ready_check_preview(&mut self, forever: bool) -> GString {
        let result = (|| {
            if self.model.is_some() {
                return Err("RegistryUi already has a screen".into());
            }
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let parent = self.hud_parent()?;
            let mut registry = parent.registry();
            super::register_metal_frame_style(
                &mut registry,
                game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
            )?;
            self.show_viewport_screen_in(
                ReadyCheckFrameState {
                    visible: true,
                    initiator: "Raidleader".into(),
                    difficulty_name: None,
                },
                ready_check_screen,
                ScreenPostsetup::None,
                registry,
                parent,
            )?;
            self.enable_cursor_inputs();
            Ok::<(), String>(())
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn ready_check_preview_actions(&mut self) -> Array<GString> {
        self.drain_bag_inputs()
            .expect("ReadyCheck preview cursor must be initialized")
            .into_iter()
            .filter_map(|(_, input)| match input {
                crate::bag_cursor::BagInput::Click { action, .. } => Some(action.as_str().into()),
                crate::bag_cursor::BagInput::Release { .. } => None,
            })
            .collect()
    }
}
