//! Supplied PARTY_INVITE through its production spec and ordinary StaticPopup host.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::{popup::PopupStack, static_popup_component::StaticPopupState};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_party_invite_shadow_preview(&mut self, forever: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let mut stack = PopupStack::default();
            stack.push(crate::party_frames::invite_popup_spec("Adriel"));
            self.show_static_popups(StaticPopupState {
                popups: stack.visible(),
            })
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
