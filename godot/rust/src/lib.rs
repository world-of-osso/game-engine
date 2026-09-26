mod account;
mod animation;
mod assets;
mod scene;
mod terrain;
mod ui;

use std::{collections::HashMap, path::PathBuf};

use account::{Account, AccountEvent};
use game_engine_network::UnitSnapshot;
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
    account: Account,
    units: HashMap<u64, UnitSnapshot>,
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            model_scene: None,
            login_ui: None,
            account: Account::new(PathBuf::from(
                ProjectSettings::singleton()
                    .globalize_path("res://../data")
                    .to_string(),
            )),
            units: HashMap::new(),
        }
    }

    fn process(&mut self, _delta: f64) {
        if let Err(error) = self.poll_account() {
            self.account.session.feedback = Some(error.clone());
            godot_error!("Account update failed: {error}");
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
    fn poll_account(&mut self) -> Result<(), String> {
        for event in self.account.poll()? {
            match event {
                AccountEvent::Screen(screen) => {
                    let name = GString::from(format!("{screen:?}").as_str());
                    self.base_mut()
                        .emit_signal("screen_requested", &[name.to_variant()]);
                    if let Some(login) = self.login_ui.as_mut() {
                        let status = self.account.session.feedback.as_deref().unwrap_or("");
                        let error = login.bind_mut().set_status(GString::from(status));
                        if !error.is_empty() {
                            return Err(error.to_string());
                        }
                    }
                }
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
        Ok((bounds, missing_textures))
    }
}
