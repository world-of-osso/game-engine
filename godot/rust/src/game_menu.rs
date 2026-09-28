//! Main-menu overlay and native Options controller; the underlying scene stays attached.

use game_engine_core::{
    client_options_data::{load_options_file_with_legacy, save_options_file_to_path},
    input_bindings_data::InputBinding,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    game_menu_component::GameMenuView,
    game_menu_main::{
        ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    },
    options_menu_component::{ACTION_OPTIONS_DEFAULTS, ACTION_OPTIONS_OKAY, OptionsCategory},
    options_menu_data::{self as policy, BindingCapture, OptionsModel},
};
use godot::{classes::InputEvent, prelude::*};

use crate::{GameClient, ui::RegistryUi};

impl GameClient {
    pub(super) fn open_game_menu(&mut self) -> Result<(), String> {
        if self.game_menu_ui.is_some() {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GameMenuUI");
        ui.set_layer(10);
        self.base_mut().add_child(&ui);
        let model = self.new_game_menu_model();
        let result = ui
            .bind_mut()
            .show_game_menu_view(policy::build_view_model(&model));
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.gui_release_focus();
        }
        self.physical_input.clear();
        self.game_menu_options = Some(model);
        self.game_menu_ui = Some(ui);
        Ok(())
    }

    fn new_game_menu_model(&self) -> OptionsModel {
        let file = &self.client_options;
        let graphics = policy::graphics_draft_from_file(&file.graphics);
        let sound = policy::sound_draft_from_file(&file.sound);
        let camera = policy::camera_draft_from_file(&file.camera);
        let hud = policy::hud_draft_from_file(&file.hud);
        OptionsModel {
            logged_in: self.account.session.screen != SessionScreen::GameMenu,
            view: GameMenuView::MainMenu,
            category: OptionsCategory::Sound,
            modal_position: file.modal_offset.unwrap_or([0.0, 0.0]),
            draft_graphics: graphics.clone(),
            draft_sound: sound.clone(),
            draft_camera: camera.clone(),
            draft_hud: hud.clone(),
            committed_graphics: graphics,
            committed_sound: sound,
            committed_camera: camera,
            committed_hud: hud,
            draft_bindings: file.bindings.clone(),
            committed_bindings: file.bindings.clone(),
            binding_section: game_engine_core::input_bindings_data::BindingSection::Movement,
            binding_capture: BindingCapture::None,
        }
    }

    pub(super) fn close_game_menu(&mut self) {
        if let Some(ui) = self.game_menu_ui.take() {
            ui.free();
            self.game_menu_options = None;
            self.physical_input.clear();
        }
    }

    pub(super) fn handle_game_menu_key(&mut self, key: godot::global::Key) -> Result<bool, String> {
        if self.game_menu_ui.is_some() {
            if key == godot::global::Key::ESCAPE {
                let model = self
                    .game_menu_options
                    .as_mut()
                    .expect("menu has options model");
                if model.binding_capture != BindingCapture::None {
                    model.binding_capture = BindingCapture::None;
                    self.refresh_game_menu()?;
                } else if model.view == GameMenuView::Options {
                    model.view = GameMenuView::MainMenu;
                    self.refresh_game_menu()?;
                } else {
                    self.close_game_menu();
                }
            }
            return Ok(true);
        }
        if key != godot::global::Key::ESCAPE
            || self.account.session.screen != SessionScreen::InWorld
            || !self.account.session.gameplay_input_allowed()
        {
            return Ok(false);
        }
        self.open_game_menu()?;
        Ok(true)
    }

    pub(super) fn capture_game_menu_binding(&mut self, event: &Gd<InputEvent>) -> bool {
        let Some(model) = self.game_menu_options.as_mut() else {
            return false;
        };
        let BindingCapture::Listening(action) = model.binding_capture else {
            return false;
        };
        if let Ok(key) = event.clone().try_cast::<godot::classes::InputEventKey>() {
            if !key.is_pressed() || key.is_echo() {
                return true;
            }
            if key.get_keycode() == godot::global::Key::ESCAPE {
                return false;
            }
            if let Some(binding) = crate::input_keys::binding_key(key.get_physical_keycode()) {
                model
                    .draft_bindings
                    .assign(action, InputBinding::Keyboard(binding));
                model.binding_capture = BindingCapture::None;
                if let Err(error) = self
                    .commit_game_menu_options()
                    .and_then(|()| self.refresh_game_menu())
                {
                    godot_error!("Cannot save captured menu binding: {error}");
                }
            }
            return true;
        }
        if let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        {
            if button.is_pressed() {
                if let Some(binding) =
                    crate::input_keys::binding_mouse_button(button.get_button_index())
                {
                    model
                        .draft_bindings
                        .assign(action, InputBinding::Mouse(binding));
                    model.binding_capture = BindingCapture::None;
                    if let Err(error) = self
                        .commit_game_menu_options()
                        .and_then(|()| self.refresh_game_menu())
                    {
                        godot_error!("Cannot save captured menu binding: {error}");
                    }
                }
            }
            return true;
        }
        false
    }

    fn refresh_game_menu(&mut self) -> Result<(), String> {
        let model = self
            .game_menu_options
            .as_ref()
            .expect("menu has options model");
        self.game_menu_ui
            .as_mut()
            .expect("menu has UI")
            .bind_mut()
            .set_game_menu_view(policy::build_view_model(model))
    }

    fn commit_game_menu_options(&mut self) -> Result<(), String> {
        let snapshot = policy::apply_snapshot(
            self.game_menu_options
                .as_mut()
                .expect("menu has options model"),
        );
        let path = self.data_root.join("ui/options_settings.ron");
        // Re-read externally owned realm/EULA before each write, not just at startup.
        let mut file = load_options_file_with_legacy(&path);
        policy::apply_graphics_file_snapshot(&mut file.graphics, &snapshot.graphics);
        policy::apply_sound_file_snapshot(&mut file.sound, &snapshot.sound);
        policy::apply_camera_file_snapshot(&mut file.camera, &snapshot.camera);
        policy::apply_hud_file_snapshot(&mut file.hud, &snapshot.hud);
        file.bindings = snapshot.bindings;
        file.modal_offset = Some(snapshot.modal_position);
        save_options_file_to_path(&path, &file)?;
        self.client_options = file;
        crate::display_options::apply_graphics_display_options(&self.client_options.graphics);
        Ok(())
    }

    fn dispatch_options_action(&mut self, action: &str) -> Result<bool, String> {
        let model = self
            .game_menu_options
            .as_mut()
            .expect("menu has options model");
        let mut changed = false;
        let mut committed = false;
        if let Some(category) = policy::parse_category_action(action) {
            model.category = category;
            changed = true;
        } else if let Some(section) = policy::parse_binding_section_action(action) {
            model.binding_section = section;
            model.binding_capture = BindingCapture::None;
            changed = true;
        } else if let Some(binding) = policy::parse_binding_rebind_action(action) {
            model.binding_capture = BindingCapture::Listening(binding);
            changed = true;
        } else if let Some(binding) = policy::parse_binding_clear_action(action) {
            model.draft_bindings.clear(binding);
            model.binding_capture = BindingCapture::None;
            changed = true;
            committed = true;
        } else if let Some((field, delta)) = policy::parse_step_action(action) {
            policy::apply_step(field, delta, model);
            changed = true;
            committed = true;
        } else if let Some(field) = policy::parse_toggle_action(action) {
            changed = policy::apply_toggle(field, model);
            committed = changed;
        } else if action == ACTION_OPTIONS_DEFAULTS {
            policy::reset_category_defaults(model);
            changed = true;
            committed = true;
        }
        if committed {
            self.commit_game_menu_options()?;
        }
        if changed {
            self.refresh_game_menu()?;
        }
        Ok(changed)
    }

    pub(super) fn poll_game_menu_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.game_menu_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let sliders = ui.bind_mut().drain_slider_events();
        for slider in sliders {
            if let Some(field) = policy::parse_slider_action(&slider.action) {
                policy::apply_slider_value(
                    field,
                    slider.value,
                    self.game_menu_options
                        .as_mut()
                        .expect("menu has options model"),
                );
                self.commit_game_menu_options()?;
                self.refresh_game_menu()?;
            }
        }
        loop {
            let action = self
                .game_menu_ui
                .as_mut()
                .expect("menu has UI")
                .bind_mut()
                .pop_action()
                .to_string();
            if action.is_empty() {
                break;
            }
            if self.dispatch_options_action(&action)? {
                continue;
            }
            match action.as_str() {
                ACTION_RESUME | ACTION_OPTIONS_OKAY => {
                    self.close_game_menu();
                    break;
                }
                ACTION_EXIT => self.base().get_tree().quit(),
                ACTION_SUPPORT => godot_print!("menu_support: placeholder"),
                ACTION_LOGOUT => self.request_logout()?,
                ACTION_OPTIONS | ACTION_ADDONS => {
                    let model = self
                        .game_menu_options
                        .as_mut()
                        .expect("menu has options model");
                    model.view = GameMenuView::Options;
                    if action == ACTION_ADDONS {
                        model.category = OptionsCategory::SocialAddons;
                    }
                    self.refresh_game_menu()?;
                }
                _ => return Err(format!("Unknown game menu action: {action}")),
            }
        }
        Ok(())
    }
}
