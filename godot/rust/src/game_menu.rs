//! Main-menu overlay and native Options controller; the underlying scene stays attached.

pub(crate) mod drag;

use game_engine_core::{
    client_options_data::{load_options_file_with_legacy, options_path, save_options_file_to_path},
    input_bindings_data::{InputAction, InputBinding},
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    game_menu_component::GameMenuView,
    game_menu_main::{
        ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    },
    options_menu_component::{
        ACTION_OPTIONS_DEFAULTS, ACTION_OPTIONS_OKAY, ACTION_RESET_WINDOW_POSITIONS,
        OPTIONS_DRAG_HANDLE, OptionsCategory,
    },
    options_menu_data::{self as policy, BindingCapture, OptionsModel},
};
use godot::{
    classes::{Control, InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion},
    prelude::*,
};

use crate::{GameClient, ui::RegistryUi};

fn keyboard_binding(key: &InputEventKey) -> Option<InputBinding> {
    let binding = crate::input_keys::binding_key(key.get_physical_keycode())?;
    Some(if key.is_ctrl_pressed() {
        InputBinding::CtrlKeyboard(binding)
    } else if key.is_shift_pressed() {
        InputBinding::ShiftKeyboard(binding)
    } else {
        InputBinding::Keyboard(binding)
    })
}

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
            logged_in: self.base().get_node_or_null("SkyboxDebug").is_none(),
            view: GameMenuView::MainMenu,
            category: OptionsCategory::Sound,
            modal_position: drag::initial_position(
                file.modal_offset,
                file.modal_position,
                self.game_menu_logical_viewport(),
            ),
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
            active_layout: self.ui_layout.name.clone(),
        }
    }

    pub(super) fn close_game_menu(&mut self) {
        self.game_menu_drag = None;
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
                    self.game_menu_drag = None;
                    model.view = GameMenuView::MainMenu;
                    self.refresh_game_menu()?;
                } else {
                    self.close_game_menu();
                }
            }
            return Ok(true);
        }
        if key != godot::global::Key::ESCAPE {
            return Ok(false);
        }
        let gameplay_menu_allowed = self.account.session.screen == SessionScreen::InWorld
            && self.account.session.gameplay_input_allowed();
        let offline_sky_present = self.base().get_node_or_null("SkyboxDebug").is_some();
        if !gameplay_menu_allowed && !offline_sky_present {
            return Ok(false);
        }
        self.open_game_menu()?;
        Ok(true)
    }

    pub(super) fn capture_game_menu_binding(&mut self, event: &Gd<InputEvent>) -> bool {
        let Some(model) = self.game_menu_options.as_ref() else {
            return false;
        };
        let BindingCapture::Listening(action) = model.binding_capture else {
            return false;
        };
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if !key.is_pressed() || key.is_echo() {
                return true;
            }
            if key.get_keycode() == godot::global::Key::ESCAPE {
                return false;
            }
            if let Some(binding) = keyboard_binding(&key) {
                self.assign_captured_game_menu_binding(action, binding);
            }
            return true;
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            if button.is_pressed()
                && let Some(binding) =
                    crate::input_keys::binding_mouse_button(button.get_button_index())
            {
                self.assign_captured_game_menu_binding(action, InputBinding::Mouse(binding));
            }
            return true;
        }
        false
    }

    fn assign_captured_game_menu_binding(&mut self, action: InputAction, binding: InputBinding) {
        let model = self
            .game_menu_options
            .as_mut()
            .expect("menu has options model");
        model.draft_bindings.assign(action, binding);
        model.binding_capture = BindingCapture::None;
        if let Err(error) = self
            .commit_game_menu_options()
            .and_then(|()| self.refresh_game_menu())
        {
            godot_error!("Cannot save captured menu binding: {error}");
        }
    }

    fn game_menu_logical_viewport(&self) -> Vector2 {
        let size = self
            .base()
            .get_viewport()
            .expect("menu viewport")
            .get_visible_rect()
            .size;
        let scale = crate::ui_scale::effective_ui_scale(
            [size.x, size.y],
            self.client_options.graphics.ui_scale,
            self.account.session.screen == SessionScreen::InWorld,
        );
        size / scale
    }

    fn drag_pointer_position(&self, position: Vector2) -> Result<(Vector2, Vector2), String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Options has no viewport")?;
        let menu = self.game_menu_ui.as_ref().ok_or("Options has no UI")?;
        let canvas = menu
            .find_child_ex("RegistryCanvas")
            .owned(false)
            .done()
            .ok_or("Options has no RegistryCanvas")?
            .try_cast::<Control>()
            .map_err(|_| "Options RegistryCanvas is not a Control")?;
        let scale = canvas.get_scale().x;
        if scale <= 0.0 {
            return Err("Options canvas scale must be positive".into());
        }
        Ok((position / scale, viewport.get_visible_rect().size / scale))
    }

    fn pointer_on_options_title(&self) -> bool {
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let Some(menu) = self.game_menu_ui.as_ref() else {
            return false;
        };
        let Some(title) = menu
            .find_child_ex(OPTIONS_DRAG_HANDLE.0)
            .owned(false)
            .done()
        else {
            return false;
        };
        let Some(mut hovered) = viewport
            .gui_get_hovered_control()
            .map(|node| node.upcast::<Node>())
        else {
            return false;
        };
        loop {
            if hovered == title {
                return true;
            }
            let Some(parent) = hovered.get_parent() else {
                return false;
            };
            hovered = parent;
        }
    }

    /// Consume title capture, motion, and release before other pointer routing.
    pub(super) fn handle_game_menu_pointer(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, String> {
        if self
            .game_menu_options
            .as_ref()
            .is_none_or(|model| model.view != GameMenuView::Options)
        {
            self.game_menu_drag = None;
            return Ok(false);
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            return self.handle_options_drag_button(&button);
        }
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.handle_options_drag_motion(&motion);
        }
        Ok(false)
    }

    fn handle_options_drag_button(
        &mut self,
        button: &InputEventMouseButton,
    ) -> Result<bool, String> {
        if button.get_button_index() != godot::global::MouseButton::LEFT {
            return Ok(false);
        }
        if button.is_pressed() {
            if !self.pointer_on_options_title() {
                return Ok(false);
            }
            let (cursor, viewport) = self.drag_pointer_position(button.get_position())?;
            let position = self
                .game_menu_options
                .as_ref()
                .expect("Options model")
                .modal_position;
            self.game_menu_drag = Some(drag::OptionsDrag::begin(cursor, position, viewport));
            return Ok(true);
        }
        if self.game_menu_drag.take().is_some() {
            self.save_game_menu_drag_position()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn handle_options_drag_motion(
        &mut self,
        motion: &InputEventMouseMotion,
    ) -> Result<bool, String> {
        let Some(drag) = self.game_menu_drag.as_ref() else {
            return Ok(false);
        };
        let (cursor, viewport) = self.drag_pointer_position(motion.get_position())?;
        let position = drag.position(cursor, viewport);
        self.game_menu_options
            .as_mut()
            .expect("Options model")
            .modal_position = position;
        self.refresh_game_menu()?;
        Ok(true)
    }

    fn save_game_menu_drag_position(&mut self) -> Result<(), String> {
        let position = self
            .game_menu_options
            .as_ref()
            .expect("Options model")
            .modal_position;
        let legacy_path = self.data_root.join("ui/options_settings.ron");
        let path = options_path();
        let mut file = load_options_file_with_legacy(&legacy_path);
        file.modal_offset = Some(position);
        file.modal_position = None;
        save_options_file_to_path(&path, &file)?;
        self.client_options = file;
        Ok(())
    }

    /// Switches the open game menu to its Options panel.
    pub(super) fn show_game_menu_options(&mut self) -> Result<(), String> {
        self.game_menu_options
            .as_mut()
            .ok_or("Options needs an open game menu")?
            .view = GameMenuView::Options;
        self.refresh_game_menu()
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
        let legacy_path = self.data_root.join("ui/options_settings.ron");
        let path = options_path();
        // Re-read externally owned realm/EULA before each write, not just at startup.
        let mut file = load_options_file_with_legacy(&legacy_path);
        policy::apply_graphics_file_snapshot(&mut file.graphics, &snapshot.graphics);
        policy::apply_sound_file_snapshot(&mut file.sound, &snapshot.sound);
        policy::apply_camera_file_snapshot(&mut file.camera, &snapshot.camera);
        policy::apply_hud_file_snapshot(&mut file.hud, &snapshot.hud);
        file.bindings = snapshot.bindings;
        file.modal_offset = Some(snapshot.modal_position);
        save_options_file_to_path(&path, &file)?;
        self.client_options = file;
        let density = f32::from(self.client_options.graphics.particle_density) / 100.0;
        self.world_objects.set_particle_density(density);
        self.world.set_particle_density(density);
        self.apply_display_options();
        Ok(())
    }

    fn dispatch_options_action(&mut self, action: &str) -> Result<bool, String> {
        if action == ACTION_RESET_WINDOW_POSITIONS {
            let path = options_path().with_file_name("ui_layout.ron");
            let character_id = self.account.session.selected_character_id;
            game_engine_core::ui_layout_data::reset_window_positions(&path, character_id)?;
            self.reset_open_world_map_position();
            self.reset_open_spellbook_position()?;
            self.reset_open_merchant_position()?;
            if self.world_map.is_open() {
                self.sync_world_map()?;
            }
            return Ok(true);
        }
        if let Some(name) = policy::parse_layout_action(action) {
            self.select_ui_layout(name)?;
            self.game_menu_options
                .as_mut()
                .expect("menu has options model")
                .active_layout = name.to_string();
            self.refresh_game_menu()?;
            return Ok(true);
        }
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
                ACTION_LOGOUT => {
                    self.request_logout()?;
                    break;
                }
                ACTION_OPTIONS | ACTION_ADDONS => {
                    if action == ACTION_ADDONS {
                        self.game_menu_options
                            .as_mut()
                            .expect("menu has options model")
                            .category = OptionsCategory::SocialAddons;
                    }
                    self.show_game_menu_options()?;
                }
                _ => return Err(format!("Unknown game menu action: {action}")),
            }
        }
        Ok(())
    }
}
