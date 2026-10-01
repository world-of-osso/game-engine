mod account;
mod animation;
#[path = "../../../src/rendering/character/appearance_options.rs"]
pub mod appearance_options;
pub use game_engine_core::{customization_data, outfit_data};
mod asset_startup;
mod assets;
mod auction;
mod auras;
mod auto_attack;
mod bags;
mod camera;
mod char_create;
mod character_select;
mod chat;
mod combat_text;
mod combat_visuals;
mod damage_meter;
mod display_options;
mod entrance_bar;
#[path = "../../../src/game/equipment/equipment_appearance_data.rs"]
pub mod equipment_appearance_data;
#[path = "../../../src/game/faction_reaction.rs"]
mod faction_reaction;
mod frame_error;
mod game_menu;
mod game_objects;
mod gameplay;
mod ground;
mod input;
mod input_keys;
mod lighting;
mod loading;
mod logout;
mod loot;
mod mail;
mod merchant;
mod minimap;
mod mirror_timers;
mod nameplates;
#[path = "../../../src/game/creatures/npc_gear_data.rs"]
pub mod npc_gear_data;
mod objective_tracker;
mod particle_debug;
mod particles;
mod player_spells;
mod replicated;
mod scene;
mod sound;
mod sound_client;
mod sound_footsteps;
mod spell_assets;
mod spell_effects;
mod spell_sounds;
mod spell_tooltip;
mod spells;
mod startup;
mod swim;
mod targeting;
mod terrain;
mod ui;
mod ui_scale;
mod unit_pick;
mod wmo;
mod world;
mod world_map;
mod world_models;

use std::{collections::HashMap, path::PathBuf};

use account::{Account, AccountEvent};
use frame_error::FrameError;
use game_engine_core::client_options_data::{ClientOptionsFile, load_options_file_with_legacy};
use game_engine_network::replica::{Replica, ReplicationBatch, Schema, UnitChange};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    char_create_component::{CREATE_NAME_INPUT, CharCreateAction, CharCreateMode},
    char_select_component::{
        CampsiteEntry, CampsitePreview, CampsiteState, CharSelectAction, DELETE_CONFIRM_INPUT,
        DeleteCharacterTarget, DeleteConfirmation, step_selection,
    },
    char_select_state_from_roster,
};
use godot::classes::{INode3D, Node3D, ProjectSettings};
use godot::prelude::*;

struct GameEngineExtension;

/// Main-thread time per frame for in-world ADT object loading.
const WORLD_OBJECT_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);

// SAFETY: Godot owns extension initialization and all exposed objects use gdext's bindings.
#[gdextension]
unsafe impl ExtensionLibrary for GameEngineExtension {
    fn on_stage_deinit(stage: godot::init::InitStage) {
        // Release cached shaders before Godot tears down its rendering storage.
        if stage == godot::init::InitStage::MainLoop {
            assets::material::clear_shared_shaders();
            particles::clear_quad_mesh();
        }
    }
}

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
    campsite: CampsiteState,
    creation_scene: char_create::CreationScene,
    delete_confirmation: DeleteConfirmation,
    creation: Option<char_create::CharCreateState>,
    creation_catalog: Option<game_engine_core::customization_data::CustomizationDb>,
    name_catalog: Option<Result<char_create::NameCatalog, String>>,
    data_root: PathBuf,
    /// Pending CASC startup worker, or its spawn error; None after startup completes.
    asset_startup: Option<Result<asset_startup::AssetStartup, String>>,
    /// Account events received while CASC initializes, applied once it has.
    startup_events: Vec<AccountEvent>,
    loading_ui: Option<Gd<ui::RegistryUi>>,
    errors_ui: Option<Gd<ui::RegistryUi>>,
    mirror_timer_ui: Option<Gd<ui::RegistryUi>>,
    chat: chat::Chat,
    game_menu_ui: Option<Gd<ui::RegistryUi>>,
    world_map: world_map::WorldMap,
    minimap: minimap::Minimap,
    objective_tracker: objective_tracker::ObjectiveTracker,
    entrance_bar: entrance_bar::EntranceBar,
    damage_meter: damage_meter::DamageMeterHud,
    game_menu_options: Option<game_engine_ui_model::options_menu_data::OptionsModel>,
    game_menu_drag: Option<game_menu::drag::OptionsDrag>,
    logout: game_engine_session::logout::LogoutState,
    in_rest_area: bool,
    account: Account,
    sound: Option<Gd<sound::NativeSound>>,
    area_parents: HashMap<u32, u32>,
    /// Every replicated entity of the connection, the client's only copy.
    replica: Replica,
    world: world::WorldUnits,
    spell_effects: spell_effects::SpellEffects,
    terrain: terrain::streaming::StreamedTerrain,
    terrain_materials: terrain::material::TerrainMaterials,
    world_objects: terrain::objects::TerrainObjects,
    global_wmo: wmo::global::GlobalWmoScene,
    wmo_collision: wmo::collision::WmoCollisionBodies,
    world_lighting: lighting::WorldLighting,
    /// `Map.db2` ID of the map whose terrain is loaded; lighting selects its Light rows.
    world_map_id: Option<u32>,
    world_camera: camera::WorldCamera,
    physical_input: input::PhysicalInput,
    client_options: ClientOptionsFile,
    player_movement: gameplay::PlayerMovement,
    world_minutes: f32,
    server_hostname: String,
    startup_customize: bool,
    targeting: targeting::Targeting,
    nameplates: nameplates::Nameplates,
    spells: spells::SpellsHud,
    merchant: merchant::Merchant,
    bags: bags::Bags,
    mailbox: mail::Mailbox,
    game_objects: game_objects::GameObjects,
    loot: loot::Loot,
    auction: auction::Auction,
    auto_attack: auto_attack::AutoAttack,
    auras: auras::Auras,
}

#[godot_api]
impl INode3D for GameClient {
    fn init(base: Base<Node3D>) -> Self {
        let settings = ProjectSettings::singleton();
        let data_root = PathBuf::from(settings.globalize_path("res://../data").to_string());
        let asset_startup = Some(asset_startup::AssetStartup::start(data_root.clone()));
        let client_options =
            load_options_file_with_legacy(&data_root.join("ui/options_settings.ron")).clamped();
        // Bag items resolve names, quality and icons from the shared item tables; the
        // ~175k-row ItemSparse parse runs off the main thread.
        if let Err(error) = game_engine_ui_model::paths::set_data_root(data_root.clone()) {
            godot_error!("{error}");
        }
        game_engine_ui_model::item_catalog::warm_item_catalog();
        let mut world_objects = terrain::objects::TerrainObjects::new(
            "WorldObjects",
            WORLD_OBJECT_BUDGET,
            data_root.clone(),
        );
        let graphics = &client_options.graphics;
        if graphics.particle_effects_enabled {
            world_objects.enable_particles(f32::from(graphics.particle_density) / 100.0);
        }
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
            asset_startup,
            startup_events: Vec::new(),
            character_preview: character_select::CharacterPreview::new(data_root.clone()),
            campsite: CampsiteState::default(),
            creation_scene: char_create::CreationScene::new(data_root.clone()),
            loading_ui: None,
            errors_ui: None,
            mirror_timer_ui: None,
            chat: Default::default(),
            game_menu_ui: None,
            world_map: world_map::WorldMap::default(),
            minimap: minimap::Minimap::default(),
            objective_tracker: objective_tracker::ObjectiveTracker::default(),
            entrance_bar: entrance_bar::EntranceBar::default(),
            damage_meter: damage_meter::DamageMeterHud::default(),
            game_menu_options: None,
            game_menu_drag: None,
            logout: Default::default(),
            in_rest_area: false,
            account: Account::new(data_root.clone()),
            sound: None,
            area_parents: HashMap::new(),
            terrain: terrain::streaming::StreamedTerrain::new(data_root.clone()),
            terrain_materials: terrain::material::TerrainMaterials::default(),
            world_objects,
            global_wmo: wmo::global::GlobalWmoScene::new(data_root.clone()),
            wmo_collision: wmo::collision::WmoCollisionBodies::default(),
            world_lighting: lighting::WorldLighting::default(),
            world_map_id: None,
            world_camera: camera::WorldCamera::default(),
            physical_input: input::PhysicalInput::default(),
            client_options,
            player_movement: gameplay::PlayerMovement::default(),
            // Preserve the original GameTime default: noon, with time advancement stopped.
            world_minutes: 1440.0,
            startup_customize: false,
            targeting: targeting::Targeting::new(data_root.clone()),
            nameplates: nameplates::Nameplates::new(),
            spells: spells::SpellsHud::default(),
            merchant: merchant::Merchant::default(),
            bags: bags::Bags::default(),
            mailbox: mail::Mailbox::default(),
            game_objects: game_objects::GameObjects::new(data_root.clone()),
            loot: loot::Loot::default(),
            auction: auction::Auction::default(),
            auto_attack: auto_attack::AutoAttack::default(),
            auras: auras::Auras::default(),
            replica: Replica::default(),
            spell_effects: spell_effects::SpellEffects::new(data_root.clone()),
            world: world::WorldUnits::new(data_root),
            server_hostname: if cfg!(debug_assertions) {
                "127.0.0.1:5000"
            } else {
                "game.worldofosso.com:5000"
            }
            .into(),
        }
    }

    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        // Asset-using screens/actions must not race the startup worker's CASC locks.
        if self.asset_startup.is_some() {
            return;
        }
        let chat_used = self.chat_edit_key(&event).unwrap_or_else(|error| {
            self.handle_frame_error("Chat key", error.into());
            true
        }) || self.chat_wheel(&event);
        if chat_used {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        if self.capture_game_menu_binding(&event) {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        match self.handle_game_menu_pointer(&event) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Ok(false) => {}
            Err(error) => {
                godot_error!("Game menu drag failed: {error}");
                return;
            }
        }
        if self.world_map_pointer(&event)
            || self.minimap_pointer(&event)
            || self.spellbook_pointer(&event)
            || self.merchant_pointer(&event)
            || self.mailbox_pointer(&event)
        {
            return;
        }
        if self.game_menu_ui.is_none() {
            self.physical_input.capture(&event);
        }
    }

    /// Screen keys left unhandled by focused edit boxes.
    fn unhandled_key_input(&mut self, event: Gd<godot::classes::InputEvent>) {
        let Ok(key) = event.try_cast::<godot::classes::InputEventKey>() else {
            return;
        };
        if !key.is_pressed() {
            return;
        }
        if key.is_echo() && key.get_keycode() == godot::global::Key::ESCAPE {
            return;
        }
        // Retail Escape closes the spellbook, then the world map, before the game menu.
        if key.get_keycode() == godot::global::Key::ESCAPE && self.spellbook_open() {
            self.close_spellbook();
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        if key.get_keycode() == godot::global::Key::ESCAPE && self.world_map.is_open() {
            self.close_world_map();
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        match self.mailbox_key(key.get_keycode()) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Ok(false) => {}
            Err(error) => {
                self.handle_frame_error("Mailbox key", error.into());
                return;
            }
        }
        match self.auction_key(key.get_keycode()) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Err(error) => {
                self.handle_frame_error("Auction key", error.into());
                return;
            }
            Ok(false) => {}
        }
        match self.merchant_key(key.get_keycode()) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Err(error) => {
                self.handle_frame_error("Merchant key", error.into());
                return;
            }
            Ok(false) => {}
        }
        if self.bags_key(key.get_keycode()) {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        match self.open_chat_from_key(&key) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Err(error) => {
                self.handle_frame_error("Chat open", error.into());
                return;
            }
            Ok(false) => {}
        }
        if key.get_keycode() == godot::global::Key::ESCAPE && self.clear_target_on_escape() {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
            return;
        }
        match self.handle_game_menu_key(key.get_keycode()) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                return;
            }
            Err(error) => {
                godot_error!("Game menu key failed: {error}");
                return;
            }
            Ok(false) => {}
        }
        if self.account.session.screen != SessionScreen::CharacterSelect {
            return;
        }
        if let Err(error) = self.handle_character_select_key(key.get_keycode()) {
            self.handle_frame_error("Character select key", error);
        }
    }

    fn process(&mut self, delta: f64) {
        if !self.poll_asset_startup() {
            self.receive_account_during_startup();
            self.physical_input.finish_frame();
            return;
        }
        type Step = fn(&mut GameClient, f32) -> Result<(), FrameError>;
        // Each step runs even when an earlier one failed; only a session failure ends
        // the frame (docs/specs/godot-conversion.md, "Frame failure policy").
        let steps: &[(&str, Step)] = &[
            ("UI scale", |c, _| Ok(c.sync_registry_ui_scale()?)),
            ("UI click sounds", |c, _| Ok(c.play_ui_clicks()?)),
            ("UI actions", |c, _| c.poll_ui_actions()),
            ("Account", |c, _| c.poll_account()),
            ("Logout", |c, d| Ok(c.update_logout(f64::from(d))?)),
            (
                "Character preview",
                |c, _| Ok(c.update_character_preview()?),
            ),
            ("Creation scene", |c, d| Ok(c.update_creation_scene(d)?)),
            ("Player input", |c, d| Ok(c.update_player_input(d)?)),
            ("Targeting", |c, _| c.update_targeting()),
            ("Spells", |c, d| c.update_spells(d)),
            ("Auras", |c, _| c.update_auras()),
            ("Bags", |c, _| c.update_bags()),
            ("Merchant", |c, _| c.update_merchant()),
            ("Mailbox", |c, _| c.update_mailbox()),
            ("Loot", |c, _| c.update_loot()),
            ("Auction", |c, _| c.update_auction()),
            ("Chat", |c, d| c.update_chat(d)),
            ("World map", |c, _| Ok(c.update_world_map()?)),
            ("Minimap", |c, _| c.update_minimap()),
            ("Objective tracker", |c, _| c.update_objective_tracker()),
            ("Entrance bar", |c, d| c.update_entrance_bar(d)),
            ("Damage meter", |c, _| c.update_damage_meter()),
            ("World units", |c, d| {
                c.world.advance(d);
                Ok(())
            }),
            ("Player animation", |c, _| Ok(c.update_player_animation()?)),
            ("Footsteps", |c, _| Ok(c.update_footsteps()?)),
            ("Remote player animation", |c, _| {
                Ok(c.world.update_remote_locomotion()?)
            }),
            ("Spell visuals", |c, d| Ok(c.update_spell_visuals(d)?)),
            ("Player movement", |c, _| c.send_player_input()),
            ("Terrain", |c, _| Ok(c.poll_terrain()?)),
            ("World lighting", |c, _| Ok(c.update_world_lighting()?)),
            (
                "Terrain materials",
                |c, _| Ok(c.attach_terrain_materials()?),
            ),
            ("World objects", |c, _| {
                c.attach_world_objects();
                Ok(())
            }),
            ("Loading", |c, d| c.update_loading_readiness(d)),
            ("World errors", |c, d| Ok(c.update_world_errors(d)?)),
            ("Mirror timers", |c, d| Ok(c.update_mirror_timers(d)?)),
            ("Delete confirmation", |c, d| {
                Ok(c.tick_delete_confirmation(d)?)
            }),
            ("Login fade", |c, d| Ok(c.advance_login_fade(d)?)),
            ("World camera", |c, d| Ok(c.update_world_camera(d)?)),
            ("Nameplates", |c, _| Ok(c.update_nameplates()?)),
            ("Culling", |c, _| {
                c.cull_world_objects();
                Ok(())
            }),
            ("UI scale after updates", |c, _| {
                Ok(c.sync_registry_ui_scale()?)
            }),
        ];
        for (step, run) in steps {
            if let Err(error) = run(self, delta as f32)
                && self.handle_frame_error(step, error)
            {
                break;
            }
        }
        self.physical_input.finish_frame();
        if let Err(error) = self.update_sound() {
            frame_error::report_once(&format!("Sound update failed: {error}"));
        }
    }

    fn exit_tree(&mut self) {
        // The display server keeps the custom cursor texture until it is replaced; left
        // set, it outlives RenderingServer and its RID leaks at exit.
        self.set_world_cursor(None);
        self.stop_sound();
        if let Err(error) = self.account.stop() {
            godot_error!("Account shutdown failed: {error}");
        }
    }

    fn ready(&mut self) {
        // Model animation nodes tick at priority 0 before this observer reads their selected clock.
        self.base_mut().set_process_priority(1);
        // The root viewport is still attaching children during ready.
        self.base_mut().call_deferred("apply_display_options", &[]);
        // Asset-backed initialization resumes from process after the worker completes.
        if let Err(error) = self.connect_focus_reset() {
            godot_error!("Cannot initialize client: {error}");
            self.base().get_tree().quit_ex().exit_code(1).done();
        }
    }
}

#[godot_api]
impl GameClient {
    #[signal]
    fn screen_requested(screen: GString);

    /// Applies the graphics options to the root viewport.
    #[func]
    fn apply_display_options(&mut self) {
        let mut viewport = self
            .base()
            .get_viewport()
            .expect("GameClient has no viewport");
        display_options::apply_graphics_display_options(
            &self.client_options.graphics,
            &mut viewport,
        );
    }

    #[func]
    fn fps_overlay_enabled(&self) -> bool {
        self.client_options.hud.show_fps_overlay
    }

    #[func]
    fn clear_physical_input(&mut self) {
        self.physical_input.clear();
    }

    /// Test hook for missing-asset handling: point `asset` ("cursor" or "target_ring")
    /// at texture `fdid`. The next frame loads it again.
    #[func]
    fn override_texture_fdid(&mut self, asset: GString, fdid: i64) -> GString {
        let Ok(fdid) = u32::try_from(fdid) else {
            return GString::from(format!("Texture FDID {fdid} out of range").as_str());
        };
        match asset.to_string().as_str() {
            "cursor" => self.merchant.override_cursor_fdid(fdid),
            "target_ring" => self.targeting.override_ring_fdid(fdid),
            other => return GString::from(format!("No texture override for {other}").as_str()),
        }
        GString::new()
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
            .map_err(FrameError::from)
            .and_then(|()| Ok(self.reset_world()?));
        match connection {
            Ok(()) => GString::new(),
            Err(error) => GString::from(error.to_string().as_str()),
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
        state.set(
            "unit_count",
            self.replica
                .units()
                .filter(|unit| replicated::is_unit(*unit))
                .count() as i64,
        );
        state.set("world_attached", self.world.root().is_some());
        state.set(
            "zone_id",
            &self
                .current_zone_id()
                .map(|id| id.to_variant())
                .unwrap_or_default(),
        );
        state.set("terrain", &terrain::state::terrain_state(&self.terrain));
        let area_id = local_transform.and_then(|transform| {
            self.terrain
                .area_id_at(transform.origin.x, transform.origin.z)
        });
        state.set(
            "area_id",
            &area_id.map(|id| id.to_variant()).unwrap_or_default(),
        );
        let mut objects = VarDictionary::new();
        objects.set("spawned", self.world_objects.spawned_count() as i64);
        objects.set("pending", self.world_objects.pending_count() as i64);
        objects.set("failures", self.world_objects.failure_count() as i64);
        if let Some(particles) = self.world_objects.particle_state() {
            objects.set("particles", &particles);
        }
        state.set("world_objects", &objects);
        state.set(
            "local_player_position",
            &local_transform
                .map(|transform| transform.origin.to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "local_server_position",
            &self
                .world
                .local_player_server_position()
                .map(|position| position.to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "local_player_id",
            &self
                .world
                .local_player_id()
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "local_player_facing",
            &self
                .world
                .local_player_facing()
                .map(|yaw| yaw.to_variant())
                .unwrap_or_default(),
        );
        state.set("camera_yaw", self.world_camera.yaw());
        state.set("camera_pitch", self.world_camera.pitch());
        state.set("camera_distance", self.world_camera.distance());
        state.set("local_player_swimming", self.player_movement.swimming);
        state.set(
            "local_server_speed",
            &self
                .world
                .local_player_id()
                .and_then(|id| {
                    self.replica
                        .unit(id)?
                        .get::<shared::components::MovementSpeed>()
                })
                .map(|speed| speed.0.to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "local_player_health",
            &self
                .world
                .local_player_id()
                .and_then(|id| self.replica.unit(id)?.get::<shared::components::Health>())
                .map(|health| health.current.to_variant())
                .unwrap_or_default(),
        );
        state.set("reply_received", self.account.reply_received);
        state.set("assets_starting", self.asset_startup.is_some());
        state.set("connected", self.account.is_connected());
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

    /// The selected unit, its name, what the server echoes back, and the ring's owner.
    /// Plates on screen by unit id: name, alpha, occluded, anchor, fill fraction/colour.
    #[func]
    fn nameplate_state(&self) -> VarDictionary {
        self.nameplates_snapshot()
    }

    /// The nameplate rule inputs for unit `id` (enemy, distance, shown, ...).
    #[func]
    fn nameplate_rules(&mut self, id: i64) -> VarDictionary {
        self.nameplate_rule_state(id as u64)
            .unwrap_or_else(|error| {
                godot_error!("Nameplate rules: {error}");
                VarDictionary::new()
            })
    }

    #[func]
    fn target_state(&self) -> VarDictionary {
        self.targeting_snapshot()
    }

    /// BuffFrame/DebuffFrame buttons and TargetFrame aura icons: spell, texture, rect.
    #[func]
    fn aura_state(&self) -> VarDictionary {
        self.auras_snapshot()
    }

    /// The vendor session: open vendor, its items, buyback, bag items, money, cursor.
    #[func]
    fn merchant_state(&self) -> VarDictionary {
        self.merchant_snapshot()
    }

    #[func]
    fn auction_state(&self) -> VarDictionary {
        self.auction_snapshot()
    }

    /// Read-only receiving mail state; requests only come from real mailbox/frame input.
    #[func]
    fn mail_state(&self) -> VarDictionary {
        self.mailbox_snapshot()
    }

    /// Spell visual kits started, kit models and missiles shown.
    #[func]
    fn spell_visuals_state(&self) -> VarDictionary {
        self.spell_visuals_snapshot()
    }

    /// Unit `id`'s node transform (world space), or nil.
    #[func]
    fn unit_transform(&self, id: i64) -> Variant {
        self.world
            .unit_node(id as u64)
            .map(|node| node.get_global_transform().to_variant())
            .unwrap_or_default()
    }

    /// The combat/spell clip layered over unit `id`'s locomotion, or -1.
    #[func]
    fn unit_action_id(&self, id: i64) -> i64 {
        self.world.unit_action_id(id as u64).map_or(-1, i64::from)
    }

    /// Unit `id`'s creature `display_id` (-1 without one), whether its `visual` is loaded,
    /// and its locomotion `animation` (-1 before one); empty for an unknown unit.
    #[func]
    fn unit_display(&self, id: i64) -> VarDictionary {
        let mut state = VarDictionary::new();
        if let Some((display_id, visual, animation)) = self.world.unit_display(id as u64) {
            state.set("display_id", display_id.map_or(-1, i64::from));
            state.set("visual", visual);
            state.set("animation", animation.map_or(-1, i64::from));
        }
        if let Some(rate) = self.world.unit_animation_rate(id as u64) {
            state.set("animation_rate", rate);
        }
        state
    }

    /// Known spells, bar, cooldowns, sent casts, errors and spellbook entries.
    #[func]
    fn spells_state(&self) -> VarDictionary {
        self.spells_snapshot()
    }

    /// The shown fill of mirror timer `timer`'s bar, or nil while it is not running.
    #[func]
    fn mirror_timer_fraction(&self, timer: i64) -> Variant {
        mirror_timers::mirror_timer_kind(timer)
            .ok()
            .and_then(|kind| self.mirror_fraction(kind))
            .map(|fraction| fraction.to_variant())
            .unwrap_or_default()
    }

    /// Terrain water surface at world X/Z, or nil where none is loaded.
    #[func]
    fn water_surface_at(&self, x: f32, z: f32) -> Variant {
        self.terrain
            .water_surface_at(x, z)
            .map(|height| height.to_variant())
            .unwrap_or_default()
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
    /// Report a failed frame step. A session failure stops the account and returns true;
    /// a client failure is reported once and the session continues.
    fn handle_frame_error(&mut self, step: &str, error: FrameError) -> bool {
        let error = match error {
            FrameError::Client(error) => {
                frame_error::report_once(&format!("{step}: {error}"));
                return false;
            }
            FrameError::Session(error) => error.0,
        };
        self.account.session.feedback = Some(error.clone());
        godot_error!("Account session failed in {step}: {error}");
        if let Err(ui_error) = self.update_login_status(&error, false) {
            godot_error!("Login feedback failed: {ui_error}");
        }
        if let Err(stop_error) = self.account.stop() {
            godot_error!("Account shutdown failed: {stop_error}");
        }
        true
    }

    fn effective_ui_scale(&self) -> f32 {
        let viewport = self
            .base()
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or_default();
        ui_scale::effective_ui_scale(
            [viewport.x, viewport.y],
            self.client_options.graphics.ui_scale,
            self.account.session.screen == SessionScreen::InWorld,
        )
    }

    fn for_each_registry_ui(
        &mut self,
        mut visit: impl FnMut(&mut Gd<ui::RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [
            &mut self.login_ui,
            &mut self.character_ui,
            &mut self.create_ui,
            &mut self.loading_ui,
            &mut self.errors_ui,
            &mut self.mirror_timer_ui,
            &mut self.chat.ui,
            &mut self.game_menu_ui,
            &mut self.world_map.ui,
        ] {
            if let Some(ui) = ui {
                visit(ui)?;
            }
        }
        if let Some(ui) = &mut self.bags.ui {
            visit(ui)?;
        }
        self.merchant.visit_uis(&mut visit)?;
        if let Some(ui) = &mut self.mailbox.ui {
            visit(ui)?;
        }
        self.loot.visit_uis(&mut visit)?;
        if let Some(ui) = &mut self.auction.ui {
            visit(ui)?;
        }
        self.spells.visit_uis(&mut visit)?;
        self.targeting.visit_uis(&mut visit)?;
        self.minimap.visit_uis(&mut visit)?;
        self.objective_tracker.visit_uis(&mut visit)?;
        self.auras.visit_uis(&mut visit)?;
        self.damage_meter.visit_uis(&mut visit)?;
        self.entrance_bar.visit_uis(&mut visit)
    }

    fn sync_registry_ui_scale(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        self.for_each_registry_ui(|ui| ui.bind_mut().set_ui_scale(scale))
    }

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

    fn poll_ui_actions(&mut self) -> Result<(), FrameError> {
        if self.game_menu_ui.is_some() {
            return Ok(self.poll_game_menu_actions()?);
        }
        match self.account.session.screen {
            SessionScreen::Login => self.poll_login_actions(),
            SessionScreen::CharacterSelect => self.poll_character_actions(),
            SessionScreen::CharacterCreate => self.poll_create_actions(),
            _ => Ok(()),
        }
    }

    fn poll_character_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.character_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        let typed = ui.bind_mut().frame_text(DELETE_CONFIRM_INPUT.0.into());
        let action = ui.bind_mut().pop_action().to_string();
        self.update_delete_typed_text(&typed.to_string())?;
        let Some(parsed) = CharSelectAction::parse(&action) else {
            if action.is_empty() {
                return Ok(());
            }
            return Err(format!("Character action not yet converted: {action}").into());
        };
        match parsed {
            CharSelectAction::SelectChar(index) => Ok(self.select_character(Some(index))?),
            CharSelectAction::EnterWorld => Ok(self.account.send_enter_world()?),
            CharSelectAction::CreateToggle => {
                self.account.session.screen = SessionScreen::CharacterCreate;
                Ok(self.show_account_screen(SessionScreen::CharacterCreate)?)
            }
            CharSelectAction::DeleteChar => Ok(self.open_delete_confirmation()?),
            CharSelectAction::ConfirmDeleteChar => self.confirm_delete_character(),
            CharSelectAction::CancelDeleteChar => {
                self.delete_confirmation.clear();
                Ok(self.sync_delete_confirmation()?)
            }
            CharSelectAction::Back => {
                self.account.session.screen = SessionScreen::Login;
                Ok(self.show_account_screen(SessionScreen::Login)?)
            }
            CharSelectAction::Menu => Ok(self.open_game_menu()?),
            CharSelectAction::CampsiteToggle => {
                self.campsite.panel_visible = !self.campsite.panel_visible;
                Ok(self.sync_campsite_state()?)
            }
            CharSelectAction::CampsitePage(page) => {
                self.campsite.page = page;
                Ok(self.sync_campsite_state()?)
            }
            CharSelectAction::SelectCampsite(id) => {
                self.character_preview.select_scene(id);
                self.campsite.selected_id = Some(id);
                self.campsite.panel_visible = false;
                Ok(self.sync_campsite_state()?)
            }
        }
    }

    /// Original character-select keys: Up/Down navigate, Enter enters or confirms deletion,
    /// Escape cancels a pending deletion.
    fn handle_character_select_key(&mut self, key: godot::global::Key) -> Result<(), FrameError> {
        use godot::global::Key;
        let deleting = self.delete_confirmation.target.is_some();
        match key {
            Key::ESCAPE if deleting => {
                self.delete_confirmation.clear();
                Ok(self.sync_delete_confirmation()?)
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
                Ok(self.select_character(next)?)
            }
            Key::ENTER | Key::KP_ENTER if self.account.session.selected_index.is_some() => {
                Ok(self.account.send_enter_world()?)
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

    fn sync_campsite_state(&mut self) -> Result<(), String> {
        match self.character_ui.as_mut() {
            Some(ui) => ui.bind_mut().set_state(self.campsite.clone()),
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

    fn confirm_delete_character(&mut self) -> Result<(), FrameError> {
        if !self.delete_confirmation.ready() {
            return Ok(());
        }
        let Some(target) = self.delete_confirmation.target.take() else {
            return Ok(());
        };
        self.account.send_delete_character(target.character_id)?;
        self.delete_confirmation.clear();
        Ok(self.sync_delete_confirmation()?)
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

    fn poll_create_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.create_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
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
            return Ok(self.sync_creation_ui()?);
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
        Ok(self.sync_creation_ui()?)
    }

    fn apply_creation_effect(
        &mut self,
        effect: char_create::CharCreateEffect,
    ) -> Result<(), FrameError> {
        use char_create::CharCreateEffect;
        match effect {
            CharCreateEffect::ExitToCharSelect => {
                self.creation = None;
                self.account.session.screen = SessionScreen::CharacterSelect;
                Ok(self.show_account_screen(SessionScreen::CharacterSelect)?)
            }
            // The edit box shows `CharCreateState::name` through the next UI state.
            CharCreateEffect::SetNameText(name) => {
                if let Some(state) = self.creation.as_mut() {
                    state.name = name;
                }
                Ok(())
            }
            CharCreateEffect::SendCreate(request) => {
                Ok(self.account.send_create_character(request)?)
            }
            CharCreateEffect::FocusNameInput => match self.create_ui.as_mut() {
                Some(ui) if ui.bind().has_frame(CREATE_NAME_INPUT.0) => {
                    Ok(ui.bind_mut().focus_frame_named(CREATE_NAME_INPUT.0)?)
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
    ) -> Result<(), FrameError> {
        let Some(state) = self.creation.as_mut() else {
            return Ok(());
        };
        match char_create::receive_create_result(state, success, error) {
            Some(effect) => self.apply_creation_effect(effect),
            None => Ok(self.sync_creation_ui()?),
        }
    }

    fn poll_login_actions(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::Login {
            return Ok(());
        }
        let Some(login) = self.login_ui.as_mut() else {
            return Ok(());
        };
        let error = login.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        let action = login.bind_mut().pop_action().to_string();
        match action.as_str() {
            "" => Ok(()),
            "connect" => {
                let credentials = login.bind().credentials();
                let username = credential_field(&credentials, "username")?;
                let password = credential_field(&credentials, "password")?;
                if username.trim().is_empty() || password.trim().is_empty() {
                    return Ok(self.update_login_status("Please fill in all fields", false)?);
                }
                self.account
                    .connect(&self.server_hostname, &username, &password, false)?;
                self.reset_world()?;
                Ok(self.update_login_status("Connecting...", true)?)
            }
            "reconnect" => {
                self.account.connect(&self.server_hostname, "", "", false)?;
                self.reset_world()?;
                Ok(self.update_login_status("Connecting...", true)?)
            }
            "exit" => {
                self.base().get_tree().quit();
                Ok(())
            }
            other => Err(format!("Login action not yet converted: {other}").into()),
        }
    }

    fn advance_login_fade(&mut self, delta: f32) -> Result<(), String> {
        match self.login_ui.as_mut() {
            Some(login) => login.bind_mut().advance_login_fade(delta),
            None => Ok(()),
        }
    }

    fn show_session_feedback(&mut self) -> Result<(), String> {
        let status = self.account.session.feedback.clone().unwrap_or_default();
        self.update_login_status(&status, false)
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

    /// A failure handling one event is reported and the next event still applies;
    /// only a transport or protocol failure ends the poll.
    /// While CASC initializes, the session still receives its traffic (a login reply
    /// updates the session at once); events that show screens or units wait for it.
    fn receive_account_during_startup(&mut self) {
        match self.account.poll() {
            Ok(events) => self.startup_events.extend(events),
            Err(error) => {
                self.handle_frame_error("Account", error.into());
            }
        }
    }

    fn poll_account(&mut self) -> Result<(), FrameError> {
        let mut events = std::mem::take(&mut self.startup_events);
        events.extend(self.account.poll()?);
        for event in events {
            match self.apply_account_event(event) {
                Err(FrameError::Client(error)) => {
                    frame_error::report_once(&format!("Account event: {error}"))
                }
                result => result?,
            }
        }
        self.world
            .select_local_player(self.account.session.selected_character_name.as_deref());
        self.world
            .update_visibility(&self.replica, self.world_minutes);
        self.account
            .session
            .finish_reconnect(self.world.local_player_node().is_some());
        Ok(())
    }

    fn apply_account_event(&mut self, event: AccountEvent) -> Result<(), FrameError> {
        match event {
            AccountEvent::Screen(screen) => {
                self.show_account_screen(screen)?;
                self.show_session_feedback()?;
            }
            AccountEvent::Feedback => self.show_session_feedback()?,
            AccountEvent::WorldReset => self.reset_world()?,
            AccountEvent::RestState(update) => {
                self.in_rest_area = update.snapshot.is_some_and(|rest| rest.in_rest_area);
            }
            AccountEvent::LoadTerrain(request) => self.request_terrain(request)?,
            AccountEvent::NewWorld(destination) => self.transfer_world(destination)?,
            AccountEvent::TransferError(error) => self.add_world_error(&error)?,
            AccountEvent::CastFailed(failed) => self.show_cast_failed(failed)?,
            AccountEvent::Combat(message) => self.receive_combat_message(message)?,
            AccountEvent::Mail(message) => self.receive_mail(message)?,
            AccountEvent::ReplicationStarted(schema) => self.start_replication(schema)?,
            AccountEvent::Replication(batch) => self.apply_replication(batch)?,
            AccountEvent::ReplicationEnded => {
                self.replica.clear();
                self.project_replication()?;
            }
            AccountEvent::RosterChanged => self.sync_character_select_state()?,
            AccountEvent::CharacterCreated { success, error } => {
                self.receive_creation_result(success, error)?
            }
            AccountEvent::MirrorTimer(message) => self.receive_mirror_timer(message)?,
            AccountEvent::Npc(message) => self.receive_npc_message(message)?,
            AccountEvent::Loot(message) => self.receive_loot_message(message)?,
            AccountEvent::Auction(reply) => self.auction.session.receive(reply),
            AccountEvent::Chat(message) => self.receive_chat(&message),
        }
        Ok(())
    }

    /// A new connection: the previous connection's entities leave the world first.
    fn start_replication(&mut self, schema: std::sync::Arc<Schema>) -> Result<(), String> {
        self.replica.clear();
        self.project_replication()?;
        self.replica = Replica::new(schema);
        Ok(())
    }

    fn apply_replication(&mut self, batch: ReplicationBatch) -> Result<(), String> {
        self.replica
            .apply(batch)
            .map_err(|error| format!("Replication: {error}"))?;
        self.project_replication()
    }

    /// Update the world node of each changed player or creature, and the node of each
    /// changed game object, from their replicated components; an entity that stops being
    /// either, or despawns, loses its node. Every change is applied; failures are joined.
    fn project_replication(&mut self) -> Result<(), String> {
        let mut parent = self.to_gd().upcast::<Node3D>();
        let mut errors = Vec::new();
        for change in self.replica.drain_changes() {
            let (server_id, components) = match change {
                UnitChange::Changed {
                    server_id,
                    components,
                } => (server_id, components),
                UnitChange::Despawned(server_id) => {
                    self.remove_replicated(server_id);
                    continue;
                }
            };
            let unit = self.replica.unit(server_id).expect("changed entity exists");
            if replicated::is_unit(unit) {
                self.world.upsert(&mut parent, unit);
                if self
                    .replica
                    .changed::<shared::components::UnitAuras>(components)
                {
                    self.auras.aura_set_changed(server_id, true);
                }
            } else if let Some(info) = unit.get::<shared::protocol::GameObjectInfo>() {
                errors.extend(self.game_objects.upsert(&mut parent, unit, info).err());
            } else {
                self.remove_replicated(server_id);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    fn remove_replicated(&mut self, server_id: u64) {
        self.game_objects.remove(server_id);
        self.mailbox.close_for(server_id);
        self.loot.lootable.remove(&server_id);
        self.world.remove(server_id);
        self.auras.aura_set_changed(server_id, false);
    }

    fn request_terrain(&mut self, request: shared::protocol::LoadTerrain) -> Result<(), String> {
        let map_changed = self.terrain.state().map.as_deref() != Some(&request.map_name);
        if map_changed {
            self.world_map_id = Some(account::read_map_id(&self.data_root, &request.map_name)?);
        }
        self.terrain.request_map(
            request.map_name,
            (request.initial_tile_y, request.initial_tile_x),
        )?;
        if map_changed {
            self.world_camera.reset();
            self.world_lighting.reset();
            self.world.update_lighting(None);
            self.game_objects.reset();
            self.mailbox.reset();
            self.terrain_materials.reset();
            self.world_objects.reset();
            self.global_wmo.reset();
            self.wmo_collision.reset();
            self.account.session.screen = SessionScreen::Loading;
            self.show_account_screen(SessionScreen::Loading)?;
        }
        Ok(())
    }

    fn transfer_world(&mut self, destination: shared::protocol::NewWorld) -> Result<(), String> {
        self.terrain.reset()?;
        self.world_camera.reset();
        self.terrain_materials.reset();
        self.world_objects.reset();
        self.global_wmo.reset();
        self.wmo_collision.reset();
        self.world_lighting.reset();
        self.world.update_lighting(None);
        self.game_objects.reset();
        self.mailbox.reset();
        let [x, y, z] = destination.position;
        let tile = game_engine_core::terrain_height_data::bevy_to_tile_coords(x, z);
        self.terrain.request_map(destination.map_directory, tile)?;
        self.world_map_id = Some(destination.map_id);
        if let Some(mut player) = self.world.local_player_node() {
            player.set_position(Vector3::new(x, y, z));
            player.set_rotation(Vector3::new(0.0, destination.facing, 0.0));
            self.world
                .set_local_player_facing(destination.facing + std::f32::consts::FRAC_PI_2);
        }
        Ok(())
    }

    /// Stream terrain results. A tile or map that fails to load is reported once and
    /// stays absent; the rest of the map still loads.
    fn poll_terrain(&mut self) -> Result<(), String> {
        self.terrain.poll()?;
        if let Some(error) = self.terrain.map_error() {
            frame_error::report_once(&format!("Terrain map: {error}"));
        }
        for ((y, x), error) in self.terrain.failures() {
            frame_error::report_once(&format!("Terrain tile ({y}, {x}): {error}"));
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
        let map_id = self.world_map_id.ok_or("Loaded terrain has no map ID")?;
        // WebWowViewerCpp applies the MFOG fog of the WMO interior the camera is in.
        let wmo_fog = self
            .world_camera
            .position()
            .and_then(|camera| self.world_objects.camera_fog(camera));
        if let Some(light) = self.world_lighting.sync(
            &mut parent,
            &wdt.lighting,
            map_id,
            player.origin,
            self.world_minutes,
            wmo_fog.as_ref(),
        )? {
            self.world.update_lighting(Some(light.clone()));
            self.game_objects.update_lighting(Some(light.clone()));
            self.world_objects.update_lighting(&light);
            self.global_wmo.update_lighting(&light);
            self.terrain_materials.update_lighting(light);
        }
        Ok(())
    }

    fn attach_world_objects(&mut self) {
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.world_objects
            .sync(&mut parent, &self.terrain, &terrain::objects::AllObjects);
        if let Some(player) = self.world.local_player_transform() {
            let origin = player.origin;
            self.wmo_collision.sync(
                &mut parent,
                &self.terrain,
                glam::Vec3::new(origin.x, origin.y, origin.z),
            );
        }
    }

    fn cull_world_objects(&mut self) {
        if let Some(camera) = self.world_camera.position() {
            let frustum = self.world_camera.frustum();
            let frame = godot::classes::Engine::singleton().get_process_frames();
            let delta_ms = self.base().get_process_delta_time() * 1000.0;
            // Portal culling first: it decides which WMO doodads are drawn this frame.
            self.world_objects.cull_wmos(camera, &frustum);
            self.world_objects
                .cull_doodads(camera, &frustum, delta_ms, frame);
            if let Some(transform) = self.world_camera.transform() {
                self.world_objects.update_particles(
                    transform,
                    &frustum,
                    (delta_ms / 1000.0) as f32,
                );
            }
            self.world.apply_animation_lod(camera, frame);
        }
    }

    fn attach_terrain_materials(&mut self) -> Result<(), String> {
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.terrain_materials.sync(&mut parent, &self.terrain)
    }

    fn update_loading_readiness(&mut self, delta: f32) -> Result<(), FrameError> {
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
        let mut parent = self.to_gd().upcast::<Node3D>();
        let global_wmo = self.global_wmo.sync(&mut parent, &self.terrain);
        if let Some(wmo) = self.global_wmo.take_spawned() {
            self.world_objects
                .adopt_wmo(wmo.unique_id, &wmo.node, wmo.doodads, wmo.culled);
        }
        let state = self.terrain.state();
        let readiness = loading::evaluate_native_loading(
            position,
            &state,
            self.terrain_materials.attached_tiles(),
            self.terrain_materials.failures(),
            global_wmo,
        );
        if let Some(ui) = self.loading_ui.as_mut() {
            ui.bind_mut().advance_loading(
                readiness.progress_percent,
                readiness.status_text,
                delta,
            )?;
        }
        if readiness.complete {
            self.account.session.screen = SessionScreen::InWorld;
            self.show_account_screen(SessionScreen::InWorld)?;
            self.account.finish_world_port()?;
        }
        Ok(())
    }

    fn update_world_camera(&mut self, delta: f32) -> Result<(), String> {
        // A WMO-only map (a dungeon) has no ADT tiles; its world is the global WMO.
        if self.terrain.parsed_tiles.is_empty() && !self.terrain.state().global_wmo_present {
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
        self.stop_sound();
        self.logout.clear();
        self.loot.reset();
        self.game_objects.reset();
        self.mailbox.reset();
        self.in_rest_area = false;
        self.character_preview.reset();
        self.creation_scene.reset();
        self.physical_input.clear();
        self.player_movement = gameplay::PlayerMovement::default();
        if let Some(ui) = self.errors_ui.as_mut() {
            ui.bind_mut().clear_errors()?;
            ui.set_visible(false);
        }
        self.world_camera.reset();
        self.world_lighting.reset();
        self.world_map_id = None;
        self.entrance_bar.close();
        self.terrain_materials.reset();
        self.world_objects.reset();
        self.global_wmo.reset();
        self.wmo_collision.reset();
        self.spell_effects.reset();
        self.world.reset();
        self.replica.clear();
        self.replica.drain_changes();
        self.auras.reset();
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
        let drag = self.account_orbit_drag();
        let mut parent = self.to_gd().upcast::<Node3D>();
        self.character_preview
            .sync(&mut parent, selected.as_ref(), self.world_minutes, drag)
    }

    /// Original account-scene orbit: left-drag motion scaled by camera sensitivity.
    fn account_orbit_drag(&self) -> glam::Vec2 {
        use game_engine_core::input_bindings_data::{BindingMouseButton, InputState};
        if !self.physical_input.mouse_pressed(BindingMouseButton::Left) {
            return glam::Vec2::ZERO;
        }
        game_engine_core::char_select_camera_data::scaled_orbit_delta(
            glam::Vec2::from_array(self.physical_input.motion()),
            self.client_options.camera.mouse_sensitivity,
        )
    }

    fn update_creation_scene(&mut self, delta: f32) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::CharacterCreate {
            self.creation_scene.reset();
            return Ok(());
        }
        let drag = self.account_orbit_drag();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Character creation scene has no viewport")?
            .get_visible_rect()
            .size;
        let mut parent = self.to_gd().upcast::<Node3D>();
        let (Some(state), Some(db)) = (self.creation.as_mut(), self.creation_catalog.as_ref())
        else {
            self.creation_scene.reset();
            return Ok(());
        };
        self.creation_scene.sync(
            &mut parent,
            state,
            db,
            size.x / size.y.max(1.0),
            drag,
            delta,
        )
    }

    fn show_account_screen(&mut self, screen: SessionScreen) -> Result<(), String> {
        self.close_game_menu();
        if screen != SessionScreen::InWorld {
            self.logout.clear();
            self.sync_logout_overlay()?;
        }
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
            SessionScreen::GameMenu => {}
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
        self.apply_startup_customize(screen)?;
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
        let result =
            authored_campsites(&self.data_root, self.campsite.selected_id).and_then(|campsite| {
                self.campsite = campsite;
                ui.bind_mut().set_state(state)?;
                ui.bind_mut().set_state(self.campsite.clone())
            });
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

/// Campsite selector entries from the authored Warband catalog, panel closed.
fn authored_campsites(
    data_root: &std::path::Path,
    selected: Option<u32>,
) -> Result<CampsiteState, String> {
    use game_engine_core::warband_scene_data::{read_authored_catalog, read_texture_kit_art};
    let catalog = read_authored_catalog(data_root)?;
    let kits: Vec<u32> = catalog
        .scenes
        .iter()
        .map(|scene| scene.texture_kit)
        .collect();
    let art = read_texture_kit_art(data_root, &kits)?;
    Ok(CampsiteState {
        selected_id: selected.or_else(|| catalog.scenes.first().map(|scene| scene.id)),
        scenes: catalog
            .scenes
            .iter()
            .map(|scene| CampsiteEntry {
                id: scene.id,
                name: scene.name.clone(),
                preview_image: art.get(&scene.texture_kit).map(|art| CampsitePreview {
                    fdid: art.fdid,
                    tex_coords: art.tex_coords,
                }),
            })
            .collect(),
        panel_visible: false,
        page: 0,
    })
}
