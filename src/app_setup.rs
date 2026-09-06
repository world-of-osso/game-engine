use super::*;
use std::io::Write;

pub(crate) fn gizmos_enabled_for_app(app: &App) -> bool {
    game::inworld_scene_stage::configured_inworld_scene_stage_for_app(app)
        != InWorldSceneStage::Empty
}

fn default_plugins(
    enable_sound: bool,
    enable_gizmos: bool,
    enable_pipelining: bool,
) -> bevy::app::PluginGroupBuilder {
    let mut plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            name: Some("com.worldofosso.game-engine".to_string()),
            ..default()
        }),
        ..default()
    });
    if !enable_sound {
        plugins = plugins.disable::<bevy::audio::AudioPlugin>();
    }
    if !enable_gizmos {
        plugins = plugins
            .disable::<bevy::light::LightPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::gizmos_render::GizmoRenderPlugin>();
    }
    apply_pipelining_policy(plugins, enable_pipelining)
}

pub(crate) fn pipelining_enabled(args: &[String]) -> bool {
    !has_flag(args, "--no-pipelined-rendering")
}

fn apply_pipelining_policy(
    plugins: bevy::app::PluginGroupBuilder,
    enabled: bool,
) -> bevy::app::PluginGroupBuilder {
    if enabled {
        plugins
    } else {
        plugins.disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
    }
}

#[cfg(test)]
#[path = "../tests/unit/pipeline_isolation_tests.rs"]
mod pipeline_isolation_tests;

pub(crate) fn run_screenshot_regression_app(
    args: &[String],
    screenshot: Option<ScreenshotRequest>,
) {
    let screenshot = screenshot_regression_request_or_exit(args, screenshot);

    let mut app = App::new();
    app.add_plugins(default_plugins(false, true, pipelining_enabled(args)));
    app.init_state::<game_state::GameState>();
    app.insert_state(game_state::GameState::InWorld);
    app.insert_resource(game_engine::ui::plugin::UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(ui_toolkit::render_texture::BlpLoaderRes(Box::new(
        GameBlpLoader,
    )));
    app.insert_resource(client_options::GraphicsOptions::default());
    app.insert_resource(networking::CurrentZone::default());
    app.init_resource::<terrain::AdtManager>();
    app.init_resource::<terrain_heightmap::TerrainHeightmap>();
    insert_data_resources(&mut app);
    app.add_plugins((
        terrain_material::TerrainMaterialPlugin,
        m2_effect_material::M2EffectMaterialPlugin,
        skybox_m2_material::SkyboxM2MaterialPlugin,
        water_material::WaterMaterialPlugin,
        sky::SkyPlugin,
    ));
    app.insert_resource(screenshot);
    app.add_systems(Startup, log_window_backend);
    app.add_systems(PostStartup, setup_explicit_asset_scene);
    app.add_systems(Update, take_regression_screenshot);
    app.run();
}

fn screenshot_regression_request_or_exit(
    args: &[String],
    screenshot: Option<ScreenshotRequest>,
) -> ScreenshotRequest {
    let screenshot = screenshot.unwrap_or_else(|| {
        eprintln!("--screenshot-regression requires `screenshot <OUT>`");
        std::process::exit(1);
    });
    if parse_asset_path_from_args(args).is_none() {
        eprintln!("--screenshot-regression requires an explicit asset path");
        std::process::exit(1);
    }
    screenshot
}

fn take_regression_screenshot(mut commands: Commands, req: Option<ResMut<ScreenshotRequest>>) {
    let Some(mut req) = req else { return };
    if req.frames_remaining > 0 {
        req.frames_remaining -= 1;
        return;
    }
    commands.remove_resource::<ScreenshotRequest>();
    let output = req.output.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            save_regression_screenshot(&trigger.image, &output);
            exit.write(AppExit::Success);
        },
    );
}

fn save_regression_screenshot(img: &bevy::image::Image, output: &PathBuf) {
    let webp_data = match game_engine::screenshot::encode_webp(
        img,
        game_engine::screenshot::DEFAULT_WEBP_QUALITY,
    ) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    std::fs::write(output, &webp_data)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

fn register_bevy_plugins(
    app: &mut App,
    enable_sound: bool,
    enable_gizmos: bool,
    enable_pipelining: bool,
) {
    app.add_plugins(default_plugins(
        enable_sound,
        enable_gizmos,
        enable_pipelining,
    ));
    if !enable_pipelining {
        info!("Pipelined rendering disabled: render work runs sequentially on the main thread");
    }
    register_exit_diagnostics(app);
    register_ui_plugins(app);
    register_world_plugins(app);
    register_render_plugins(app);
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            refresh_interval: Duration::from_millis(500),
            ..default()
        },
    });
}

const EXIT_DIAGNOSTICS_PATH: &str = "/tmp/game-engine-exit-diagnostics.log";

#[derive(Resource)]
struct ExitDiagnosticsGuard;

impl Drop for ExitDiagnosticsGuard {
    fn drop(&mut self) {
        write_exit_diagnostic("ExitDiagnosticsGuard dropped");
    }
}

fn register_exit_diagnostics(app: &mut App) {
    write_exit_diagnostic("exit diagnostics plugin registered");
    app.insert_resource(ExitDiagnosticsGuard)
        .add_systems(Startup, arm_exit_diagnostics)
        .add_systems(PreUpdate, log_window_exit_messages)
        .add_systems(Last, log_app_exit_messages);
}

fn arm_exit_diagnostics() {
    write_exit_diagnostic("Startup schedule reached");
    warn!("Exit diagnostics armed");
}

fn write_exit_diagnostic(message: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(EXIT_DIAGNOSTICS_PATH)
        .expect("failed to open exit diagnostics log");
    writeln!(
        file,
        "{:?} pid={} {message}",
        std::time::SystemTime::now(),
        std::process::id()
    )
    .expect("failed to write exit diagnostics log");
}

fn log_window_exit_messages(
    mut close_requested: MessageReader<bevy::window::WindowCloseRequested>,
    mut closed: MessageReader<bevy::window::WindowClosed>,
    mut destroyed: MessageReader<bevy::window::WindowDestroyed>,
) {
    for event in close_requested.read() {
        write_exit_diagnostic(&format!("WindowCloseRequested window={:?}", event.window));
        warn!(window = ?event.window, "Exit diagnostics: WindowCloseRequested");
    }
    for event in closed.read() {
        write_exit_diagnostic(&format!("WindowClosed window={:?}", event.window));
        warn!(window = ?event.window, "Exit diagnostics: WindowClosed");
    }
    for event in destroyed.read() {
        write_exit_diagnostic(&format!("WindowDestroyed window={:?}", event.window));
        warn!(window = ?event.window, "Exit diagnostics: WindowDestroyed");
    }
}

fn log_app_exit_messages(mut exits: MessageReader<AppExit>) {
    for exit in exits.read() {
        write_exit_diagnostic(&format!("AppExit {exit:?}"));
        warn!(?exit, "Exit diagnostics: AppExit");
    }
}

fn register_ui_plugins(app: &mut App) {
    app.add_plugins(game_engine::auction_house::AuctionHousePlugin)
        .add_plugins(game_engine::collection::CollectionPlugin)
        .add_plugins(game_engine::duel::DuelPlugin)
        .add_plugins(game_engine::encounter_journal::EncounterJournalPlugin)
        .add_plugins(game_engine::inspect::InspectPlugin)
        .add_plugins(game_engine::currency::CurrencyPlugin)
        .add_plugins(logout::LogoutPlugin)
        .add_plugins(game_engine::profession::ProfessionPlugin)
        .add_plugins(game_engine::talent::TalentPlugin)
        .add_plugins(game_engine::trade::TradePlugin)
        .add_plugins(game_engine::mail::MailPlugin)
        .add_plugins(game_engine::ui::plugin::UiPlugin)
        .add_plugins(game_engine::ui::automation::UiAutomationPlugin)
        .add_plugins(IpcPlugin)
        .add_plugins(client_options::ClientOptionsPlugin);
}

fn register_world_plugins(app: &mut App) {
    app.add_plugins(WowCameraPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(CollisionPlugin)
        .add_plugins(game_engine::culling::CullingPlugin)
        .add_plugins(AdtStreamingPlugin)
        .add_plugins(minimap::MinimapPlugin)
        .add_plugins(action_bar::ActionBarPlugin)
        .add_plugins(unit_frames::InWorldUnitFramesPlugin)
        .add_plugins(health_bar::HealthBarPlugin)
        .add_plugins(nameplate::NameplatePlugin)
        .add_plugins(quest_sparkle::QuestSparklePlugin)
        .add_plugins(target::TargetPlugin)
        .add_plugins(equipment::EquipmentPlugin)
        .add_plugins(character_customization::CharacterCustomizationPlugin);
}

pub(crate) fn particle_plugin_is_enabled(stage: Option<InWorldSceneStage>) -> bool {
    game::inworld_scene_stage::effective_inworld_scene_stage(stage)
        .includes(InWorldSceneStage::Particles)
}

fn m2_effect_material_plugin_is_enabled(stage: Option<InWorldSceneStage>) -> bool {
    game::inworld_scene_stage::effective_inworld_scene_stage(stage)
        .includes(InWorldSceneStage::Character)
}

fn register_render_plugins(app: &mut App) {
    let stage = app.world().get_resource::<InWorldSceneStage>().copied();
    let m2_effect_material_plugin_enabled = m2_effect_material_plugin_is_enabled(stage);
    let particle_plugin_enabled = particle_plugin_is_enabled(stage);

    app.add_plugins(terrain_material::TerrainMaterialPlugin);
    if m2_effect_material_plugin_enabled {
        app.add_plugins(m2_effect_material::M2EffectMaterialPlugin);
    } else {
        app.init_asset::<m2_effect_material::M2EffectMaterial>();
    }
    app.add_plugins(skybox_m2_material::SkyboxM2MaterialPlugin)
        .add_plugins(water_material::WaterMaterialPlugin)
        .add_plugins(sky::SkyPlugin);
    if particle_plugin_enabled {
        app.add_plugins(particle::ParticlePlugin);
    }
    app.add_plugins(weather::WeatherPlugin).add_systems(
        Update,
        terrain_objects::sync_wmo_sidn_emissive
            .run_if(in_state(game_state::GameState::InWorld))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    );
}

pub(crate) fn register_plugins(
    app: &mut App,
    enable_sound: bool,
    enable_gizmos: bool,
    enable_pipelining: bool,
) {
    register_bevy_plugins(app, enable_sound, enable_gizmos, enable_pipelining);
    app.insert_resource(ui_toolkit::render_texture::BlpLoaderRes(Box::new(
        GameBlpLoader,
    )));
    app.add_systems(
        Startup,
        (
            log_window_backend,
            setup_explicit_asset_scene,
            wow_cursor::install_wow_cursor
                .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui),
            game_engine::ui::panel_styles::register_panel_styles
                .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui),
        ),
    )
    .add_systems(
        Update,
        wow_cursor::update_wow_cursor_style
            .run_if(in_state(game_state::GameState::InWorld))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui),
    );
    status_sync::init_status_resources(app);
}

fn log_window_backend(display: Option<Res<bevy::winit::DisplayHandleWrapper>>) {
    let Some(display) = display else {
        info!("Window backend: unavailable (no display handle resource)");
        return;
    };
    let Ok(handle) = display.0.display_handle() else {
        warn!("Window backend: unavailable (failed to acquire display handle)");
        return;
    };
    let backend = match handle.as_raw() {
        RawDisplayHandle::Wayland(_) => "Wayland",
        RawDisplayHandle::Xlib(_) => "X11 (Xlib)",
        RawDisplayHandle::Xcb(_) => "X11 (XCB)",
        RawDisplayHandle::Windows(_) => "Windows",
        RawDisplayHandle::AppKit(_) => "AppKit",
        RawDisplayHandle::UiKit(_) => "UIKit",
        RawDisplayHandle::Android(_) => "Android",
        RawDisplayHandle::Web(_) => "Web",
        RawDisplayHandle::Orbital(_) => "Orbital",
        _ => "Unknown",
    };
    info!(
        "Window backend: {backend} (DISPLAY={:?}, WAYLAND_DISPLAY={:?})",
        std::env::var_os("DISPLAY"),
        std::env::var_os("WAYLAND_DISPLAY")
    );
}

struct GameBlpLoader;

impl ui_toolkit::render_texture::BlpLoader for GameBlpLoader {
    fn load_blp_to_image(&self, path: &std::path::Path) -> Result<bevy::image::Image, String> {
        game_engine::asset::blp::load_blp_to_image(path)
    }
    fn load_blp_gpu_image(&self, path: &std::path::Path) -> Result<bevy::image::Image, String> {
        game_engine::asset::blp::load_blp_gpu_image(path)
    }
    fn ensure_texture(&self, fdid: u32) -> Option<PathBuf> {
        let path = PathBuf::from(format!("data/textures/{fdid}.blp"));
        if path.exists() { Some(path) } else { None }
    }
}

fn configure_server_resources(
    app: &mut App,
    enable_sound: bool,
    server_arg: Option<cli_args::ServerArg>,
    server_override: bool,
    initial_state: Option<game_state::GameState>,
    startup_login: Option<(String, String)>,
) {
    add_optional_sound_plugin(app, enable_sound);
    let server_arg = resolve_server_arg(
        server_arg,
        initial_state,
        client_options::load_preferred_realm(),
    );
    insert_server_resources(app, server_arg);
    app.insert_resource(scenes::login::LoginRealmSelectionLock(server_override));
    insert_initial_state_resource(app, initial_state);
    insert_startup_login_resources(app, startup_login);
}

fn add_optional_sound_plugin(app: &mut App, enable_sound: bool) {
    if enable_sound {
        app.add_plugins(sound::SoundPlugin);
    }
}

fn resolve_server_arg(
    server_arg: Option<cli_args::ServerArg>,
    initial_state: Option<game_state::GameState>,
    preferred_realm: cli_args::RealmPreset,
) -> Option<cli_args::ServerArg> {
    server_arg.or_else(|| default_connecting_server_arg(initial_state, preferred_realm))
}

pub(crate) fn default_connecting_server_arg(
    initial_state: Option<game_state::GameState>,
    preferred_realm: cli_args::RealmPreset,
) -> Option<cli_args::ServerArg> {
    if initial_state == Some(game_state::GameState::Connecting) {
        Some(default_server_arg_or_exit(preferred_realm))
    } else {
        None
    }
}

fn default_server_arg_or_exit(preferred_realm: cli_args::RealmPreset) -> cli_args::ServerArg {
    match preferred_realm.to_server_arg() {
        Ok(server) => server,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn insert_server_resources(app: &mut App, server_arg: Option<cli_args::ServerArg>) {
    let Some(server) = server_arg else {
        return;
    };
    app.insert_resource(networking::ServerAddr(server.addr));
    app.insert_resource(networking::ServerHostname(server.hostname));
    if server.dev {
        app.insert_resource(scenes::login::DevServer);
    }
}

fn insert_initial_state_resource(app: &mut App, initial_state: Option<game_state::GameState>) {
    if let Some(state) = initial_state {
        app.insert_resource(game_state::InitialGameState(state));
    }
}

fn insert_startup_login_resources(app: &mut App, startup_login: Option<(String, String)>) {
    if let Some((username, password)) = startup_login {
        app.insert_resource(networking::LoginUsername(username));
        app.insert_resource(networking::LoginPassword(password));
        app.insert_resource(networking::LoginMode::Login);
    }
}

pub(crate) fn configure_app_plugins(app: &mut App, enable_sound: bool, parsed: &mut ParsedArgs) {
    #[cfg(debug_assertions)]
    game_engine::ui::screen::init_global_hot_reload(vec![std::path::PathBuf::from(
        "src/ui/screens",
    )]);

    configure_server_resources(
        app,
        enable_sound,
        parsed.server_addr.take(),
        parsed.server_override,
        parsed.initial_state,
        parsed.startup_login.clone(),
    );
    add_screen_plugins(app, parsed.initial_state);
    app.add_systems(
        OnEnter(game_state::GameState::InWorld),
        setup_default_world_scene,
    );
    status_sync::register_status_sync_systems(app);
}

fn add_screen_plugins(app: &mut App, initial_state: Option<game_state::GameState>) {
    add_core_screen_plugins(app);
    add_game_runtime_plugins(app);
    add_frame_plugins(app);
    add_misc_runtime_plugins(app);
    add_debug_scene_plugin(app, initial_state);
}

fn add_core_screen_plugins(app: &mut App) {
    app.add_plugins((
        game_state::GameStatePlugin,
        networking::NetworkPlugin,
        scenes::eula::EulaScreenPlugin,
        scenes::login::LoginScreenPlugin,
        scenes::loading::LoadingScreenPlugin,
        scenes::char_select::CharSelectPlugin,
        scenes::char_select::CharSelectScenePlugin,
        scenes::selection_debug::SelectionDebugScreenPlugin,
        scenes::selection_debug::InWorldSelectionDebugScreenPlugin,
        scenes::char_create::CharCreatePlugin,
        scenes::char_create::CharCreateScenePlugin,
        scenes::char_select::campsite::CampsitePopupScreenPlugin,
        scenes::game_menu::GameMenuScreenPlugin,
    ));
}

fn add_game_runtime_plugins(app: &mut App) {
    app.add_plugins((
        game_engine::achievement::AchievementPlugin,
        game_engine::barber_shop::BarberShopPlugin,
        game_engine::death::DeathPlugin,
        game_engine::friends::FriendsPlugin,
        game_engine::ignore_list::IgnoreListPlugin,
        game_engine::lfg::LfgPlugin,
        game_engine::pvp::PvpPlugin,
        game_engine::world_map::WorldMapPlugin,
    ));
}

fn add_frame_plugins(app: &mut App) {
    app.add_plugins((
        scenes::encounter_journal_frame::EncounterJournalFramePlugin,
        scenes::friends_frame::FriendsFramePlugin,
        scenes::guild_frame::GuildFramePlugin,
        scenes::inspect_frame::InspectFramePlugin,
        scenes::achievement_frame::AchievementFramePlugin,
        scenes::bag_frame::BagFramePlugin,
        scenes::calendar_frame::CalendarFramePlugin,
        scenes::professions_frame::ProfessionsFramePlugin,
        scenes::talent_frame::TalentFramePlugin,
        scenes::tooltip_frame::TooltipFramePlugin,
        scenes::world_map_frame::WorldMapFramePlugin,
        trash_button_screen::TrashButtonScreenPlugin,
        orbit_camera::OrbitCameraPlugin,
    ));
}

fn add_misc_runtime_plugins(app: &mut App) {
    app.add_plugins(game_engine::calendar::CalendarPlugin);
    app.add_plugins(game_engine::guild::GuildPlugin);
    app.add_plugins(game_engine::who::WhoPlugin);
    app.add_plugins(game_engine::reputation::ReputationPlugin);
    app.add_plugins(game_engine::ui::addon_runtime::AddonRuntimePlugin);
    app.add_plugins(taxi::TaxiPlugin);
    app.add_plugins(scenes::casting_bar_frame::CastingBarFramePlugin);
    app.add_plugins(scenes::mail_frame::MailFramePlugin);
    app.add_plugins(scenes::merchant_frame::MerchantFramePlugin);
    app.add_plugins(scenes::group_frames::GroupFramesPlugin);
    app.add_plugins(scenes::loot_rules_frame::LootRulesFramePlugin);
}

fn add_debug_scene_plugin(app: &mut App, initial_state: Option<game_state::GameState>) {
    match initial_state {
        Some(game_state::GameState::DebugCharacter) => {
            app.add_plugins(scenes::geoset_debug::DebugCharacterScenePlugin);
        }
        Some(game_state::GameState::M2Debug) => {
            app.add_plugins(scenes::m2_debug::M2DebugScenePlugin);
        }
        Some(game_state::GameState::SkyboxDebug) => {
            app.add_plugins(scenes::skybox_debug::SkyboxDebugScenePlugin);
        }
        Some(game_state::GameState::ParticleDebug) => {
            app.add_plugins(scenes::particle_debug::ParticleDebugScenePlugin);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m2_effect_material_infrastructure_is_stage_gated() {
        let observed = [
            Some(InWorldSceneStage::Empty),
            Some(InWorldSceneStage::Character),
            None,
        ]
        .into_iter()
        .map(|stage| {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<bevy::mesh::Mesh>();
            app.init_asset::<bevy::shader::Shader>();
            if let Some(stage) = stage {
                app.insert_resource(stage);
            }

            register_render_plugins(&mut app);

            (
                stage,
                app.world()
                    .contains_resource::<Assets<m2_effect_material::M2EffectMaterial>>(),
                app.world().contains_resource::<
                    bevy::pbr::EntitiesNeedingSpecialization<
                        m2_effect_material::M2EffectMaterial,
                    >,
                >(),
            )
        })
        .collect::<Vec<_>>();

        assert_eq!(
            observed,
            vec![
                (Some(InWorldSceneStage::Empty), true, false),
                (Some(InWorldSceneStage::Character), true, true),
                (None, true, true),
            ],
        );
    }

    #[test]
    fn gizmos_are_disabled_only_for_explicit_empty_stage() {
        let observed = [
            Some(InWorldSceneStage::Empty),
            Some(InWorldSceneStage::Character),
            None,
        ]
        .into_iter()
        .map(|stage| {
            let mut app = App::new();
            if let Some(stage) = stage {
                app.insert_resource(stage);
            }
            (stage, gizmos_enabled_for_app(&app))
        })
        .collect::<Vec<_>>();

        assert_eq!(
            observed,
            vec![
                (Some(InWorldSceneStage::Empty), false),
                (Some(InWorldSceneStage::Character), true),
                (None, true),
            ]
        );
    }

    #[test]
    fn gizmo_runtime_resources_follow_policy() {
        for enable_gizmos in [false, true] {
            let mut app = App::new();
            app.add_plugins(
                default_plugins(false, enable_gizmos, true)
                    .disable::<bevy::winit::WinitPlugin>()
                    .disable::<bevy::log::LogPlugin>()
                    .disable::<bevy::gilrs::GilrsPlugin>()
                    .disable::<bevy::app::TerminalCtrlCHandlerPlugin>(),
            );

            assert_eq!(
                app.world()
                    .contains_resource::<bevy::prelude::GizmoConfigStore>(),
                enable_gizmos,
            );
            assert_eq!(
                app.world()
                    .contains_resource::<bevy::gizmos::GizmoHandles>(),
                enable_gizmos,
            );
        }
    }

    #[test]
    fn audio_plugin_registration_follows_sound_flag() {
        for enable_sound in [false, true] {
            let mut app = App::new();
            app.add_plugins(
                default_plugins(enable_sound, true, true)
                    .disable::<bevy::winit::WinitPlugin>()
                    .disable::<bevy::log::LogPlugin>()
                    .disable::<bevy::gilrs::GilrsPlugin>()
                    .disable::<bevy::app::TerminalCtrlCHandlerPlugin>(),
            );
            add_optional_sound_plugin(&mut app, enable_sound);

            assert_eq!(
                app.world()
                    .contains_resource::<Assets<bevy::audio::AudioSource>>(),
                enable_sound,
            );
            assert_eq!(
                app.world().contains_resource::<sound::SoundSettings>(),
                enable_sound,
            );
        }
    }
}
