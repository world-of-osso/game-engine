//! Native host for the source conversation payload and Retail TalkingHead frame.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{talking_head::TalkingHeadState, talking_head_component};
use godot::{
    classes::{InputEvent, InputEventMouseButton},
    global::MouseButton,
    prelude::*,
};
use shared::protocol::ConversationUpdate;

pub(crate) mod camera;
pub(crate) mod voice;

#[derive(Default)]
pub(crate) struct TalkingHeadHud {
    state: TalkingHeadState,
    pub(super) ui: Option<Gd<RegistryUi>>,
    voice: voice::TalkingHeadVoice,
}

impl GameClient {
    pub(crate) fn receive_talking_head(
        &mut self,
        update: ConversationUpdate,
    ) -> Result<(), String> {
        if self.world_map_id != Some(update.map_id) {
            return Err(format!(
                "conversation map{} is not the current world map",
                update.map_id
            ));
        }
        let owner = self
            .world
            .local_player_id()
            .ok_or("conversation arrived before local player identity")?;
        self.talking_head.state.receive(update, owner)
    }

    pub(crate) fn update_talking_head(&mut self, delta: f32) -> Result<(), FrameError> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        if !in_world {
            self.talking_head.state.clear();
        }
        self.talking_head.state.advance(delta);
        if let Some(mut ui) = self.talking_head.ui.clone() {
            consume_actions(&mut self.talking_head.state, &mut ui.bind_mut());
        }
        if self.talking_head.ui.is_none() && self.talking_head.state.current().is_some() {
            self.attach_talking_head()?;
        }
        if let Some(ui) = &mut self.talking_head.ui {
            ui.set_visible(in_world);
            ui.bind_mut().set_state(self.talking_head.state.clone())?;
            camera::sync_source_camera(
                ui,
                self.talking_head.state.current().as_ref(),
                &self.data_root,
            )?;
        }
        let mut parent = self.base().clone().upcast();
        self.talking_head.voice.sync(
            self.talking_head.state.current().as_ref(),
            &mut parent,
            &self.data_root,
            &self.client_options.sound,
        )?;
        Ok(())
    }

    fn attach_talking_head(&mut self) -> Result<(), String> {
        self.extract_art(&crate::quests::screen_texture_fdids(
            self.talking_head.state.clone(),
            talking_head_component::talking_head_screen,
        ));
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("TalkingHeadUI");
        self.base_mut().add_child(&ui);
        let shown = ui
            .bind_mut()
            .show_talking_head(self.talking_head.state.clone());
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.talking_head.ui = Some(ui);
        Ok(())
    }

    pub(crate) fn talking_head_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        dismiss_pointer(
            &mut self.talking_head.state,
            self.talking_head.ui.as_ref(),
            event,
        )
    }
}

pub(crate) fn consume_actions(state: &mut TalkingHeadState, ui: &mut RegistryUi) {
    loop {
        let action = ui.pop_action().to_string();
        if action.is_empty() {
            break;
        }
        if action == talking_head_component::DISMISS {
            state.dismiss();
        }
    }
}

pub(crate) fn dismiss_pointer(
    state: &mut TalkingHeadState,
    ui: Option<&Gd<RegistryUi>>,
    event: &Gd<InputEvent>,
) -> bool {
    if state.presentation().is_none() {
        return false;
    }
    let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
        return false;
    };
    if button.get_button_index() != MouseButton::RIGHT || button.is_pressed() {
        return false;
    }
    let Some(rect) = ui.and_then(|ui| ui.bind().frame_viewport_rect("TalkingHeadFrame")) else {
        return false;
    };
    let bounds = Rect2::new(
        Vector2::new(rect[0], rect[1]),
        Vector2::new(rect[2], rect[3]),
    );
    if !bounds.contains_point(button.get_position()) {
        return false;
    }
    state.dismiss();
    true
}
