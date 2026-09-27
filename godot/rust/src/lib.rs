mod account;
mod animation;
mod assets;
mod camera;
mod lighting;
mod loading;
mod scene;
mod terrain;
mod ui;
mod world;
mod world_models;

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
    errors_ui: Option<Gd<ui::RegistryUi>>,
    account: Account,
    units: HashMap<u64, UnitSnapshot>,
    world: world::WorldUnits,
    terrain: terrain::streaming::StreamedTerrain,
    terrain_materials: terrain::material::TerrainMaterials,
    world_lighting: lighting::WorldLighting,
    world_camera: camera::WorldCamera,
    world_minutes: f32,
    server_hostname: String,
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        let settings = ProjectSettings::singleton();
        let data_root = PathBuf::from(settings.globalize_path("res://../data").to_string());
        let cache_root =
            PathBuf::from(settings.globalize_path("user://asset-resolver").to_string());
        Self {
            base,
            model_scene: None,
            login_ui: None,
            character_ui: None,
            loading_ui: None,
            errors_ui: None,
            account: Account::new(data_root.clone()),
            terrain: terrain::streaming::StreamedTerrain::new(
                data_root.clone(),
                cache_root.clone(),
            ),
            terrain_materials: terrain::material::TerrainMaterials::default(),
            world_lighting: lighting::WorldLighting::default(),
            world_camera: camera::WorldCamera::default(),
            // Preserve the original GameTime default: noon, with time advancement stopped.
            world_minutes: 1440.0,
            units: HashMap::new(),
            world: world::WorldUnits::new(data_root, cache_root),
            server_hostname: if cfg!(debug_assertions) {
                "127.0.0.1:5000"
            } else {
                "game.worldofosso.com:5000"
            }
            .into(),
        }
    }

    fn process(&mut self, delta: f64) {
        let update = self
            .poll_ui_actions()
            .and_then(|()| self.poll_account())
            .map(|()| self.world.advance(delta as f32))
            .and_then(|()| self.terrain.poll())
            .and_then(|()| self.update_world_lighting())
            .and_then(|()| self.attach_terrain_materials())
            .and_then(|()| self.update_loading_readiness())
            .and_then(|()| self.update_world_errors(delta as f32))
            .and_then(|()| self.update_world_camera(delta as f32));
        if let Err(error) = update {
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
        let connection = self
            .account
            .connect(
                &server.to_string(),
                &username.to_string(),
                &password.to_string(),
                register,
            )
            .and_then(|()| self.reset_world());
        match connection {
            Ok(()) => GString::new(),
            Err(error) => GString::from(error.as_str()),
        }
    }

    #[func]
    fn account_state(&self) -> VarDictionary {
        let local_transform = self.world.local_player_transform();
        let session = &self.account.session;
        let mut state = VarDictionary::new();
        state.set("screen", format!("{:?}", session.screen).as_str());
        state.set(
            "reconnect_phase",
            format!("{:?}", session.reconnect_phase).as_str(),
        );
        state.set("gameplay_input_allowed", session.gameplay_input_allowed());
        state.set("status", session.feedback.as_deref().unwrap_or(""));
        state.set("character_count", session.characters.len() as i64);
        state.set("unit_count", self.units.len() as i64);
        state.set("world_attached", self.world.root().is_some());
        state.set("terrain", &terrain::state::terrain_state(&self.terrain));
        state.set(
            "local_player_position",
            &local_transform
                .map(|transform| transform.origin.to_variant())
                .unwrap_or_default(),
        );
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

    /// Authored terrain surface at world X/Z, or nil when no loaded grid covers the point.
    #[func]
    fn terrain_height_at(&self, x: f32, z: f32) -> Variant {
        self.terrain
            .height_at(x, z)
            .map(|height| height.to_variant())
            .unwrap_or_default()
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
                self.reset_world()?;
                self.update_login_status("Connecting...", true)
            }
            "reconnect" => {
                self.account.connect(&self.server_hostname, "", "", false)?;
                self.reset_world()?;
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
                AccountEvent::WorldReset => self.reset_world()?,
                AccountEvent::LoadTerrain(request) => self.request_terrain(request)?,
                AccountEvent::NewWorld(destination) => self.transfer_world(destination)?,
                AccountEvent::TransferError(error) => self.add_world_error(error)?,
                AccountEvent::UnitUpdated(unit) => {
                    let mut parent = self.to_gd().upcast::<Node3D>();
                    self.world.upsert(&mut parent, &unit);
                    self.units.insert(unit.server_id, unit);
                }
                AccountEvent::UnitRemoved(id) => {
                    self.world.remove(id);
                    self.units.remove(&id);
                }
            }
        }
        self.world
            .select_local_player(self.account.session.selected_character_name.as_deref());
        self.account
            .session
            .finish_reconnect(self.world.local_player_node().is_some());
        Ok(())
    }

    fn request_terrain(&mut self, request: shared::protocol::LoadTerrain) -> Result<(), String> {
        let map_changed = self.terrain.state().map.as_deref() != Some(&request.map_name);
        self.terrain.request_map(
            request.map_name,
            (request.initial_tile_y, request.initial_tile_x),
        )?;
        if map_changed {
            self.world_camera.reset();
            self.world_lighting.reset();
            self.terrain_materials.reset();
            self.account.session.screen = SessionScreen::Loading;
            self.show_account_screen(SessionScreen::Loading)?;
        }
        Ok(())
    }

    fn transfer_world(&mut self, destination: shared::protocol::NewWorld) -> Result<(), String> {
        self.terrain.reset()?;
        self.terrain_materials.reset();
        self.world_lighting.reset();
        let [x, y, z] = destination.position;
        let tile = game_engine_core::terrain_height_data::bevy_to_tile_coords(x, z);
        self.terrain.request_map(destination.map_directory, tile)?;
        if let Some(mut player) = self.world.local_player_node() {
            player.set_position(Vector3::new(x, y, z));
            player.set_rotation(Vector3::new(0.0, destination.facing, 0.0));
        }
        Ok(())
    }

    fn update_world_lighting(&mut self) -> Result<(), String> {
        let mut parent = self.to_gd().upcast::<Node3D>();
        let Some(wdt) = self.terrain.map_wdt.as_ref() else {
            return Ok(());
        };
        let Some(player) = self.world.local_player_transform() else {
            return Ok(());
        };
        let map = self
            .terrain
            .map_name()
            .ok_or("Parsed WDT has no map name")?;
        let map_id = game_engine_core::light_lookup_data::map_name_to_id(map)
            .ok_or_else(|| format!("No authored map ID for terrain map {map}"))?;
        if let Some(light) = self.world_lighting.sync(
            &mut parent,
            &wdt.lighting,
            map_id,
            player.origin,
            self.world_minutes,
        )? {
            self.terrain_materials.update_lighting(light);
        }
        Ok(())
    }

    fn attach_terrain_materials(&mut self) -> Result<(), String> {
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.terrain_materials.sync(&mut parent, &self.terrain)
    }

    fn update_loading_readiness(&mut self) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::Loading {
            return Ok(());
        }
        let position = self.world.local_player_transform().map(|transform| {
            let origin = transform.origin;
            (origin.x, origin.z)
        });
        if let (Some((x, z)), Some(map)) = (position, self.terrain.map_name().map(str::to_owned)) {
            let tile = game_engine_core::terrain_height_data::bevy_to_tile_coords(x, z);
            self.terrain.request_map(map, tile)?;
        }
        let state = self.terrain.state();
        let readiness = loading::evaluate_native_loading(
            position,
            &state,
            self.terrain_materials.attached_tiles(),
        );
        if let Some(ui) = self.loading_ui.as_mut() {
            ui.bind_mut()
                .set_loading_state(readiness.progress_percent, readiness.status_text)?;
        }
        if readiness.complete {
            self.account.session.screen = SessionScreen::InWorld;
            self.show_account_screen(SessionScreen::InWorld)?;
            self.account.finish_world_port()?;
        }
        Ok(())
    }

    fn update_world_camera(&mut self, delta: f32) -> Result<(), String> {
        if self.terrain.parsed_tiles.is_empty() {
            return Ok(());
        }
        let Some(player) = self.world.local_player_node() else {
            return Ok(());
        };
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.world_camera
            .sync(&mut parent, &player, &self.terrain, delta)
    }

    fn reset_world(&mut self) -> Result<(), String> {
        if let Some(ui) = self.errors_ui.as_mut() {
            ui.bind_mut().clear_errors()?;
            ui.set_visible(false);
        }
        self.world_camera.reset();
        self.world_lighting.reset();
        self.terrain_materials.reset();
        self.world.reset();
        self.units.clear();
        self.terrain.reset()
    }

    fn show_account_screen(&mut self, screen: SessionScreen) -> Result<(), String> {
        if screen != SessionScreen::InWorld
            && let Some(ui) = self.errors_ui.as_mut()
            && ui.is_visible()
        {
            ui.bind_mut().clear_errors()?;
        }
        match screen {
            SessionScreen::CharacterSelect => self.attach_character_ui()?,
            SessionScreen::Loading => self.attach_loading_ui()?,
            SessionScreen::InWorld => self.attach_errors_ui()?,
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
        if let Some(ui) = self.errors_ui.as_mut() {
            ui.set_visible(screen == SessionScreen::InWorld);
        }
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

    fn attach_errors_ui(&mut self) -> Result<(), String> {
        if self.errors_ui.is_some() {
            return Ok(());
        }
        let mut ui = ui::RegistryUi::new_alloc();
        ui.set_name("UIErrors");
        self.base_mut().add_child(&ui);
        let result = ui.bind_mut().show_errors();
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        ui.set_visible(self.account.session.screen == SessionScreen::InWorld);
        self.errors_ui = Some(ui);
        Ok(())
    }

    fn add_world_error(&mut self, error: &str) -> Result<(), String> {
        self.attach_errors_ui()?;
        self.errors_ui
            .as_mut()
            .expect("error UI attached")
            .bind_mut()
            .add_error(error)
    }

    fn update_world_errors(&mut self, delta: f32) -> Result<(), String> {
        if self.account.session.screen == SessionScreen::InWorld
            && let Some(ui) = self.errors_ui.as_mut()
        {
            ui.bind_mut().tick_errors(delta)?;
        }
        Ok(())
    }

    fn attach_character_ui(&mut self) -> Result<(), String> {
        let mut ui = ui::RegistryUi::new_alloc();
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
        if let Some(previous) = self.character_ui.replace(ui.clone()) {
            previous.free();
        }
        ui.set_name("CharacterSelectUI");
        Ok(())
    }

    fn attach_loading_ui(&mut self) -> Result<(), String> {
        let mut ui = ui::RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        let error = ui.bind_mut().show_loading();
        if !error.is_empty() {
            ui.free();
            return Err(error.to_string());
        }
        if let Some(previous) = self.loading_ui.replace(ui.clone()) {
            previous.free();
        }
        ui.set_name("LoadingUI");
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
