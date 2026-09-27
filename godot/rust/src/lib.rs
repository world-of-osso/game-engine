mod account;
mod animation;
#[path = "../../../src/rendering/character/appearance_options.rs"]
pub mod appearance_options;
pub use game_engine_core::{customization_data, outfit_data};
mod assets;
mod camera;
mod char_create;
mod character_select;
#[path = "../../../src/game/equipment/equipment_appearance_data.rs"]
pub mod equipment_appearance_data;
mod gameplay;
mod ground;
mod input;
mod input_keys;
mod lighting;
mod loading;
mod scene;
mod terrain;
mod ui;
mod wmo;
mod world;
mod world_models;

use std::{collections::HashMap, path::PathBuf};

use account::{Account, AccountEvent};
use game_engine_core::client_options_data::{ClientOptionsFile, load_options_file_with_legacy};
use game_engine_network::UnitSnapshot;
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    char_create_component::{CREATE_NAME_INPUT, CharCreateAction, CharCreateMode},
    char_select_component::{
        CharSelectAction, DELETE_CONFIRM_INPUT, DeleteCharacterTarget, DeleteConfirmation,
        step_selection,
    },
    char_select_state_from_roster,
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
    create_ui: Option<Gd<ui::RegistryUi>>,
    character_preview: character_select::CharacterPreview,
    delete_confirmation: DeleteConfirmation,
    creation: Option<char_create::CharCreateState>,
    creation_catalog: Option<game_engine_core::customization_data::CustomizationDb>,
    name_catalog: Option<Result<char_create::NameCatalog, String>>,
    data_root: PathBuf,
    loading_ui: Option<Gd<ui::RegistryUi>>,
    errors_ui: Option<Gd<ui::RegistryUi>>,
    account: Account,
    units: HashMap<u64, UnitSnapshot>,
    world: world::WorldUnits,
    terrain: terrain::streaming::StreamedTerrain,
    terrain_materials: terrain::material::TerrainMaterials,
    world_lighting: lighting::WorldLighting,
    world_camera: camera::WorldCamera,
    physical_input: input::PhysicalInput,
    client_options: ClientOptionsFile,
    player_movement: gameplay::PlayerMovement,
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
        let client_options =
            load_options_file_with_legacy(&data_root.join("ui/options_settings.ron")).clamped();
        Self {
            base,
            model_scene: None,
            login_ui: None,
            character_ui: None,
            create_ui: None,
            delete_confirmation: DeleteConfirmation::default(),
            creation: None,
            creation_catalog: None,
            name_catalog: None,
            data_root: data_root.clone(),
            character_preview: character_select::CharacterPreview::new(
                data_root.clone(),
                cache_root.clone(),
            ),
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
            physical_input: input::PhysicalInput::default(),
            client_options,
            player_movement: gameplay::PlayerMovement::default(),
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

    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        self.physical_input.capture(&event);
    }

    /// Screen keys left unhandled by focused edit boxes.
    fn unhandled_key_input(&mut self, event: Gd<godot::classes::InputEvent>) {
        let Ok(key) = event.try_cast::<godot::classes::InputEventKey>() else {
            return;
        };
        if !key.is_pressed() || self.account.session.screen != SessionScreen::CharacterSelect {
            return;
        }
        if let Err(error) = self.handle_character_select_key(key.get_keycode()) {
            godot_error!("Character select key failed: {error}");
        }
    }

    fn process(&mut self, delta: f64) {
        let update = self
            .poll_ui_actions()
            .and_then(|()| self.poll_account())
            .and_then(|()| self.update_character_preview())
            .and_then(|()| self.update_player_input(delta as f32))
            .map(|()| self.world.advance(delta as f32))
            .and_then(|()| self.update_player_animation())
            .and_then(|()| self.send_player_input(delta as f32))
            .and_then(|()| self.terrain.poll())
            .and_then(|()| self.update_world_lighting())
            .and_then(|()| self.attach_terrain_materials())
            .and_then(|()| self.update_loading_readiness())
            .and_then(|()| self.update_world_errors(delta as f32))
            .and_then(|()| self.tick_delete_confirmation(delta as f32))
            .and_then(|()| self.advance_login_fade(delta as f32))
            .and_then(|()| self.update_world_camera(delta as f32));
        self.physical_input.finish_frame();
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
        if let Err(error) = self
            .connect_focus_reset()
            .and_then(|()| self.attach_login_ui())
        {
            godot_error!("Cannot initialize client: {error}");
        }
    }
}

#[godot_api]
impl GameClient {
    #[signal]
    fn screen_requested(screen: GString);

    #[func]
    fn fps_overlay_enabled(&self) -> bool {
        self.client_options.hud.show_fps_overlay
    }

    #[func]
    fn clear_physical_input(&mut self) {
        self.physical_input.clear();
    }

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
    fn connect_focus_reset(&mut self) -> Result<(), String> {
        let mut window = self.base().get_window().ok_or("Client has no window")?;
        let callback = self.to_gd().callable("clear_physical_input");
        // Deliver outside the active gdext borrow; unrelated scene notifications
        // must not re-enter GameClient while it attaches or updates child nodes.
        let error = window.connect_flags(
            "focus_exited",
            &callback,
            godot::classes::object::ConnectFlags::DEFERRED,
        );
        if error != godot::global::Error::OK {
            return Err(format!("Connect window focus reset: {error:?}"));
        }
        Ok(())
    }

    fn poll_ui_actions(&mut self) -> Result<(), String> {
        match self.account.session.screen {
            SessionScreen::Login => self.poll_login_actions(),
            SessionScreen::CharacterSelect => self.poll_character_actions(),
            SessionScreen::CharacterCreate => self.poll_create_actions(),
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
        let typed = ui.bind_mut().frame_text(DELETE_CONFIRM_INPUT.0.into());
        let action = ui.bind_mut().pop_action().to_string();
        self.update_delete_typed_text(&typed.to_string())?;
        match CharSelectAction::parse(&action) {
            Some(CharSelectAction::SelectChar(index)) => self.select_character(Some(index)),
            Some(CharSelectAction::EnterWorld) => self.account.send_enter_world(),
            Some(CharSelectAction::CreateToggle) => {
                self.account.session.screen = SessionScreen::CharacterCreate;
                self.show_account_screen(SessionScreen::CharacterCreate)
            }
            Some(CharSelectAction::DeleteChar) => self.open_delete_confirmation(),
            Some(CharSelectAction::ConfirmDeleteChar) => self.confirm_delete_character(),
            Some(CharSelectAction::CancelDeleteChar) => {
                self.delete_confirmation.clear();
                self.sync_delete_confirmation()
            }
            Some(CharSelectAction::Back) => {
                self.account.session.screen = SessionScreen::Login;
                self.show_account_screen(SessionScreen::Login)
            }
            None if action.is_empty() => Ok(()),
            _ => Err(format!("Character action not yet converted: {action}")),
        }
    }

    /// Original character-select keys: Up/Down navigate, Enter enters or confirms deletion,
    /// Escape cancels a pending deletion.
    fn handle_character_select_key(&mut self, key: godot::global::Key) -> Result<(), String> {
        use godot::global::Key;
        let deleting = self.delete_confirmation.target.is_some();
        match key {
            Key::ESCAPE if deleting => {
                self.delete_confirmation.clear();
                self.sync_delete_confirmation()
            }
            Key::ENTER | Key::KP_ENTER if deleting => self.confirm_delete_character(),
            _ if deleting => Ok(()),
            Key::UP | Key::DOWN => {
                let session = &self.account.session;
                let next = step_selection(
                    session.selected_index,
                    session.characters.len(),
                    key == Key::DOWN,
                );
                self.select_character(next)
            }
            Key::ENTER | Key::KP_ENTER if self.account.session.selected_index.is_some() => {
                self.account.send_enter_world()
            }
            _ => Ok(()),
        }
    }

    fn select_character(&mut self, index: Option<usize>) -> Result<(), String> {
        self.account.session.selected_index = index;
        self.sync_character_select_state()
    }

    fn sync_character_select_state(&mut self) -> Result<(), String> {
        let state = char_select_state_from_roster(
            &self.account.session.characters,
            self.account.session.selected_index,
        );
        match self.character_ui.as_mut() {
            Some(ui) => ui.bind_mut().set_state(state),
            None => Ok(()),
        }
    }

    fn open_delete_confirmation(&mut self) -> Result<(), String> {
        let session = &self.account.session;
        let Some(character) = session
            .selected_index
            .and_then(|index| session.characters.get(index))
        else {
            return Ok(());
        };
        self.delete_confirmation.open(DeleteCharacterTarget {
            character_id: character.character_id,
            name: character.name.clone(),
        });
        self.sync_delete_confirmation()?;
        match self.character_ui.as_mut() {
            Some(ui) => ui.bind_mut().focus_frame_named(DELETE_CONFIRM_INPUT.0),
            None => Ok(()),
        }
    }

    fn confirm_delete_character(&mut self) -> Result<(), String> {
        if !self.delete_confirmation.ready() {
            return Ok(());
        }
        let Some(target) = self.delete_confirmation.target.take() else {
            return Ok(());
        };
        self.account.send_delete_character(target.character_id)?;
        self.delete_confirmation.clear();
        self.sync_delete_confirmation()
    }

    /// Original confirmation input is upper-cased as it is typed.
    fn update_delete_typed_text(&mut self, typed: &str) -> Result<(), String> {
        let typed = typed.to_ascii_uppercase();
        if self.delete_confirmation.target.is_none() || self.delete_confirmation.typed_text == typed
        {
            return Ok(());
        }
        self.delete_confirmation.typed_text = typed;
        self.sync_delete_confirmation()
    }

    fn tick_delete_confirmation(&mut self, delta: f32) -> Result<(), String> {
        if self.delete_confirmation.target.is_none() {
            return Ok(());
        }
        self.delete_confirmation.tick(delta);
        self.sync_delete_confirmation()
    }

    fn sync_delete_confirmation(&mut self) -> Result<(), String> {
        let state = self.delete_confirmation.ui_state();
        match self.character_ui.as_mut() {
            Some(ui) => ui.bind_mut().set_state(state),
            None => Ok(()),
        }
    }

    fn poll_create_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.create_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let name = ui
            .bind_mut()
            .frame_text(CREATE_NAME_INPUT.0.into())
            .to_string();
        let action = ui.bind_mut().pop_action().to_string();
        let (Some(state), Some(db)) = (self.creation.as_mut(), self.creation_catalog.as_ref())
        else {
            return Ok(());
        };
        // Original: the name draft follows the edit box while it exists (Customize mode).
        if state.mode == CharCreateMode::Customize {
            state.name = name;
        }
        let Some(action) = CharCreateAction::parse(&action) else {
            return self.sync_creation_ui();
        };
        let names = self
            .name_catalog
            .as_ref()
            .map_or(Err("Authored random names are unavailable"), |names| {
                names.as_ref().map_err(String::as_str)
            });
        let draft = state.name.clone();
        let effects = char_create::reduce(
            state,
            action,
            db,
            names,
            &draft,
            char_create::fresh_random_seed(),
        );
        for effect in effects {
            self.apply_creation_effect(effect)?;
        }
        self.sync_creation_ui()
    }

    fn apply_creation_effect(
        &mut self,
        effect: char_create::CharCreateEffect,
    ) -> Result<(), String> {
        use char_create::CharCreateEffect;
        match effect {
            CharCreateEffect::ExitToCharSelect => {
                self.creation = None;
                self.account.session.screen = SessionScreen::CharacterSelect;
                self.show_account_screen(SessionScreen::CharacterSelect)
            }
            // The edit box shows `CharCreateState::name` through the next UI state.
            CharCreateEffect::SetNameText(name) => {
                if let Some(state) = self.creation.as_mut() {
                    state.name = name;
                }
                Ok(())
            }
            CharCreateEffect::SendCreate(request) => self.account.send_create_character(request),
            CharCreateEffect::FocusNameInput => match self.create_ui.as_mut() {
                Some(ui) if ui.bind().has_frame(CREATE_NAME_INPUT.0) => {
                    ui.bind_mut().focus_frame_named(CREATE_NAME_INPUT.0)
                }
                _ => Ok(()),
            },
        }
    }

    /// Enter creation with the original default Human Warrior and a random appearance.
    fn start_creation(&mut self) -> Result<(), String> {
        if self.creation_catalog.is_none() {
            self.creation_catalog = Some(
                game_engine_core::npc_appearance_assets::load_customization_db(&self.data_root)?,
            );
        }
        if self.name_catalog.is_none() {
            let names = char_create::NameCatalog::load(&self.data_root.join("NameGen.csv"));
            if let Err(error) = &names {
                godot_error!("Random name control unavailable: {error}");
            }
            self.name_catalog = Some(names);
        }
        let db = self
            .creation_catalog
            .as_ref()
            .expect("loaded customization catalog");
        self.creation = Some(char_create::initial_state(None, db));
        self.sync_creation_ui()
    }

    fn sync_creation_ui(&mut self) -> Result<(), String> {
        let (Some(state), Some(db), Some(ui)) = (
            self.creation.as_ref(),
            self.creation_catalog.as_ref(),
            self.create_ui.as_mut(),
        ) else {
            return Ok(());
        };
        let names = self
            .name_catalog
            .as_ref()
            .map_or(Err("Authored random names are unavailable"), |names| {
                names.as_ref().map_err(String::as_str)
            });
        let focused = state.mode == CharCreateMode::Customize
            && ui.bind().is_frame_focused(CREATE_NAME_INPUT.0);
        let size = ui
            .get_viewport()
            .ok_or("Character creation UI has no viewport")?
            .get_visible_rect()
            .size;
        let ui_state =
            char_create::ui_state(state, db, names, focused, (size.x as u32, size.y as u32));
        ui.bind_mut().set_state(ui_state)
    }

    fn receive_creation_result(
        &mut self,
        success: bool,
        error: Option<String>,
    ) -> Result<(), String> {
        let Some(state) = self.creation.as_mut() else {
            return Ok(());
        };
        match char_create::receive_create_result(state, success, error) {
            Some(effect) => self.apply_creation_effect(effect),
            None => self.sync_creation_ui(),
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

    fn advance_login_fade(&mut self, delta: f32) -> Result<(), String> {
        match self.login_ui.as_mut() {
            Some(login) => login.bind_mut().advance_login_fade(delta),
            None => Ok(()),
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
                AccountEvent::TransferError(error) => self.add_world_error(&error)?,
                AccountEvent::UnitUpdated(unit) => {
                    let mut parent = self.to_gd().upcast::<Node3D>();
                    self.world.upsert(&mut parent, &unit);
                    self.units.insert(unit.server_id, unit);
                }
                AccountEvent::RosterChanged => self.sync_character_select_state()?,
                AccountEvent::CharacterCreated { success, error } => {
                    self.receive_creation_result(success, error)?
                }
                AccountEvent::UnitRemoved(id) => {
                    self.world.remove(id);
                    self.units.remove(&id);
                }
            }
        }
        self.world
            .select_local_player(self.account.session.selected_character_name.as_deref());
        self.world
            .update_visibility(&self.units, self.world_minutes);
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
            self.world.update_lighting(None);
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
        self.world.update_lighting(None);
        let [x, y, z] = destination.position;
        let tile = game_engine_core::terrain_height_data::bevy_to_tile_coords(x, z);
        self.terrain.request_map(destination.map_directory, tile)?;
        if let Some(mut player) = self.world.local_player_node() {
            player.set_position(Vector3::new(x, y, z));
            player.set_rotation(Vector3::new(0.0, destination.facing, 0.0));
            self.world
                .set_local_player_facing(destination.facing + std::f32::consts::FRAC_PI_2);
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
            self.world.update_lighting(Some(light.clone()));
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
        self.world_camera.configure(&self.client_options.camera);
        self.world_camera
            .sync(&mut parent, &player, &self.terrain, delta)
    }

    fn reset_world(&mut self) -> Result<(), String> {
        self.character_preview.reset();
        self.physical_input.clear();
        self.player_movement = gameplay::PlayerMovement::default();
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

    fn update_character_preview(&mut self) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::CharacterSelect {
            self.character_preview.reset();
            return Ok(());
        }
        let selected = self
            .account
            .session
            .selected_index
            .and_then(|index| self.account.session.characters.get(index))
            .cloned();
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.character_preview
            .sync(&mut parent, selected.as_ref(), self.world_minutes)
    }

    fn show_account_screen(&mut self, screen: SessionScreen) -> Result<(), String> {
        if screen != SessionScreen::CharacterSelect {
            self.character_preview.reset();
        }
        if screen != SessionScreen::InWorld
            && let Some(ui) = self.errors_ui.as_mut()
            && ui.is_visible()
        {
            ui.bind_mut().clear_errors()?;
        }
        match screen {
            SessionScreen::CharacterSelect => self.attach_character_ui()?,
            SessionScreen::CharacterCreate => self.attach_create_ui()?,
            SessionScreen::Loading => self.attach_loading_ui()?,
            SessionScreen::InWorld => self.attach_errors_ui()?,
            SessionScreen::Login => {
                if let Some(ui) = self.character_ui.take() {
                    ui.free();
                }
                if let Some(ui) = self.create_ui.take() {
                    ui.free();
                }
                if let Some(ui) = self.loading_ui.take() {
                    ui.free();
                }
            }
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
            (&mut self.create_ui, SessionScreen::CharacterCreate),
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
        self.delete_confirmation.clear();
        let result = ui.bind_mut().set_state(state);
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

    fn attach_create_ui(&mut self) -> Result<(), String> {
        let mut ui = ui::RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        let error = ui.bind_mut().show_character_create();
        if !error.is_empty() {
            ui.free();
            return Err(error.to_string());
        }
        if let Some(previous) = self.create_ui.replace(ui.clone()) {
            previous.free();
        }
        ui.set_name("CharacterCreateUI");
        self.start_creation()
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
        if self.is_development_realm() {
            let credentials = game_engine_core::client_options_data::load_login_credentials()
                .unwrap_or(game_engine_core::client_options_data::LoginCredentials {
                    username: "admin".into(),
                    password: "admin".into(),
                });
            login
                .bind_mut()
                .prefill_login(&credentials.username, &credentials.password)?;
        }
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

impl GameClient {
    /// Original login prefills credentials only for the development realm.
    fn is_development_realm(&self) -> bool {
        use game_engine_core::realm_preset_data::RealmPreset;
        RealmPreset::ALL
            .into_iter()
            .find(|preset| preset.matches_hostname(&self.server_hostname))
            .is_some_and(RealmPreset::is_dev)
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
