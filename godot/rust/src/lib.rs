mod account;
mod animation;
mod assets;
mod scene;
mod terrain;
mod ui;

use std::{collections::HashMap, path::PathBuf};

use account::{Account, AccountEvent};
use game_engine_network::UnitSnapshot;
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    char_select_component::CharSelectAction, char_select_state_from_roster,
};
use godot::classes::{INode3D, Node3D, ProjectSettings};
use godot::prelude::*;

struct GameEngineExtension;

// SAFETY: Godot owns extension initialization and all exposed objects use gdext's bindings.
#[gdextension]
unsafe impl ExtensionLibrary for GameEngineExtension {}

/// Native root for the Godot client scene.
#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct GameClient {
    base: Base<Node3D>,
    model_scene: Option<Gd<Node3D>>,
    login_ui: Option<Gd<ui::RegistryUi>>,
    character_ui: Option<Gd<ui::RegistryUi>>,
    loading_ui: Option<Gd<ui::RegistryUi>>,
    account: Account,
    units: HashMap<u64, UnitSnapshot>,
    server_hostname: String,
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            model_scene: None,
            login_ui: None,
            character_ui: None,
            loading_ui: None,
            account: Account::new(PathBuf::from(
                ProjectSettings::singleton()
                    .globalize_path("res://../data")
                    .to_string(),
            )),
            units: HashMap::new(),
            server_hostname: if cfg!(debug_assertions) {
                "127.0.0.1:5000"
            } else {
                "game.worldofosso.com:5000"
            }
            .into(),
        }
    }

    fn process(&mut self, _delta: f64) {
        if let Err(error) = self.poll_ui_actions().and_then(|()| self.poll_account()) {
            self.account.session.feedback = Some(error.clone());
            godot_error!("Account update failed: {error}");
            if let Err(ui_error) = self.update_login_status(&error, false) {
                godot_error!("Login feedback failed: {ui_error}");
            }
            if let Err(stop_error) = self.account.stop() {
                godot_error!("Account shutdown failed: {stop_error}");
            }
        }
    }

    fn exit_tree(&mut self) {
        if let Err(error) = self.account.stop() {
            godot_error!("Account shutdown failed: {error}");
        }
    }

    fn ready(&mut self) {
        if let Err(error) = self.attach_login_ui() {
            godot_error!("Cannot initialize login UI: {error}");
        }
    }
}

#[godot_api]
impl GameClient {
    #[signal]
    fn screen_requested(screen: GString);

    #[func]
    fn set_server(&mut self, server: GString) {
        self.server_hostname = server.to_string();
    }

    #[func]
    fn connect_account(
        &mut self,
        server: GString,
        username: GString,
        password: GString,
        register: bool,
    ) -> GString {
        match self.account.connect(
            &server.to_string(),
            &username.to_string(),
            &password.to_string(),
            register,
        ) {
            Ok(()) => {
                self.units.clear();
                GString::new()
            }
            Err(error) => GString::from(error.as_str()),
        }
    }

    #[func]
    fn account_state(&self) -> VarDictionary {
        let session = &self.account.session;
        let mut state = VarDictionary::new();
        state.set("screen", format!("{:?}", session.screen).as_str());
        state.set("status", session.feedback.as_deref().unwrap_or(""));
        state.set("character_count", session.characters.len() as i64);
        state.set("unit_count", self.units.len() as i64);
        state.set("reply_received", self.account.reply_received);
        state.set(
            "selected_character_id",
            &session
                .selected_character_id
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "selected_character_name",
            session.selected_character_name.as_deref().unwrap_or(""),
        );
        state
    }

    #[func]
    fn load_model_scene(&mut self, path: GString) -> VarDictionary {
        let mut result = VarDictionary::new();
        match self.import_model_scene(&path) {
            Ok((bounds, missing_textures)) => {
                result.set("bounds", bounds);
                result.set("missing_texture_fdids", &missing_textures);
            }
            Err(error) => result.set("error", error.as_str()),
        }
        result
    }
}

impl GameClient {
    fn poll_ui_actions(&mut self) -> Result<(), String> {
        match self.account.session.screen {
            SessionScreen::Login => self.poll_login_actions(),
            SessionScreen::CharacterSelect => self.poll_character_actions(),
            _ => Ok(()),
        }
    }

    fn poll_character_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.character_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let action = ui.bind_mut().pop_action().to_string();
        match CharSelectAction::parse(&action) {
            Some(CharSelectAction::SelectChar(index)) => {
                self.account.session.selected_index = Some(index);
                let state = char_select_state_from_roster(
                    &self.account.session.characters,
                    self.account.session.selected_index,
                );
                ui.bind_mut().set_character_select_state(state)
            }
            Some(CharSelectAction::EnterWorld) => self.account.send_enter_world(),
            Some(CharSelectAction::Back) => {
                self.account.session.screen = SessionScreen::Login;
                self.show_account_screen(SessionScreen::Login)
            }
            None if action.is_empty() => Ok(()),
            _ => Err(format!("Character action not yet converted: {action}")),
        }
    }

    fn poll_login_actions(&mut self) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::Login {
            return Ok(());
        }
        let Some(login) = self.login_ui.as_mut() else {
            return Ok(());
        };
        let error = login.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let action = login.bind_mut().pop_action().to_string();
        match action.as_str() {
            "" => Ok(()),
            "connect" => {
                let credentials = login.bind().credentials();
                let username = credential_field(&credentials, "username")?;
                let password = credential_field(&credentials, "password")?;
                if username.trim().is_empty() || password.trim().is_empty() {
                    return self.update_login_status("Please fill in all fields", false);
                }
                self.account
                    .connect(&self.server_hostname, &username, &password, false)?;
                self.units.clear();
                self.update_login_status("Connecting...", true)
            }
            "reconnect" => {
                self.account.connect(&self.server_hostname, "", "", false)?;
                self.units.clear();
                self.update_login_status("Connecting...", true)
            }
            "exit" => {
                self.base().get_tree().quit();
                Ok(())
            }
            other => Err(format!("Login action not yet converted: {other}")),
        }
    }

    fn update_login_status(&mut self, status: &str, connecting: bool) -> Result<(), String> {
        let Some(login) = self.login_ui.as_mut() else {
            return Ok(());
        };
        let mut login = login.bind_mut();
        let error = login.set_connecting(connecting);
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let error = login.set_status(GString::from(status));
        if !error.is_empty() {
            return Err(error.to_string());
        }
        Ok(())
    }

    fn poll_account(&mut self) -> Result<(), String> {
        for event in self.account.poll()? {
            match event {
                AccountEvent::Screen(screen) => {
                    self.show_account_screen(screen)?;
                    let status = self.account.session.feedback.clone().unwrap_or_default();
                    self.update_login_status(&status, false)?;
                }
                AccountEvent::WorldReset => self.units.clear(),
                AccountEvent::UnitUpdated(unit) => {
                    self.units.insert(unit.server_id, unit);
                }
                AccountEvent::UnitRemoved(id) => {
                    self.units.remove(&id);
                }
            }
        }
        Ok(())
    }

    fn show_account_screen(&mut self, screen: SessionScreen) -> Result<(), String> {
        match screen {
            SessionScreen::CharacterSelect => self.attach_character_ui()?,
            SessionScreen::Loading => self.attach_loading_ui()?,
            SessionScreen::Login => {
                if let Some(ui) = self.character_ui.take() {
                    ui.free();
                }
                if let Some(ui) = self.loading_ui.take() {
                    ui.free();
                }
            }
            _ => {}
        }
        self.set_account_ui_visibility(screen);
        let name = GString::from(format!("{screen:?}").as_str());
        self.base_mut()
            .emit_signal("screen_requested", &[name.to_variant()]);
        Ok(())
    }

    fn set_account_ui_visibility(&mut self, screen: SessionScreen) {
        for (ui, target) in [
            (&mut self.login_ui, SessionScreen::Login),
            (&mut self.character_ui, SessionScreen::CharacterSelect),
            (&mut self.loading_ui, SessionScreen::Loading),
        ] {
            if let Some(ui) = ui {
                ui.set_visible(screen == target);
            }
        }
    }

    fn attach_character_ui(&mut self) -> Result<(), String> {
        let mut ui = ui::RegistryUi::new_alloc();
        ui.set_name("CharacterSelectUI");
        self.base_mut().add_child(&ui);
        let error = ui.bind_mut().show_character_select();
        if !error.is_empty() {
            ui.free();
            return Err(error.to_string());
        }
        let state = char_select_state_from_roster(
            &self.account.session.characters,
            self.account.session.selected_index,
        );
        let result = ui.bind_mut().set_character_select_state(state);
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        if let Some(previous) = self.character_ui.replace(ui) {
            previous.free();
        }
        Ok(())
    }

    fn attach_loading_ui(&mut self) -> Result<(), String> {
        let mut ui = ui::RegistryUi::new_alloc();
        ui.set_name("LoadingUI");
        self.base_mut().add_child(&ui);
        let error = ui.bind_mut().show_loading();
        if !error.is_empty() {
            ui.free();
            return Err(error.to_string());
        }
        if let Some(previous) = self.loading_ui.replace(ui) {
            previous.free();
        }
        Ok(())
    }

    fn attach_login_ui(&mut self) -> Result<(), String> {
        let viewport = self.base().get_viewport().ok_or("Client has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut login = ui::create_login_ui(size.x, size.y)?;
        login.set_name("LoginUI");
        self.base_mut().add_child(&login);
        self.login_ui = Some(login);
        Ok(())
    }

    fn import_model_scene(&mut self, path: &GString) -> Result<(Aabb, PackedInt32Array), String> {
        let (model, missing_textures) = assets::load_model_node(path)?;
        let bounds = match scene::collect_mesh_bounds(&model) {
            Ok(bounds) => bounds,
            Err(error) => {
                model.free();
                return Err(error);
            }
        };
        let mut container = Node3D::new_alloc();
        container.set_name("ModelScene");
        container.add_child(&model);
        if let Some(previous) = self.model_scene.take() {
            self.base_mut().remove_child(&previous);
            previous.free();
        }
        self.base_mut().add_child(&container);
        scene::attach_preview_camera(&mut container, bounds);
        scene::attach_preview_light(&mut container);
        self.model_scene = Some(container);
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        Ok((bounds, missing_textures))
    }
}

fn credential_field(credentials: &VarDictionary, name: &str) -> Result<String, String> {
    let value = credentials
        .get(name)
        .ok_or_else(|| format!("Missing login field {name}"))?;
    let value = value
        .try_to::<GString>()
        .map_err(|_| format!("Invalid login field type {name}"))?;
    Ok(value.to_string())
}
