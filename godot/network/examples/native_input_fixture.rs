//! Real Godot UI/input → native movement → owned loopback UDP PlayerInput proof.
//! With the native extension and this example built, run the installed executable
//! at target/debug/examples/native_input_fixture; most modes also need the root launcher.

use std::{
    fs,
    io::{BufRead, BufReader},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{
    self as network, LinkOf, MessageReceiver, MessageSender, NetworkTarget, Replicate,
    ReplicationSender, server,
};
use shared::{
    components::{
        EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Gold, ModelDisplay, Npc,
        Player, Position, UnitFactionTemplate,
    },
    protocol::{
        ActionBarSnapshot, ActionRef, AuthChannel, BagContents, CharacterListEntry,
        CloseInteraction, CombatChannel, CombatEvent, CombatEventType, EnterWorldResponse,
        InteractNpc, InteractionChannel, InteractionKind, InteractionOpened, InventoryChannel,
        InventorySnapshot, KnownSpellsSnapshot, LoadTerrain, LoginRequest, LoginResponse,
        MerchantChannel, NpcFlags, NpcRole, PlayerInput, SelectCharacter, SpellCastIntent,
        TalentChannel, TerrainChannel, VendorInventory, VendorItem,
    },
};

#[path = "fixture_support/mod.rs"]
mod fixture_support;
#[path = "native_input_fixture/footsteps.rs"]
mod footsteps;
#[path = "native_input_fixture/logout.rs"]
mod logout;
#[path = "native_input_fixture/menu.rs"]
mod menu;
#[path = "native_input_fixture/merchant_click.rs"]
mod merchant_click;
#[path = "native_input_fixture/portal_density.rs"]
mod portal_density;
#[path = "native_input_fixture/portal_particles.rs"]
mod portal_particles;
#[path = "native_input_fixture/reset_windows.rs"]
mod reset_windows;
#[path = "native_input_fixture/sound.rs"]
mod sound;
#[path = "native_input_fixture/sound_click.rs"]
mod sound_click;
#[path = "native_input_fixture/swimming.rs"]
mod swimming;

const NAME: &str = "Input Fixture";
const SWIM_START: [f32; 3] = [-8558.0, 144.960_08, 522.0];
const REMOTE_NAME: &str = "Remote Fixture";
const UNEQUIPPED_NAME: &str = "Unequipped Fixture";
const COLLECTION_NAME: &str = "Collection Fixture";
// Authored terrain height; the transfer-only fixture's Y=83 is below this surface.
const FIRST: [f32; 3] = [-8949.0, 112.879_913, 0.0];
const REMOTE: [f32; 3] = [-8946.0, 114.245_974, 0.0];
const TICK: Duration = Duration::from_millis(5);
// Five authored preview loads reached 88s before world readiness on the expanded fixture.
const TIMEOUT: Duration = Duration::from_secs(180);
const OVERLAY_TIMEOUT: Duration = Duration::from_secs(240);
const RELEASE_DRAIN: Duration = Duration::from_millis(250);
const RELEASE_QUIET: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupScreen {
    CharSelect,
    InWorld,
    Overlay,
    Swimming,
    Menu,
    Logout,
    Sound,
    SoundClick,
    MerchantClick,
    Footsteps,
    ResetWindows,
    PortalParticlesEnabled,
    PortalParticlesDisabled,
    PortalDensity,
}

impl StartupScreen {
    fn from_example_args() -> Self {
        let mut args = std::env::args().skip(1);
        let screen = match args.next().as_deref() {
            None => Self::CharSelect,
            Some("inworld") => Self::InWorld,
            Some("overlay") => Self::Overlay,
            Some("swimming") => Self::Swimming,
            Some("menu") => Self::Menu,
            Some("logout") => Self::Logout,
            Some("sound") => Self::Sound,
            Some("sound-click") => Self::SoundClick,
            Some("merchant-click") => Self::MerchantClick,
            Some("footsteps") => Self::Footsteps,
            Some("reset-windows") => Self::ResetWindows,
            Some("portal-particles-enabled") => Self::PortalParticlesEnabled,
            Some("portal-particles-disabled") => Self::PortalParticlesDisabled,
            Some("portal-density") => Self::PortalDensity,
            Some(other) => {
                panic!(
                    "unknown fixture startup screen: {other}; expected inworld, overlay, swimming, menu, logout, sound, sound-click, merchant-click, footsteps, reset-windows, portal-particles-enabled, portal-particles-disabled or portal-density"
                )
            }
        };
        assert!(
            args.next().is_none(),
            "expected at most one fixture argument"
        );
        screen
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::CharSelect | Self::Menu => "charselect",
            Self::InWorld
            | Self::Overlay
            | Self::Swimming
            | Self::Logout
            | Self::Sound
            | Self::SoundClick
            | Self::MerchantClick
            | Self::Footsteps
            | Self::ResetWindows
            | Self::PortalParticlesEnabled
            | Self::PortalParticlesDisabled
            | Self::PortalDensity => "inworld",
        }
    }
}

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<(Entity, LoginRequest)>,
    connection: Option<Entity>,
    selections: Vec<SelectCharacter>,
    inputs: Vec<PlayerInput>,
    interactions: Vec<InteractNpc>,
    closes: Vec<CloseInteraction>,
    casts: Vec<SpellCastIntent>,
    vendor: Option<Entity>,
    /// A moving or jumping input arrived whose release no stop input has reported yet.
    unreported_release: bool,
    stops: u32,
}

fn receive_requests(
    mut logins: Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
    mut selections: Query<&mut MessageReceiver<SelectCharacter>>,
    mut inputs: Query<&mut MessageReceiver<PlayerInput>>,
    mut interactions: Query<&mut MessageReceiver<InteractNpc>>,
    mut closes: Query<&mut MessageReceiver<CloseInteraction>>,
    mut casts: Query<&mut MessageReceiver<SpellCastIntent>>,
    mut incoming: ResMut<Incoming>,
) {
    for (entity, mut receiver) in &mut logins {
        incoming
            .logins
            .extend(receiver.receive().map(|request| (entity, request)));
    }
    for mut receiver in &mut selections {
        incoming.selections.extend(receiver.receive());
    }
    for mut receiver in &mut inputs {
        incoming.inputs.extend(receiver.receive());
    }
    for mut receiver in &mut interactions {
        incoming.interactions.extend(receiver.receive());
    }
    for mut receiver in &mut closes {
        incoming.closes.extend(receiver.receive());
    }
    for mut receiver in &mut casts {
        incoming.casts.extend(receiver.receive());
    }
}

fn send<M: network::Message, C: network::Channel>(app: &mut App, message: M) {
    let world = app.world_mut();
    let connection = world
        .resource::<Incoming>()
        .connection
        .expect("fixture response requires an authenticated requester");
    let mut sender = world
        .get_mut::<MessageSender<M>>(connection)
        .expect("requesting fixture connection has no message sender");
    sender.send::<C>(message);
}

fn start_server() -> (App, SocketAddr) {
    // ServerUdpIo does not expose the assigned port when bound to port zero.
    let reservation = UdpSocket::bind("127.0.0.1:0").expect("reserve local fixture port");
    let address = reservation.local_addr().expect("read fixture address");
    drop(reservation);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.init_resource::<Incoming>();
    app.add_systems(Update, receive_requests);
    app.add_observer(|link: On<Add, LinkOf>, mut commands: Commands| {
        commands.entity(link.entity).insert(ReplicationSender);
    });
    app.finish();
    app.cleanup();
    let entity = app
        .world_mut()
        .spawn((
            network::LocalAddr(address),
            server::ServerUdpIo::default(),
            server::NetcodeServer::new(server::NetcodeConfig::default()),
        ))
        .id();
    app.world_mut().trigger(server::Start { entity });
    (app, address)
}

struct FixtureConfig {
    home: PathBuf,
}

impl FixtureConfig {
    fn create(root: &Path, screen: StartupScreen) -> Self {
        let diagnostics = if screen == StartupScreen::Logout {
            std::env::temp_dir().join("game-engine-logout-config")
        } else {
            root.join("data/diagnostics")
        };
        fs::create_dir_all(&diagnostics).expect("create fixture diagnostics directory");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("fixture clock after epoch")
            .as_nanos();
        let home = diagnostics.join(format!("native-input-{}-{timestamp}", std::process::id()));
        fs::create_dir(&home).expect("create unique fixture config home");
        let config = Self { home };
        let credentials = config.home.join("world-of-osso/credentials.ron");
        fs::create_dir(credentials.parent().expect("credentials parent"))
            .expect("create fixture credentials directory");
        fs::write(&credentials, "(username:\"fixture\",password:\"fixture\")")
            .expect("write fixture-only credentials");
        config.persist_menu_defaults(screen);
        if matches!(
            screen,
            StartupScreen::PortalParticlesEnabled
                | StartupScreen::PortalParticlesDisabled
                | StartupScreen::PortalDensity
        ) {
            fs::write(
                config.home.join("world-of-osso/options_settings.ron"),
                if screen == StartupScreen::PortalDensity {
                    "(graphics:(particleEffectsEnabled:true,particleDensity:100))"
                } else if screen == StartupScreen::PortalParticlesEnabled {
                    "(graphics:(particleEffectsEnabled:true))"
                } else {
                    "(graphics:(particleEffectsEnabled:false))"
                },
            )
            .expect("write isolated persisted portal particle setting");
        }
        if screen == StartupScreen::ResetWindows {
            fs::write(
                config.home.join("world-of-osso/options_settings.ron"),
                "(graphics:(uiScale:1.25),modal_offset:Some((80.0,-32.0)))",
            )
            .expect("seed nondefault Options modal offset");
            fs::write(config.home.join("world-of-osso/ui_layout.ron"), "(window_positions:{\"17\":{\"CharacterFrame\":(25.0,30.0)},\"18\":{\"CharacterFrame\":(75.0,80.0),\"SpellBookRoot\":(70.0,90.0)}},edit_mode:(layouts:{\"Layout 1\":(elements:{\"PlayerFrame\":(anchor:TopLeft,offset:(12.0,24.0))})},active_layout:{\"18\":\"Layout 1\"}))")
                .expect("seed two characters and edit mode layout");
        }
        if screen == StartupScreen::MerchantClick {
            fs::write(
                config.home.join("world-of-osso/ui_layout.ron"),
                "(window_positions:{\"18\":{\"CharacterFrame\":(75.0,80.0)}})",
            )
            .expect("seed other character placement");
        }
        if matches!(
            screen,
            StartupScreen::Sound
                | StartupScreen::SoundClick
                | StartupScreen::MerchantClick
                | StartupScreen::Footsteps
        ) {
            fs::write(
                config.home.join("world-of-osso/options_settings.ron"),
                if screen == StartupScreen::Footsteps {
                    "(graphics:(particleEffectsEnabled:false),sound:(master_volume:1.0,ambient_volume:0.3,effects_volume:0.8,music_volume:0.45,music_enabled:true,muted:false))"
                } else if screen == StartupScreen::MerchantClick {
                    "(graphics:(uiScale:1.25),modal_offset:Some((80.0,-32.0)),sound:(master_volume:1.0,ambient_volume:0.3,effects_volume:0.8,music_volume:0.45,music_enabled:true,muted:false))"
                } else {
                    "(sound:(master_volume:1.0,ambient_volume:0.3,effects_volume:0.8,music_volume:0.45,music_enabled:true,muted:false))"
                },
            )
            .expect("write isolated deterministic sound options");
        }
        config
    }

    fn persist_menu_defaults(&self, screen: StartupScreen) {
        if screen != StartupScreen::Menu {
            return;
        }
        fs::write(self.home.join("world-of-osso/options_settings.ron"), "()")
            .expect("seed menu defaults instead of migrating user legacy bindings");
    }
}

impl Drop for FixtureConfig {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.home) {
            eprintln!(
                "remove fixture config home {}: {error}",
                self.home.display()
            );
        }
    }
}

fn fixture_script(screen: StartupScreen) -> &'static str {
    match screen {
        StartupScreen::ResetWindows => "res://tests/options_reset_windows.gd",
        StartupScreen::MerchantClick => "res://tests/world_merchant_click_flow.gd",
        StartupScreen::PortalParticlesEnabled | StartupScreen::PortalParticlesDisabled => {
            "res://tests/world_portal_particles_flow.gd"
        }
        StartupScreen::PortalDensity => "res://tests/world_portal_density_flow.gd",
        StartupScreen::SoundClick => "res://tests/world_spell_click_flow.gd",
        StartupScreen::Sound => "res://tests/world_sound_flow.gd",
        StartupScreen::Footsteps => "res://tests/world_footsteps_flow.gd",
        StartupScreen::Logout => "res://tests/world_logout_flow.gd",
        StartupScreen::Menu => "res://tests/world_menu_flow.gd",
        _ => "res://tests/world_input_flow.gd",
    }
}

fn launch_godot(
    root: &Path,
    project: &Path,
    config: &FixtureConfig,
    address: SocketAddr,
    screen: StartupScreen,
    map_verify: bool,
) -> (Child, Receiver<String>, Vec<thread::JoinHandle<()>>) {
    let binary = if matches!(
        screen,
        StartupScreen::Menu
            | StartupScreen::Sound
            | StartupScreen::SoundClick
            | StartupScreen::MerchantClick
            | StartupScreen::Footsteps
            | StartupScreen::ResetWindows
            | StartupScreen::PortalParticlesEnabled
            | StartupScreen::PortalParticlesDisabled
            | StartupScreen::PortalDensity
    ) {
        std::env::var_os("GODOT_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").expect("HOME for pinned Godot"))
                    .join(".cache/game-engine/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64")
            })
    } else {
        root.join("target/debug/game-engine-launcher")
    };
    let display_args: &[&str] = if std::env::var("GODOT_TEST_VISUAL").as_deref() == Ok("1") {
        &["--display-driver", "wayland", "--audio-driver", "Dummy"]
    } else {
        &["--headless"]
    };
    // The swimming probe's phases are frame counts sized for 1/60 s steps (FRAMES_LIMIT,
    // STILL_FRAMES), but process deltas carry whole stalls: a cold shader cache's 452 ms
    // first W frame crossed the dry shore before any dry PlayerInput, and slow W+Space
    // frames carried the swimmer out of the deep water into the far shallows.
    let step_args: &[&str] = if screen == StartupScreen::Swimming {
        &["--fixed-fps", "60"]
    } else {
        &[]
    };
    let mut child = Command::new(binary)
        .current_dir(root)
        .args(display_args)
        .args(step_args)
        .args([
            "--path",
            project.to_str().expect("UTF-8 Godot project path"),
            "--script",
            fixture_script(screen),
        ])
        .args(
            if matches!(
                screen,
                StartupScreen::Menu
                    | StartupScreen::Sound
                    | StartupScreen::SoundClick
                    | StartupScreen::MerchantClick
                    | StartupScreen::Footsteps
                    | StartupScreen::ResetWindows
                    | StartupScreen::PortalParticlesEnabled
                    | StartupScreen::PortalParticlesDisabled
                    | StartupScreen::PortalDensity
            ) {
                &["--"][..]
            } else {
                &[][..]
            },
        )
        .args(["--screen", screen.as_str()])
        .args(
            if !matches!(screen, StartupScreen::CharSelect | StartupScreen::Menu) {
                &["--char", "iNpUt fIxTuRe", "--server"][..]
            } else {
                &["--server"][..]
            },
        )
        .arg(address.to_string())
        .env("GODOT_TEST_STARTUP_SCREEN", screen.as_str())
        .env("GODOT_TEST_MAP_VERIFY", if map_verify { "1" } else { "0" })
        .env(
            "GODOT_TEST_SWIMMING",
            if screen == StartupScreen::Swimming {
                "1"
            } else {
                "0"
            },
        )
        .env(
            "GODOT_TEST_OVERLAY_ONLY",
            if screen == StartupScreen::Overlay {
                "1"
            } else {
                "0"
            },
        )
        .env("CARGO", env!("CARGO"))
        .env("XDG_CONFIG_HOME", &config.home)
        .env(
            "XDG_DATA_HOME",
            project
                .parent()
                .expect("Godot project root")
                .join("user-data"),
        )
        .env("GODOT_TEST_SERVER", address.to_string())
        .env(
            "GODOT_TEST_PARTICLES",
            match screen {
                StartupScreen::PortalParticlesEnabled => "enabled",
                StartupScreen::PortalParticlesDisabled => "disabled",
                StartupScreen::PortalDensity => "density",
                _ => "",
            },
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start root launcher for native Godot fixture");
    let output = child.stdout.take().expect("read Godot stdout");
    let errors = child.stderr.take().expect("read Godot stderr");
    let (sender, receiver) = mpsc::channel();
    let readers = vec![
        read_output(output, sender.clone(), false),
        read_output(errors, sender, true),
    ];
    (child, receiver, readers)
}

fn read_output(
    output: impl std::io::Read + Send + 'static,
    sender: mpsc::Sender<String>,
    stderr: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let line = line.expect("read Godot fixture output");
            println!("godot: {line}");
            let line = if stderr {
                format!("GODOT_STDERR: {line}")
            } else {
                line
            };
            if sender.send(line).is_err() {
                return;
            }
        }
    })
}

fn starter_equipment() -> EquipmentAppearance {
    EquipmentAppearance {
        entries: [
            (EquipmentVisualSlot::MainHand, 25, 21),
            (EquipmentVisualSlot::Shirt, 38, 4),
            (EquipmentVisualSlot::Legs, 39, 7),
            (EquipmentVisualSlot::Feet, 40, 8),
            (EquipmentVisualSlot::OffHand, 2362, 14),
        ]
        .into_iter()
        .map(|(slot, item_id, inventory_type)| EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type,
            hidden: false,
        })
        .collect(),
    }
}

fn respond_to_login(app: &mut App, screen: StartupScreen) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
    if requests.is_empty() {
        return Ok(());
    }
    if requests.len() != 1 {
        return Err(format!("expected one LoginRequest, got {}", requests.len()));
    }
    let (connection, request) = &requests[0];
    app.world_mut().resource_mut::<Incoming>().connection = Some(*connection);
    let fixture_credentials =
        request.username == "fixture" && request.password == "fixture" && request.token.is_none();
    let fixture_token = request.username.is_empty()
        && request.password.is_empty()
        && request.token.as_deref() == Some("fixture-only-token");
    if !fixture_credentials && !fixture_token {
        return Err("unexpected fixture authentication request".into());
    }
    let equipped = CharacterListEntry {
        character_id: 17,
        name: NAME.into(),
        level: 10,
        race: 1,
        class: 2,
        appearance: Default::default(),
        equipment_appearance: starter_equipment(),
    };
    let unequipped = CharacterListEntry {
        character_id: 18,
        name: UNEQUIPPED_NAME.into(),
        equipment_appearance: EquipmentAppearance::default(),
        ..equipped.clone()
    };
    let collection = CharacterListEntry {
        character_id: 19,
        name: COLLECTION_NAME.into(),
        equipment_appearance: EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Chest,
                item_id: Some(1),
                display_info_id: Some(175942),
                inventory_type: 5,
                hidden: false,
            }],
        },
        ..equipped.clone()
    };
    let characters = if !matches!(screen, StartupScreen::CharSelect | StartupScreen::Menu) {
        vec![unequipped, equipped, collection]
    } else {
        vec![equipped, unequipped, collection]
    };
    send::<_, AuthChannel>(
        app,
        LoginResponse {
            success: true,
            token: "fixture-only-token".into(),
            characters,
            error: None,
        },
    );
    Ok(())
}

fn respond_to_selection(
    app: &mut App,
    screen: StartupScreen,
    selected: &mut Option<Entity>,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
    for request in requests {
        if selected.is_some() || request.character_id != 17 {
            return Err(format!(
                "unexpected SelectCharacter: {}",
                request.character_id
            ));
        }
        let spawn = if screen == StartupScreen::Swimming {
            SWIM_START
        } else if matches!(
            screen,
            StartupScreen::PortalParticlesEnabled
                | StartupScreen::PortalParticlesDisabled
                | StartupScreen::PortalDensity
        ) {
            // Azeroth 30_48, just outside MODD 1112; the native camera faces -Z.
            [-8766.11, 88.5, -845.5]
        } else {
            FIRST
        };
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: NAME.into(),
                    race: 1,
                    class: 2,
                    appearance: Default::default(),
                },
                starter_equipment(),
                Position {
                    x: spawn[0],
                    y: spawn[1],
                    z: spawn[2],
                },
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        if screen == StartupScreen::MerchantClick {
            app.world_mut()
                .entity_mut(player)
                .insert((Gold(1250), UnitFactionTemplate(1)));
            let vendor = app
                .world_mut()
                .spawn((
                    Npc {
                        template_id: 1213,
                        name: "Fixture Vendor".into(),
                    },
                    NpcFlags(NpcFlags::VENDOR),
                    UnitFactionTemplate(35),
                    ModelDisplay { display_id: 26 },
                    Position {
                        x: FIRST[0] - 2.0,
                        y: FIRST[1] + 0.1,
                        z: FIRST[2] + 3.0,
                    },
                    Replicate::to_clients(NetworkTarget::All),
                ))
                .id();
            app.world_mut().resource_mut::<Incoming>().vendor = Some(vendor);
        }
        if screen == StartupScreen::SoundClick {
            app.world_mut()
                .entity_mut(player)
                .insert(shared::components::UnitLevel(10));
        }
        *selected = Some(player);
        let mut remote_player = Player {
            name: REMOTE_NAME.into(),
            race: 1,
            class: 2,
            appearance: Default::default(),
        };
        remote_player.appearance.sex = 1;
        *remote = Some(
            app.world_mut()
                .spawn((
                    remote_player,
                    starter_equipment(),
                    Position {
                        x: REMOTE[0],
                        y: REMOTE[1],
                        z: REMOTE[2],
                    },
                    Replicate::to_clients(NetworkTarget::All),
                ))
                .id(),
        );
        send::<_, AuthChannel>(
            app,
            EnterWorldResponse {
                success: true,
                player_entity: Some(player.to_bits()),
                error: None,
            },
        );
        if screen == StartupScreen::SoundClick {
            send::<_, TalentChannel>(
                app,
                KnownSpellsSnapshot {
                    spells: vec![1464, 88163],
                },
            );
            send::<_, TalentChannel>(
                app,
                ActionBarSnapshot {
                    slots: vec![(0, ActionRef::Spell(1464)), (11, ActionRef::Spell(1464))],
                },
            );
        }
        // Intentionally withhold LoadTerrain until Loading input has been observed.
    }
    Ok(())
}

/// Moving and jumping inputs since the last call. Each release must report exactly one
/// stop input (no direction, no jump); stops are checked here and not returned.
fn take_inputs(app: &mut App) -> Vec<PlayerInput> {
    let mut incoming = app.world_mut().resource_mut::<Incoming>();
    let inputs = std::mem::take(&mut incoming.inputs);
    let mut moving = Vec::with_capacity(inputs.len());
    for input in inputs {
        if input.direction != [0.0; 3] || input.jumping {
            incoming.unreported_release = true;
            moving.push(input);
            continue;
        }
        assert!(
            std::mem::take(&mut incoming.unreported_release),
            "stop PlayerInput without preceding movement: {input:?}"
        );
        assert!(
            input.position.iter().all(|axis| axis.is_finite()),
            "stop PlayerInput lost its reported position: {input:?}"
        );
        incoming.stops += 1;
    }
    moving
}

/// Fails when a drained release has not reported its stop input.
fn ensure_release_reported(app: &App, stage: impl std::fmt::Debug) -> Result<(), String> {
    if app.world().resource::<Incoming>().unreported_release {
        return Err(format!("{stage:?}: the release sent no stop PlayerInput"));
    }
    Ok(())
}

fn assert_forward_input(inputs: Vec<PlayerInput>) -> Result<bool, String> {
    for input in &inputs {
        if !input.position.iter().all(|axis| axis.is_finite()) {
            return Err(format!("W input lost its reported position: {input:?}"));
        }
        let [x, y, z] = input.direction;
        if x.abs() > 0.15 || y.abs() > 0.01 || z > -0.9 || z < -1.1 {
            return Err(format!(
                "W produced unexpected facing-PI direction: {input:?}"
            ));
        }
        if (input.facing_yaw.abs() - std::f32::consts::PI).abs() > 0.15 {
            return Err(format!("W changed original facing yaw: {input:?}"));
        }
        if !input.running || input.jumping {
            return Err(format!("W changed run or jump state: {input:?}"));
        }
    }
    Ok(!inputs.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Directional {
    Walk,
    Backward,
    Left,
    Right,
}

impl Directional {
    fn expected_vector(self, yaw: f32) -> [f32; 3] {
        let forward = [yaw.sin(), yaw.cos()];
        match self {
            Self::Walk => [forward[0], 0.0, forward[1]],
            Self::Backward => [-forward[0], 0.0, -forward[1]],
            Self::Left => [forward[1], 0.0, -forward[0]],
            Self::Right => [-forward[1], 0.0, forward[0]],
        }
    }
}

fn assert_idle_jump_input(inputs: Vec<PlayerInput>) -> Result<bool, String> {
    for input in &inputs {
        if !input.position.iter().all(|axis| axis.is_finite())
            || !input.facing_yaw.is_finite()
            || input
                .direction
                .iter()
                .any(|component| !component.is_finite())
            || input
                .direction
                .iter()
                .any(|component| component.abs() > 0.01)
            || !input.jumping
        {
            return Err(format!(
                "idle Space produced unexpected PlayerInput: {input:?}"
            ));
        }
    }
    Ok(!inputs.is_empty())
}

fn assert_running_input(inputs: Vec<PlayerInput>, jumping: bool) -> Result<bool, String> {
    for input in &inputs {
        let expected = Directional::Walk.expected_vector(input.facing_yaw);
        if !input.position.iter().all(|axis| axis.is_finite())
            || !input.facing_yaw.is_finite()
            || input
                .direction
                .iter()
                .zip(expected)
                .any(|(actual, expected)| !actual.is_finite() || (actual - expected).abs() > 0.15)
            || !input.running
            || input.jumping != jumping
        {
            return Err(format!(
                "running W+Space produced unexpected PlayerInput: {input:?}"
            ));
        }
    }
    Ok(!inputs.is_empty())
}

fn assert_directional_input(
    inputs: Vec<PlayerInput>,
    direction: Directional,
) -> Result<bool, String> {
    for input in &inputs {
        let expected = direction.expected_vector(input.facing_yaw);
        if !input.position.iter().all(|axis| axis.is_finite())
            || !input.facing_yaw.is_finite()
            || input
                .direction
                .iter()
                .any(|component| !component.is_finite())
            || input
                .direction
                .iter()
                .zip(expected)
                .any(|(actual, expected)| (actual - expected).abs() > 0.15)
            || input.running != (direction != Directional::Walk)
            || input.jumping
        {
            return Err(format!(
                "{direction:?} produced unexpected decoded PlayerInput: {input:?}"
            ));
        }
    }
    Ok(!inputs.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    AwaitLoading,
    AwaitWorld,
    OverlayDone,
    AwaitRemoved,
    AwaitRestored,
    AwaitRemoteRemoved,
    AwaitRemoteRestored,
    Held,
    Released,
    Moving(Directional),
    Settling(Directional),
    FinalStand,
    Jumping,
    JumpReleased,
    JumpLanded,
    JumpStand,
    RunningBeforeJump,
    RunningJump,
    RunningJumpReleased,
    RunningJumpLanded,
    RunningResumed,
    RunningStopped,
    RunningStand,
    Stopped,
}

fn accept_phase_line(
    app: &mut App,
    player: Option<Entity>,
    remote: Option<Entity>,
    phase: &mut Phase,
    line: &str,
    overlay_only: bool,
) -> Result<(), String> {
    match (&*phase, line) {
        (_, line) if line.starts_with("GODOT_STDERR: ERROR:") => {
            return Err(format!("Godot runtime error: {line}"));
        }
        (Phase::AwaitLoading, "FIXTURE LOADING_OBSERVED") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived during withheld-terrain Loading interval".into());
            }
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            *phase = Phase::AwaitWorld;
        }
        (Phase::AwaitWorld, "FIXTURE OVERLAY_DONE") if overlay_only => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived during overlay-only world loading".into());
            }
            *phase = Phase::OverlayDone;
        }
        (Phase::AwaitWorld, "FIXTURE WORLD_READY") if !overlay_only => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived before native InWorld readiness".into());
            }
            app.world_mut()
                .entity_mut(player.expect("selected player exists at world readiness"))
                .insert(EquipmentAppearance::default());
            *phase = Phase::AwaitRemoved;
        }
        (Phase::AwaitRemoved, "FIXTURE EQUIPMENT_REMOVED") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived during equipment removal".into());
            }
            app.world_mut()
                .entity_mut(player.expect("selected player exists at equipment removal"))
                .insert(starter_equipment());
            *phase = Phase::AwaitRestored;
        }
        (Phase::AwaitRestored, "FIXTURE EQUIPMENT_RESTORED") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived before restored equipment".into());
            }
            app.world_mut()
                .entity_mut(remote.expect("remote player exists at equipment restoration"))
                .insert(EquipmentAppearance::default());
            *phase = Phase::AwaitRemoteRemoved;
        }
        (Phase::AwaitRemoteRemoved, "FIXTURE REMOTE_EQUIPMENT_REMOVED") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived during remote equipment removal".into());
            }
            app.world_mut()
                .entity_mut(remote.expect("remote player exists at equipment removal"))
                .insert(starter_equipment());
            *phase = Phase::AwaitRemoteRestored;
        }
        (Phase::AwaitRemoteRestored, "FIXTURE REMOTE_EQUIPMENT_RESTORED") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived before remote equipment restoration".into());
            }
            *phase = Phase::Held;
        }
        (Phase::Held, "FIXTURE RELEASED") => *phase = Phase::Released,
        (Phase::Released, "FIXTURE WALK_START") => *phase = Phase::Moving(Directional::Walk),
        (Phase::Moving(Directional::Walk), "FIXTURE WALK_END") => {
            *phase = Phase::Settling(Directional::Walk)
        }
        (Phase::Settling(Directional::Walk), "FIXTURE BACKWARD_START") => {
            *phase = Phase::Moving(Directional::Backward)
        }
        (Phase::Moving(Directional::Backward), "FIXTURE BACKWARD_END") => {
            *phase = Phase::Settling(Directional::Backward)
        }
        (Phase::Settling(Directional::Backward), "FIXTURE LEFT_START") => {
            *phase = Phase::Moving(Directional::Left)
        }
        (Phase::Moving(Directional::Left), "FIXTURE LEFT_END") => {
            *phase = Phase::Settling(Directional::Left)
        }
        (Phase::Settling(Directional::Left), "FIXTURE RIGHT_START") => {
            *phase = Phase::Moving(Directional::Right)
        }
        (Phase::Moving(Directional::Right), "FIXTURE RIGHT_END") => {
            *phase = Phase::Settling(Directional::Right)
        }
        (Phase::Settling(Directional::Right), "FIXTURE FINAL_STAND") => *phase = Phase::FinalStand,
        (Phase::FinalStand, "FIXTURE JUMP_START") => *phase = Phase::Jumping,
        (Phase::Jumping, "FIXTURE JUMP_RELEASED") => *phase = Phase::JumpReleased,
        (Phase::JumpReleased, "FIXTURE JUMP_LANDED") => *phase = Phase::JumpLanded,
        (Phase::JumpLanded, "FIXTURE JUMP_STAND") => *phase = Phase::JumpStand,
        (Phase::JumpStand, "FIXTURE RUN_JUMP_RUN_START") => *phase = Phase::RunningBeforeJump,
        (Phase::RunningBeforeJump, "FIXTURE RUN_JUMP_START") => *phase = Phase::RunningJump,
        (Phase::RunningJump, "FIXTURE RUN_JUMP_RELEASED") => *phase = Phase::RunningJumpReleased,
        (Phase::RunningJumpReleased, "FIXTURE RUN_JUMP_LANDED") => {
            *phase = Phase::RunningJumpLanded
        }
        (Phase::RunningJumpLanded, "FIXTURE RUN_JUMP_RESUMED") => *phase = Phase::RunningResumed,
        (Phase::RunningResumed, "FIXTURE RUN_JUMP_W_RELEASED") => *phase = Phase::RunningStopped,
        (Phase::RunningStopped, "FIXTURE RUN_JUMP_STAND") => *phase = Phase::RunningStand,
        (Phase::RunningStand, "FIXTURE STOPPED") => *phase = Phase::Stopped,
        (_, line) if line.starts_with("FIXTURE ") => {
            return Err(format!(
                "out-of-order Godot fixture phase {phase:?}: {line}"
            ));
        }
        _ => {}
    }
    Ok(())
}

fn run_fixture(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    reader: Vec<thread::JoinHandle<()>>,
    screen: StartupScreen,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut phase = Phase::AwaitLoading;
    let mut saw_forward = false;
    let mut released_at = None;
    let mut transition_at: Option<Instant> = None;
    let mut saw_direction = false;
    let mut saw_idle_jump = false;
    let mut jump_landed_at: Option<Instant> = None;
    let mut jump_stand_at: Option<Instant> = None;
    let mut running_landed_at: Option<Instant> = None;
    let mut running_release_at: Option<Instant> = None;
    let mut saw_running_jump = false;
    let mut saw_resumed_run = false;
    let deadline = Instant::now()
        + if screen == StartupScreen::Overlay {
            OVERLAY_TIMEOUT
        } else {
            TIMEOUT
        };
    let mut reader = Some(reader);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, screen)?;
        respond_to_selection(app, screen, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            if let Some(readers) = reader.take() {
                for reader in readers {
                    reader.join().map_err(|_| "Godot output reader panicked")?;
                }
            }
        }
        for line in lines.try_iter() {
            let previous = &phase;
            if *previous == Phase::AwaitLoading
                && line.trim() == "FIXTURE LOADING_OBSERVED"
                && selected.is_none()
            {
                return Err("Loading was observed before character selection".into());
            }
            let previous_phase = phase;
            let next_start = line.trim().ends_with("_START") && line.starts_with("FIXTURE ");
            if next_start
                && matches!(previous_phase, Phase::Settling(_))
                && transition_at.expect("direction release recorded").elapsed()
                    < RELEASE_DRAIN + RELEASE_QUIET
            {
                return Err(format!(
                    "next direction began before drained quiet interval: {line}"
                ));
            }
            if line.trim().ends_with("_END")
                && matches!(previous_phase, Phase::Moving(_))
                && !saw_direction
            {
                return Err(format!(
                    "no decoded stable {previous_phase:?} PlayerInput on InputChannel"
                ));
            }
            if line.trim() == "FIXTURE FINAL_STAND"
                && transition_at.expect("right release recorded").elapsed()
                    < RELEASE_DRAIN + RELEASE_QUIET
            {
                return Err("final Stand occurred before drained quiet interval".into());
            }
            if line.trim() == "FIXTURE JUMP_RELEASED" && !saw_idle_jump {
                return Err("no decoded stationary jumping PlayerInput on InputChannel".into());
            }
            if line.trim() == "FIXTURE RUN_JUMP_RUN_START"
                && jump_stand_at.expect("idle Stand recorded").elapsed()
                    < RELEASE_DRAIN + RELEASE_QUIET
            {
                return Err("running jump began before idle jump quiet interval".into());
            }
            if line.trim() == "FIXTURE RUN_JUMP_RELEASED" && !saw_running_jump {
                return Err(
                    "no decoded forward-running jumping PlayerInput on InputChannel".into(),
                );
            }
            if line.trim() == "FIXTURE RUN_JUMP_W_RELEASED" && !saw_resumed_run {
                return Err("no decoded resumed forward-running nonjump PlayerInput".into());
            }
            accept_phase_line(
                app,
                selected,
                remote,
                &mut phase,
                line.trim(),
                screen == StartupScreen::Overlay,
            )?;
            if phase == Phase::JumpLanded {
                jump_landed_at.get_or_insert_with(Instant::now);
            }
            if phase == Phase::JumpStand {
                jump_stand_at.get_or_insert_with(Instant::now);
            }
            if phase == Phase::RunningJumpLanded {
                running_landed_at.get_or_insert_with(Instant::now);
            }
            if phase == Phase::RunningStopped {
                running_release_at.get_or_insert_with(Instant::now);
            }
            if phase == Phase::Released && released_at.is_none() {
                released_at = Some(Instant::now());
            }
            if matches!(phase, Phase::Moving(_) | Phase::Settling(_)) && phase != previous_phase {
                transition_at = Some(Instant::now());
                saw_direction = false;
            }
        }
        match phase {
            Phase::AwaitLoading
            | Phase::AwaitWorld
            | Phase::OverlayDone
            | Phase::AwaitRemoved
            | Phase::AwaitRestored
            | Phase::AwaitRemoteRemoved
            | Phase::AwaitRemoteRestored => {
                if !take_inputs(app).is_empty() {
                    return Err(format!("PlayerInput arrived during {phase:?}"));
                }
            }
            Phase::Held => saw_forward |= assert_forward_input(take_inputs(app))?,
            Phase::Moving(direction) => {
                let inputs = take_inputs(app);
                // Prior held direction may still be in flight after the key transition.
                if transition_at.expect("direction start recorded").elapsed() >= RELEASE_DRAIN {
                    saw_direction |= assert_directional_input(inputs, direction)?;
                }
            }
            Phase::Jumping | Phase::JumpReleased => {
                saw_idle_jump |= assert_idle_jump_input(take_inputs(app))?;
            }
            Phase::RunningBeforeJump => {
                saw_forward |= assert_running_input(take_inputs(app), false)?;
            }
            Phase::RunningJump | Phase::RunningJumpReleased => {
                let (jumping, forward): (Vec<_>, Vec<_>) = take_inputs(app)
                    .into_iter()
                    .partition(|input| input.jumping);
                saw_running_jump |= assert_running_input(jumping, true)?;
                assert_running_input(forward, false)?;
            }
            Phase::RunningJumpLanded => {
                let (jumping, forward): (Vec<_>, Vec<_>) = take_inputs(app)
                    .into_iter()
                    .partition(|input| input.jumping);
                if running_landed_at
                    .expect("running landing recorded")
                    .elapsed()
                    >= RELEASE_DRAIN
                    && !jumping.is_empty()
                {
                    return Err(format!(
                        "running jump packets continued after landing: {jumping:?}"
                    ));
                }
                assert_running_input(jumping, true)?;
                saw_resumed_run |= assert_running_input(forward, false)?;
            }
            Phase::RunningResumed => {
                saw_resumed_run |= assert_running_input(take_inputs(app), false)?;
            }
            Phase::RunningStopped | Phase::RunningStand => {
                let inputs = take_inputs(app);
                if running_release_at
                    .expect("running W release recorded")
                    .elapsed()
                    >= RELEASE_DRAIN
                {
                    if !inputs.is_empty() {
                        return Err(format!(
                            "running jump packets continued after W release: {inputs:?}"
                        ));
                    }
                    ensure_release_reported(app, phase)?;
                }
            }
            Phase::JumpLanded | Phase::JumpStand => {
                let inputs = take_inputs(app);
                if jump_landed_at.expect("idle landing recorded").elapsed() >= RELEASE_DRAIN {
                    if !inputs.is_empty() {
                        return Err(format!(
                            "idle jump packets continued after landing: {inputs:?}"
                        ));
                    }
                    ensure_release_reported(app, phase)?;
                }
            }
            Phase::Settling(_) | Phase::FinalStand => {
                let inputs = take_inputs(app);
                if transition_at.expect("direction release recorded").elapsed() >= RELEASE_DRAIN {
                    if !inputs.is_empty() {
                        return Err(format!(
                            "direction packets continued after release: {inputs:?}"
                        ));
                    }
                    ensure_release_reported(app, phase)?;
                }
            }
            Phase::Released | Phase::Stopped => {
                let stopped_at = if phase == Phase::Stopped {
                    running_release_at.expect("running W release recorded")
                } else {
                    released_at.expect("W release marker recorded")
                };
                let inputs = take_inputs(app);
                if stopped_at.elapsed() >= RELEASE_DRAIN {
                    if !inputs.is_empty() {
                        return Err(format!(
                            "movement packets continued after release: {inputs:?}"
                        ));
                    }
                    ensure_release_reported(app, phase)?;
                }
            }
        }
        if let Some(status) = status {
            if screen == StartupScreen::Overlay && status.success() && phase == Phase::OverlayDone {
                println!(
                    "PASS: authored WMO shader 7 overlay-only CPU/GPU diagnostic; no PlayerInput"
                );
                return Ok(());
            }
            if !status.success() || phase != Phase::Stopped {
                return Err(format!("Godot exited {status} at phase {phase:?}"));
            }
            if !saw_forward {
                return Err("no decoded held-W PlayerInput on InputChannel".into());
            }
            if !saw_idle_jump {
                return Err("no decoded stationary jumping PlayerInput on InputChannel".into());
            }
            if !saw_running_jump || !saw_resumed_run {
                return Err(
                    "running jump lacked decoded jumping or resumed nonjump packets".into(),
                );
            }
            let stopped_for = running_release_at
                .expect("running W release recorded")
                .elapsed();
            if stopped_for >= RELEASE_DRAIN + RELEASE_QUIET {
                ensure_release_reported(app, phase)?;
                let stops = app.world().resource::<Incoming>().stops;
                println!("stop inputs: {stops}, one per release");
                println!(
                    "PASS: Loading blocked input; W and Walk/Backward/Left/Right decoded UDP; idle Space sent stationary jumping packets 37/38/39/0; running W+Space sent forward jumping then nonjump packets 37/38/187/5/0 and returned quiet"
                );
                return Ok(());
            }
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for Godot input phase {phase:?}; saw_forward={saw_forward}"
    ))
}

fn stage_isolated_project(
    root: &Path,
    screen: StartupScreen,
) -> Option<reset_windows::FixtureProject> {
    let project = matches!(
        screen,
        StartupScreen::ResetWindows
            | StartupScreen::SoundClick
            | StartupScreen::MerchantClick
            | StartupScreen::PortalDensity
    )
    .then(|| {
        reset_windows::FixtureProject::create(
            root,
            matches!(
                screen,
                StartupScreen::SoundClick
                    | StartupScreen::MerchantClick
                    | StartupScreen::PortalDensity
            ),
        )
        .expect("stage isolated reset data and Godot project")
    });
    if screen == StartupScreen::PortalDensity {
        project
            .as_ref()
            .expect("density fixture project")
            .stage_density_sensitive_portal(root)
            .expect("stage controlled density-sensitive portal without changing original");
    }
    project
}

fn main() {
    let screen = StartupScreen::from_example_args();
    let (mut app, address) = start_server();
    println!("FIXTURE ENDPOINT {address}");
    let checkout = fixture_support::checkout_root_from_executable("native_input_fixture")
        .expect("locate originating fixture checkout");
    let root = checkout.as_path();
    let launcher = root.join("target/debug/game-engine-launcher");
    if !matches!(
        screen,
        StartupScreen::Sound
            | StartupScreen::SoundClick
            | StartupScreen::MerchantClick
            | StartupScreen::Footsteps
            | StartupScreen::ResetWindows
            | StartupScreen::PortalParticlesEnabled
            | StartupScreen::PortalParticlesDisabled
            | StartupScreen::PortalDensity
    ) {
        assert!(
            launcher.is_file(),
            "build root launcher first: missing {}",
            launcher.display()
        );
    }
    let reset_project = stage_isolated_project(root, screen);
    let project = reset_project
        .as_ref()
        .map_or_else(|| root.join("godot"), |fixture| fixture.project.clone());
    let config_root = reset_project
        .as_ref()
        .map_or(root, |fixture| fixture.root_path());
    let config = FixtureConfig::create(config_root, screen);
    let (mut child, lines, reader) = launch_godot(root, &project, &config, address, screen, false);
    let result = match screen {
        StartupScreen::Swimming => swimming::run(&mut app, &mut child, lines, reader),
        StartupScreen::Menu => menu::run(&mut app, &mut child, lines, reader),
        StartupScreen::ResetWindows => reset_windows::run(
            &mut app, &mut child, lines, reader, root, &config, &project, address,
        ),
        StartupScreen::Logout => logout::run(&mut app, &mut child, lines, reader, root, address),
        StartupScreen::SoundClick => sound_click::run(&mut app, &mut child, lines, reader),
        StartupScreen::MerchantClick => merchant_click::run(&mut app, &mut child, lines, reader),
        StartupScreen::Sound => sound::run(&mut app, &mut child, lines, reader),
        StartupScreen::Footsteps => footsteps::run(&mut app, &mut child, lines, reader),
        StartupScreen::PortalDensity => portal_density::run(&mut app, &mut child, lines, reader),
        StartupScreen::PortalParticlesEnabled | StartupScreen::PortalParticlesDisabled => {
            portal_particles::run(&mut app, &mut child, lines, reader, screen)
        }
        StartupScreen::CharSelect | StartupScreen::InWorld | StartupScreen::Overlay => {
            run_fixture(&mut app, &mut child, lines, reader, screen)
        }
    };
    if result.is_err()
        && child
            .try_wait()
            .expect("inspect fixture child status")
            .is_none()
    {
        child.kill().expect("terminate failed Godot fixture");
        child.wait().expect("reap failed Godot fixture");
    }
    if let Err(error) = result {
        panic!("native input fixture: {error}");
    }
}
