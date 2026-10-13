//! Ephemeral Options popup routing, owned by this canvas, not saved client settings.
use super::{RegistryUi, UiInput};
use game_engine_ui_model::game_menu_component::{GameMenuView, GameMenuViewModel};
use game_engine_ui_model::options_dropdown::OptionsDropdownState;
use godot::classes::{InputEvent, InputEventKey, InputEventMouseButton};
use godot::prelude::*;

impl RegistryUi {
    fn options_dropdown_state(&self) -> OptionsDropdownState {
        self.model
            .as_ref()
            .and_then(|model| model.shared.get::<OptionsDropdownState>())
            .cloned()
            .unwrap_or_default()
    }
    fn update_options_dropdown(&mut self, state: OptionsDropdownState) -> Result<(), String> {
        self.model
            .as_mut()
            .ok_or("Options canvas missing")?
            .shared
            .insert(state);
        self.sync_model()
    }
    pub(super) fn handle_options_dropdown_click(
        &mut self,
        event: &UiInput,
    ) -> Result<bool, String> {
        let (UiInput::Click(id) | UiInput::FrameClick { id, .. }) = event else {
            return Ok(false);
        };
        let Some(model) = self.model.as_mut() else {
            return Ok(false);
        };
        if !model
            .shared
            .get::<GameMenuViewModel>()
            .is_some_and(|view| view.view == GameMenuView::Options)
        {
            return Ok(false);
        }
        let Some(action) = model.click_action(*id) else {
            return Ok(false);
        };
        let mut state = self.options_dropdown_state();
        if let Some(key) = action.strip_prefix("options_dropdown:") {
            let (button, _) = self
                .frame_rect(&format!("Choice{key}"))
                .ok_or("Options dropdown button has no layout")?;
            let (root, _) = self
                .frame_rect("OptionsRoot")
                .ok_or("Options root has no layout")?;
            let scale = self
                .model
                .as_ref()
                .ok_or("Options canvas missing")?
                .registry
                .ui_scale;
            state.toggle(
                key,
                [
                    (button[0] - root[0]) / scale,
                    (button[1] + button[3] - root[1]) / scale,
                ],
            );
            self.update_options_dropdown(state)?;
            return Ok(true);
        }
        if state.menu.is_some() && action.starts_with("options_") {
            state.close();
            // Queue before rebuilding: the selected radio node is removed with its popup.
            self.actions.push_back(action);
            self.update_options_dropdown(state)?;
            return Ok(true);
        }
        Ok(false)
    }
    /// Escape is consumed; outside clicks close the popup but retain normal event routing.
    pub(super) fn handle_options_dropdown_input(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, String> {
        let mut state = self.options_dropdown_state();
        let Some(menu) = state.menu.as_ref() else {
            return Ok(false);
        };
        if let Ok(key) = event.clone().try_cast::<InputEventKey>()
            && key.is_pressed()
            && key.get_keycode() == godot::global::Key::ESCAPE
        {
            state.close();
            self.update_options_dropdown(state)?;
            return Ok(true);
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>()
            && button.is_pressed()
        {
            let point = button.get_position();
            let popup = self
                .frame_viewport_rect(&format!("OptionsChoicePopup{}", menu.key))
                .ok_or("Open Options popup has no native rectangle")?;
            let dropdown = self
                .frame_viewport_rect(&format!("Choice{}", menu.key))
                .ok_or("Open Options dropdown has no native rectangle")?;
            let contains = |[x, y, w, h]: [f32; 4]| {
                Rect2::new(Vector2::new(x, y), Vector2::new(w, h)).contains_point(point)
            };
            if !contains(popup) && !contains(dropdown) {
                state.close();
                self.update_options_dropdown(state)?;
            }
        }
        Ok(false)
    }
}
