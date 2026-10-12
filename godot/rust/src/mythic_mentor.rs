//! Built-in Keys window: server data only, native title drag and per-character placement.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi, world_map::WindowDrag};
use game_engine_ui_model::mythic_mentor::{self as model, MentorView};
use godot::{
    classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion},
    global::MouseButton,
    prelude::*,
};

#[derive(Default)]
pub(crate) struct MentorUi {
    ui: Option<Gd<RegistryUi>>,
    open: bool,
    position: Option<[f32; 2]>,
    drag: Option<WindowDrag>,
}
impl MentorUi {
    pub(crate) fn free(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.drag = None;
    }
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match &mut self.ui {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }
}
impl GameClient {
    pub(super) fn toggle_mythic_mentor(&mut self) -> Result<(), FrameError> {
        if self.close_mythic_mentor() {
            return Ok(());
        }
        let character = self
            .account
            .session
            .selected_character_id
            .ok_or("Keys requires a selected character")?;
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        let mentor = &mut self.challenge_ui.mentor;
        mentor.position =
            game_engine_core::ui_layout_data::window_position(&path, character, model::FRAME)?;
        mentor.open = true;
        self.update_mythic_mentor()
    }
    pub(super) fn close_mythic_mentor(&mut self) -> bool {
        let mentor = &mut self.challenge_ui.mentor;
        let was_open = mentor.open;
        mentor.open = false;
        mentor.free();
        was_open
    }
    pub(super) fn update_mythic_mentor(&mut self) -> Result<(), FrameError> {
        self.poll_mentor_actions()?;
        if !self.challenge_ui.mentor.open || self.account.party_keystones.members.is_empty() {
            self.challenge_ui.mentor.free();
            return Ok(());
        }
        let scale = self.effective_ui_scale();
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Keys viewport missing")?
            .get_visible_rect()
            .size
            / scale;
        let height = model::panel_height(self.account.party_keystones.members.len());
        let [x, y] = self.challenge_ui.mentor.position.unwrap_or([510.0, 104.0]);
        let view = MentorView {
            position: [
                x.clamp(0.0, (viewport.x - model::PANEL_WIDTH).max(0.0)),
                y.clamp(0.0, (viewport.y - height).max(0.0)),
            ],
            snapshot: self.account.party_keystones.clone(),
        };
        self.sync_mentor(view, scale)
    }
    fn sync_mentor(&mut self, view: MentorView, scale: f32) -> Result<(), FrameError> {
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            model::mentor_screen,
        ));
        if let Some(ui) = self.challenge_ui.mentor.ui.as_mut() {
            ui.bind_mut().set_state(view)?;
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("MythicMentorUI");
        ui.set_layer(10);
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_mythic_mentor(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error.into());
        }
        self.challenge_ui.mentor.ui = Some(ui);
        Ok(())
    }
    fn poll_mentor_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.challenge_ui.mentor.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == model::CLOSE_ACTION {
                self.close_mythic_mentor();
                break;
            }
            if let Some(id) = action.strip_prefix("mentor:teleport:") {
                let spell = id.parse::<u32>().map_err(|error| error.to_string())?;
                self.account.send_cast(spell, "", None, None)?;
            }
        }
        Ok(())
    }
    pub(super) fn mentor_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Some(ui) = self.challenge_ui.mentor.ui.clone() else {
            return false;
        };
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.move_mentor(&motion, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() {
            return self.release_mentor_drag();
        }
        let host = ui.bind();
        let Some(handle) = host.frame_control("MentorDragHandle") else {
            return false;
        };
        let Some((rect, _)) = host.frame_rect(model::FRAME) else {
            return false;
        };
        if !handle
            .get_global_rect()
            .contains_point(button.get_position())
        {
            return false;
        }
        drop(host);
        match self.ui_owns_point(&ui, button.get_position()) {
            Ok(true) => {
                self.challenge_ui.mentor.drag = Some(WindowDrag::begin(
                    button.get_position() / scale,
                    [rect[0], rect[1]],
                ));
                true
            }
            Ok(false) => false,
            Err(error) => {
                godot_error!("Keys title hit-test: {error}");
                false
            }
        }
    }
    fn move_mentor(&mut self, motion: &Gd<InputEventMouseMotion>, scale: f32) -> bool {
        let Some(drag) = &self.challenge_ui.mentor.drag else {
            return false;
        };
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let size = viewport.get_visible_rect().size / scale;
        let height = model::panel_height(self.account.party_keystones.members.len());
        self.challenge_ui.mentor.position = Some(drag.position(
            motion.get_position() / scale,
            [size.x, size.y],
            [model::PANEL_WIDTH, height],
        ));
        if let Err(error) = self.update_mythic_mentor() {
            godot_error!("Keys drag: {error}");
        }
        true
    }
    fn release_mentor_drag(&mut self) -> bool {
        if self.challenge_ui.mentor.drag.take().is_none() {
            return false;
        }
        let Some(character) = self.account.session.selected_character_id else {
            return false;
        };
        let Some(position) = self.challenge_ui.mentor.position else {
            return false;
        };
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        if let Err(error) = game_engine_core::ui_layout_data::save_window_position(
            &path,
            character,
            model::FRAME,
            position,
        ) {
            godot_error!("Keys placement: {error}");
        }
        true
    }
}
#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn party_keystone_state(&self) -> GString {
        GString::from(
            &serde_json::to_string(&self.account.party_keystones).expect("serialize party keys"),
        )
    }
}
