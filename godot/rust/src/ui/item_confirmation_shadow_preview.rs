//! Supplied item-confirmation states through the production presenter/projection.
use super::{RegistryUi, ScreenPostsetup, party_preview, register_auction_popup_style};
use game_engine_ui_model::static_popup_component::{ItemConfirmation, item_confirmation};
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    screen::SharedContext,
    widget_def::Element,
};

#[derive(Clone)]
struct ConfirmationState {
    pending: bool,
    accept_enabled: bool,
}

fn confirmation_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ConfirmationState>()
        .expect("confirmation preview state");
    item_confirmation(ItemConfirmation {
        name: "ShadowConfirm",
        pending: state.pending,
        accept_enabled: state.accept_enabled,
        prompt: ("Purchase this item?", ""),
        currency_icon: None,
        item: Some(("Sharp Shortsword", 132350, 2)),
        accept: ("Confirm", "shadow_confirm:accept"),
        cancel_action: "shadow_confirm:cancel",
        screen: (1920.0, 1080.0),
    })
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_item_confirmation_shadow_preview(
        &mut self,
        forever: bool,
        pending: bool,
        accept_enabled: bool,
    ) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let parent = self.hud_parent()?;
            let mut registry = parent.registry();
            register_auction_popup_style(&mut registry);
            self.show_viewport_screen_in(
                ConfirmationState {
                    pending,
                    accept_enabled,
                },
                confirmation_screen,
                ScreenPostsetup::None,
                registry,
                parent,
            )
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
