//! Owned loopback UDP server and real Godot process for ordinary reconnect.
//! Build it with the GDExtension, then run it from the originating checkout:
//! python3 scripts/depot-build.py --root "$PWD" --fixture native_reconnect_fixture
//! GODOT_BIN=<godot> target/debug/examples/native_reconnect_fixture

#[path = "fixture_support/mod.rs"]
mod fixture_support;
use fixture_support::FixtureChild;

use std::{
    io::{BufRead, BufReader},
    net::{SocketAddr, UdpSocket},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{
    self as network, LinkOf, MessageReceiver, MessageSender, NetworkTarget, Replicate,
    ReplicationSender, server,
};
use shared::{
    components::{Player, Position},
    protocol::{
        AuthChannel, CharacterListEntry, EnterWorldResponse, LoadTerrain, LoginRequest,
        LoginResponse, SelectCharacter, TerrainChannel,
    },
};

const NAME: &str = "Reconnect Fixture";
const OTHER: &str = "Other Fixture";
// Authored terrain heights; Y=83 was below both surfaces and outside floor step reach.
const FIRST: [f32; 3] = [-8949.0, 112.879_91, 0.0];
const SECOND: [f32; 3] = [-8940.0, 117.382_83, 0.0];
const TICK: Duration = Duration::from_millis(5);
// Past the NetworkBridge's 10-second Netcode client timeout.
const TRANSPORT_SILENCE: Duration = Duration::from_secs(15);
const FIXTURE_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<(Entity, LoginRequest)>,
    selections: Vec<(Entity, SelectCharacter)>,
}

fn receive_requests(
    mut logins: Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
    mut selections: Query<(Entity, &mut MessageReceiver<SelectCharacter>)>,
    mut incoming: ResMut<Incoming>,
) {
    for (entity, mut receiver) in &mut logins {
        incoming
            .logins
            .extend(receiver.receive().map(|request| (entity, request)));
    }
    for (entity, mut receiver) in &mut selections {
        incoming
            .selections
            .extend(receiver.receive().map(|request| (entity, request)));
    }
}

fn send<M: network::Message, C: network::Channel>(app: &mut App, link: Entity, message: M) {
    let mut sender = app
        .world_mut()
        .get_mut::<MessageSender<M>>(link)
        .expect("fixture link has registered sender");
    sender.send::<C>(message);
}

fn character(id: u64, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id: id,
        name: name.into(),
        level: 10,
        race: 1,
        class: 2,
        appearance: Default::default(),
        equipment_appearance: Default::default(),
    }
}

fn start_server() -> (App, SocketAddr) {
    // ServerUdpIo does not expose the bound port when given port zero.
    let reservation = UdpSocket::bind("127.0.0.1:0").expect("reserve local fixture port");
    let address = reservation
        .local_addr()
        .expect("read local fixture address");
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

fn launch_godot(address: SocketAddr) -> (FixtureChild, Receiver<String>, thread::JoinHandle<()>) {
    let binary = std::env::var("GODOT_BIN").expect("GODOT_BIN must name the fixture executable");
    let project = fixture_support::checkout_root_from_executable("native_reconnect_fixture")
        .unwrap_or_else(|error| panic!("{error}"))
        .join("godot");
    let display_args: &[&str] = if std::env::var("GODOT_TEST_VISUAL").as_deref() == Ok("1") {
        &[]
    } else {
        &["--headless"]
    };
    let mut child = FixtureChild::spawn(
        Command::new(binary)
            .args(display_args)
            .args([
                "--path",
                project.to_str().expect("UTF-8 Godot project path"),
                "--script",
                "res://tests/world_reconnect_flow.gd",
            ])
            .env("GODOT_TEST_SERVER", address.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()),
    )
    .expect("start native Godot fixture process");
    let output = child.stdout.take().expect("read Godot stdout");
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let line = line.expect("read Godot fixture output");
            println!("godot: {line}");
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    (child, receiver, reader)
}

struct Progress {
    phase: u8,
    first_link: Option<Entity>,
    second_link: Option<Entity>,
    first_player: Option<Entity>,
    second_player: Option<Entity>,
    silence_until: Option<Instant>,
}

impl Progress {
    fn new() -> Self {
        Self {
            phase: 0,
            first_link: None,
            second_link: None,
            first_player: None,
            second_player: None,
            silence_until: None,
        }
    }

    fn receive_login(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
        for (link, request) in requests {
            let roster = match self.first_link {
                None if request.username == "fixture"
                    && request.password == "fixture"
                    && request.token.is_none() =>
                {
                    self.first_link = Some(link);
                    vec![character(17, NAME), character(18, OTHER)]
                }
                Some(first)
                    if self.second_link.is_none()
                        && link != first
                        && request.username.is_empty()
                        && request.password.is_empty()
                        && request.token.as_deref() == Some("fixture-only-token") =>
                {
                    self.second_link = Some(link);
                    vec![character(18, OTHER), character(17, NAME)]
                }
                _ => return Err(format!("unexpected LoginRequest on {link:?}: {request:?}")),
            };
            send::<_, AuthChannel>(
                app,
                link,
                LoginResponse {
                    success: true,
                    token: "fixture-only-token".into(),
                    characters: roster,
                    error: None,
                },
            );
        }
        Ok(())
    }

    fn receive_selection(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
        for (link, request) in requests {
            if request.character_id != 17 {
                return Err(format!(
                    "selected wrong character {} on {link:?}",
                    request.character_id
                ));
            }
            let position = if Some(link) == self.first_link && self.first_player.is_none() {
                FIRST
            } else if Some(link) == self.second_link && self.phase >= 2 {
                SECOND
            } else {
                return Err(format!(
                    "unexpected SelectCharacter on {link:?} at phase {}",
                    self.phase
                ));
            };
            let player = if position == FIRST {
                let player = self.spawn_player(app, position, true);
                self.first_player = Some(player);
                player
            } else {
                // The server declares the player identity now, but delays replication.
                let player = self.spawn_player(app, position, false);
                self.second_player = Some(player);
                player
            };
            send::<_, AuthChannel>(
                app,
                link,
                EnterWorldResponse {
                    success: true,
                    player_entity: Some(player.to_bits()),
                    error: None,
                },
            );
            send::<_, TerrainChannel>(
                app,
                link,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            if position == SECOND {
                self.phase = 3;
            }
        }
        Ok(())
    }

    fn spawn_player(&self, app: &mut App, position: [f32; 3], replicate: bool) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                Player {
                    name: NAME.into(),
                    race: 1,
                    class: 2,
                    appearance: Default::default(),
                },
                Position {
                    x: position[0],
                    y: position[1],
                    z: position[2],
                },
            ))
            .id();
        if replicate {
            app.world_mut()
                .entity_mut(entity)
                .insert(Replicate::to_clients(NetworkTarget::All));
        }
        entity
    }

    fn receive_line(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        match (self.phase, line) {
            (0, "FIXTURE INITIAL_READY") if self.first_player.is_some() => {
                // Pause this owned server's updates: the client receives no transport packets,
                // times out normally, then reconnects when this same server resumes.
                self.silence_until = Some(Instant::now() + TRANSPORT_SILENCE);
                self.phase = 1;
            }
            (1, "FIXTURE WORLD_RESET") => self.phase = 2,
            (3, "FIXTURE TERRAIN_REFRESHED") => {
                let player = self.second_player.ok_or("missing reconnect player")?;
                app.world_mut()
                    .entity_mut(player)
                    .insert(Replicate::to_clients(NetworkTarget::All));
                self.phase = 4;
            }
            (4, "FIXTURE RECONNECTED") => self.phase = 5,
            (_, line) if line.starts_with("FIXTURE ") => {
                return Err(format!(
                    "out-of-order Godot fixture phase {}: {line}",
                    self.phase
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

fn update_server(app: &mut App, progress: &mut Progress) -> Result<(), String> {
    let silent = progress
        .silence_until
        .is_some_and(|until| Instant::now() < until);
    if !silent {
        if progress.phase >= 2 {
            if let Some(player) = progress.first_player.take() {
                app.world_mut().despawn(player);
            }
        }
        app.update();
        progress.receive_login(app)?;
        progress.receive_selection(app)?;
    }
    Ok(())
}

fn poll_child(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
    reader: &mut Option<thread::JoinHandle<()>>,
    progress: &mut Progress,
) -> Result<bool, String> {
    let status = child.try_wait().map_err(|error| error.to_string())?;
    if status.is_some() {
        reader
            .take()
            .expect("fixture stdout reader")
            .join()
            .map_err(|_| "Godot stdout reader panicked")?;
    }
    for line in lines.try_iter() {
        progress.receive_line(app, line.trim())?;
    }
    if let Some(status) = status {
        if !status.success() || progress.phase != 5 || progress.second_link.is_none() {
            return Err(format!(
                "Godot exited {status} at phase {} (second connection: {:?})",
                progress.phase, progress.second_link
            ));
        }
        println!(
            "PASS: Godot automatically reauthenticated over a fresh UDP link and recovered selected world"
        );
        return Ok(true);
    }
    Ok(false)
}

fn run_fixture(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    reader: thread::JoinHandle<()>,
) -> Result<(), String> {
    let mut progress = Progress::new();
    let deadline = Instant::now() + FIXTURE_TIMEOUT;
    let mut reader = Some(reader);
    while Instant::now() < deadline {
        update_server(app, &mut progress)?;
        if poll_child(app, child, &lines, &mut reader, &mut progress)? {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for reconnect phase {} (second connection: {:?})",
        progress.phase, progress.second_link
    ))
}

fn main() {
    let (mut app, address) = start_server();
    println!("FIXTURE ENDPOINT {address}");
    let (mut child, lines, reader) = launch_godot(address);
    let result = run_fixture(&mut app, &mut child, lines, reader);
    if let Err(error) = result {
        panic!("native reconnect fixture: {error}");
    }
}
