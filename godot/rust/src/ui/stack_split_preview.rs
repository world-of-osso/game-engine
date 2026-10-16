//! Supplied StackSplit state through production screen/projection/input; no inventory mutation.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::stack_split_frame_component::{
    StackSplitFrameState, stack_split_frame_screen,
};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_stack_split_preview(&mut self, forever: bool, bundles: bool, enabled: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let state = StackSplitFrameState {
                visible: true,
                x: 874.0,
                y: 480.0,
                text: if bundles { "4 Stack(s)" } else { "4" }.into(),
                total_text: bundles.then(|| "20 Total".into()),
                left_enabled: enabled,
                right_enabled: enabled,
            };
            self.show_viewport_screen(state, stack_split_frame_screen, ScreenPostsetup::None)
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn stack_split_preview_actions(&mut self) -> Array<GString> {
        self.drain_bag_inputs()
            .expect("StackSplit preview cursor must be initialized")
            .into_iter()
            .filter_map(|(_, input)| match input {
                crate::bag_cursor::BagInput::Click { action, .. } => Some(action.as_str().into()),
                crate::bag_cursor::BagInput::Release { .. } => None,
            })
            .collect()
    }
}
