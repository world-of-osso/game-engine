//! Native in-world launcher lifecycle, keyboard ownership and shared destination dispatch.
use game_engine_core::input_bindings_data::{BindingKey, InputAction, InputBinding};
use game_engine_session::SessionScreen;
use game_engine_ui_model::launcher::{self as model, LauncherView, SEARCH_FIELD};
use godot::classes::{InputEvent, InputEventKey};
use godot::prelude::*;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

fn launcher_texture_fdids(view: LauncherView) -> Vec<u32> {
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::widgets::texture::TextureSource;
    let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(ui_toolkit::atlas::thread_skin());
    shared.insert(view);
    ui_toolkit::screen::Screen::new(model::launcher_screen).sync(&shared, &mut registry);
    let mut fdids: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                return None;
            };
            let TextureSource::Atlas(atlas) = &texture.source else {
                return None;
            };
            let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) =
                ui_toolkit::atlas::get_region(atlas)?.source
            else {
                return None;
            };
            Some(fdid)
        })
        .collect();
    fdids.extend(game_engine_ui_model::panel_style_data::metal_sheet_fdids(
        game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
    ));
    fdids.push(6_795_680); // Existing Modern diamond-dialog border.
    fdids
}

#[derive(Default)]
pub(crate) struct Launcher {
    pub view: LauncherView,
    pub ui: Option<Gd<RegistryUi>>,
}

impl GameClient {
    pub(super) fn open_launcher(&mut self) -> Result<(), String> {
        self.launcher.view.action(model::ACTION_OPEN);
        self.physical_input.clear_gameplay();
        self.sync_launcher()?;
        self.launcher
            .ui
            .as_mut()
            .ok_or("Launcher UI missing")?
            .bind_mut()
            .focus_frame_named(SEARCH_FIELD)
    }

    pub(super) fn close_launcher(&mut self) {
        self.launcher.view.open = false;
        if let Some(ui) = self.launcher.ui.take() {
            ui.free();
        }
        self.physical_input.clear_gameplay();
    }

    /// Navigation is observed before the focused LineEdit consumes Enter/arrow/Escape.
    pub(super) fn launcher_input(&mut self, event: &Gd<InputEvent>) -> Result<bool, FrameError> {
        if self.account.session.screen != SessionScreen::InWorld
            || !self.account.session.gameplay_input_allowed()
        {
            return Ok(false);
        }
        let Ok(key) = event.clone().try_cast::<InputEventKey>() else {
            return Ok(false);
        };
        let Some(binding_key) = crate::input_keys::binding_key(key.get_physical_keycode()) else {
            return Ok(false);
        };
        let toggle = self.launcher_toggle_key(binding_key, &key);
        if toggle && key.is_pressed() && !key.is_echo() {
            if self.launcher.view.open {
                self.close_launcher();
            } else if self.game_menu_ui.is_none() && self.keyboard_free() {
                self.open_launcher()?;
            } else {
                return Ok(false);
            }
            return Ok(true);
        }
        if !self.launcher.view.open {
            return Ok(false);
        }
        if toggle {
            return Ok(true);
        }
        self.launcher_navigation(binding_key, &key)
    }

    fn launcher_toggle_key(&self, key: BindingKey, event: &InputEventKey) -> bool {
        let binding = if event.is_ctrl_pressed() {
            InputBinding::CtrlKeyboard(key)
        } else if event.is_shift_pressed() {
            InputBinding::ShiftKeyboard(key)
        } else {
            InputBinding::Keyboard(key)
        };
        self.client_options
            .bindings
            .binding(InputAction::ToggleLauncher)
            == Some(binding)
    }

    fn launcher_navigation(
        &mut self,
        key: BindingKey,
        event: &InputEventKey,
    ) -> Result<bool, FrameError> {
        let navigation = matches!(
            key,
            BindingKey::Escape
                | BindingKey::Enter
                | BindingKey::ArrowLeft
                | BindingKey::ArrowRight
                | BindingKey::ArrowUp
                | BindingKey::ArrowDown
        );
        if !navigation {
            return Ok(false);
        }
        if !event.is_pressed()
            || (event.is_echo() && matches!(key, BindingKey::Escape | BindingKey::Enter))
        {
            return Ok(true);
        }
        self.poll_launcher_search()?;
        let action = self.launcher.view.handle_key(
            key,
            event.is_ctrl_pressed(),
            event.is_shift_pressed(),
            &self.client_options.bindings,
        );
        if !self.launcher.view.open {
            self.close_launcher();
        }
        if let Some(action) = action {
            self.dispatch_launcher_entry(&action)?;
        }
        self.sync_launcher()?;
        Ok(true)
    }

    fn poll_launcher_search(&mut self) -> Result<(), String> {
        if let Some(ui) = self.launcher.ui.as_mut() {
            let mut host = ui.bind_mut();
            let error = host.sync_input();
            if !error.is_empty() {
                return Err(error.to_string());
            }
            let text = host.frame_text(SEARCH_FIELD.into()).to_string();
            self.launcher.view.set_query(&text);
        }
        Ok(())
    }

    pub(super) fn update_launcher(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_launcher();
            return Ok(());
        }
        if !self.launcher.view.open {
            return Ok(());
        }
        self.poll_launcher_search()?;
        let action = self
            .launcher
            .ui
            .as_mut()
            .ok_or("Launcher UI missing")?
            .bind_mut()
            .pop_action()
            .to_string();
        if !action.is_empty() {
            let destination = self.launcher.view.action(&action);
            if !self.launcher.view.open {
                self.close_launcher();
            }
            if let Some(destination) = destination {
                self.dispatch_launcher_entry(&destination)?;
            }
        }
        Ok(self.sync_launcher()?)
    }

    fn sync_launcher(&mut self) -> Result<(), String> {
        if !self.launcher.view.open {
            return Ok(());
        }
        let scale = self.effective_ui_scale();
        let view = self.launcher.view.clone();
        if let Some(ui) = self.launcher.ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(view);
        }
        self.extract_art(&launcher_texture_fdids(view.clone()));
        self.drawable_fdid(model::CHARACTER_ICON.mask_fdid);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("LauncherUI");
        ui.set_layer(20);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_launcher(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.launcher.ui = Some(ui);
        Ok(())
    }

    fn dispatch_launcher_entry(&mut self, action: &str) -> Result<(), FrameError> {
        if action.starts_with(game_engine_ui_model::micro_menu::ACTION_PREFIX) {
            return self.micro_button_click(action);
        }
        match action {
            "bag_toggle:0" => self.toggle_bag_action(action)?,
            game_engine_ui_model::minimap::ACTION_TOGGLE_WORLD_MAP => self.toggle_world_map()?,
            game_engine_ui_model::game_menu_main::ACTION_OPTIONS => {
                self.open_launcher_options(false)?
            }
            model::ACTION_KEY_BINDINGS => self.open_launcher_options(true)?,
            game_engine_ui_model::game_menu_main::ACTION_SUPPORT => self.show_support(),
            _ => return Err(format!("Unknown launcher entry {action}").into()),
        }
        Ok(())
    }

    fn open_launcher_options(&mut self, key_bindings: bool) -> Result<(), String> {
        self.open_game_menu()?;
        if key_bindings {
            let model = self
                .game_menu_options
                .as_mut()
                .ok_or("Options model missing")?;
            model.category =
                game_engine_ui_model::options_menu_component::OptionsCategory::Keybindings;
            model.binding_section =
                game_engine_core::input_bindings_data::BindingSection::Interface;
        }
        self.show_game_menu_options()
    }
}
