//! Owned loopback server and real Godot process for the native world-transfer fixture.
//! Build it with the GDExtension, then run it from the originating checkout:
//! python3 scripts/depot-build.py --root "$PWD" --fixture native_transfer_fixture
//! GODOT_BIN=<godot> target/debug/examples/native_transfer_fixture [--global-wmo]

#[path = "fixture_support/mod.rs"]
mod fixture_support;

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
        LoginResponse, NewWorld, SelectCharacter, TerrainChannel, TransferAbortReason,
        TransferAborted, TransferChannel, WorldPortAck,
    },
};

const NAME: &str = "Transfer Fixture";
const FIRST: [f32; 3] = [-8949.0, 112.87991, 0.0];
const SECOND: [f32; 3] = [-8940.0, 117.38283, 0.0];
const STOCKADE: [f32; 3] = [103.0, -34.5, -76.0];
const TICK: Duration = Duration::from_millis(5);

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<LoginRequest>,
    selections: Vec<SelectCharacter>,
    acks: usize,
}

fn receive_requests(
    mut logins: Query<&mut MessageReceiver<LoginRequest>>,
    mut selections: Query<&mut MessageReceiver<SelectCharacter>>,
    mut acks: Query<&mut MessageReceiver<WorldPortAck>>,
    mut incoming: ResMut<Incoming>,
) {
    for mut receiver in &mut logins {
        incoming.logins.extend(receiver.receive());
    }
    for mut receiver in &mut selections {
        incoming.selections.extend(receiver.receive());
    }
    for mut receiver in &mut acks {
        incoming.acks += receiver.receive().count();
    }
}

fn send<M: network::Message, C: network::Channel>(app: &mut App, message: M) {
    let world = app.world_mut();
    let mut sender = world
        .query::<&mut MessageSender<M>>()
        .single_mut(world)
        .expect("one connected fixture sender");
    sender.send::<C>(message);
}

fn start_server() -> (App, SocketAddr) {
    // ServerUdpIo does not expose its assigned port when bound to port zero.
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

fn launch_godot(
    address: SocketAddr,
    global_wmo: bool,
) -> (Child, Receiver<String>, Vec<thread::JoinHandle<()>>) {
    let binary = std::env::var("GODOT_BIN").expect("GODOT_BIN must name the fixture executable");
    let project = fixture_support::checkout_root_from_executable("native_transfer_fixture")
        .unwrap_or_else(|error| panic!("{error}"))
        .join("godot");
    let script = if global_wmo {
        "res://tests/world_global_wmo_flow.gd"
    } else {
        "res://tests/world_transfer_flow.gd"
    };
    let display_args: &[&str] = if std::env::var("GODOT_TEST_VISUAL").as_deref() == Ok("1") {
        &[]
    } else {
        &["--headless"]
    };
    let mut child = Command::new(binary)
        .args(display_args)
        .args([
            "--path",
            project.to_str().expect("UTF-8 Godot project path"),
            "--script",
            script,
        ])
        .env("GODOT_TEST_SERVER", address.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native Godot fixture process");
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

fn respond_to_login(app: &mut App) -> Result<(), String> {
    let mut incoming = app.world_mut().resource_mut::<Incoming>();
    let requests = std::mem::take(&mut incoming.logins);
    if requests.is_empty() {
        return Ok(());
    }
    if requests.len() != 1 {
        return Err(format!("expected one LoginRequest, got {}", requests.len()));
    }
    let request = &requests[0];
    if request.username != "fixture" || request.password != "fixture" || request.token.is_some() {
        return Err("unexpected fixture credentials or cached token".into());
    }
    let character = CharacterListEntry {
        character_id: 17,
        name: NAME.into(),
        level: 10,
        race: 1,
        class: 2,
        appearance: Default::default(),
        equipment_appearance: Default::default(),
    };
    send::<_, AuthChannel>(
        app,
        LoginResponse {
            success: true,
            token: "fixture-only-token".into(),
            characters: vec![character],
            error: None,
        },
    );
    Ok(())
}

fn respond_to_selection(
    app: &mut App,
    selected: &mut bool,
    global_wmo: bool,
) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
    for request in requests {
        if *selected || request.character_id != 17 {
            return Err(format!(
                "unexpected SelectCharacter: {}",
                request.character_id
            ));
        }
        *selected = true;
        let position = if global_wmo { STOCKADE } else { FIRST };
        let player = app
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
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        send::<_, AuthChannel>(
            app,
            EnterWorldResponse {
                success: true,
                player_entity: Some(player.to_bits()),
                error: None,
            },
        );
        let (map_name, initial_tile_y, initial_tile_x) = if global_wmo {
            ("stormwindjail", 0, 0)
        } else {
            ("azeroth", 32, 48)
        };
        send::<_, TerrainChannel>(
            app,
            LoadTerrain {
                map_name: map_name.into(),
                initial_tile_y,
                initial_tile_x,
            },
        );
    }
    Ok(())
}

fn run_fixture(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = false;
    let mut phase = 0;
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut readers = Some(readers);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut selected, false)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("fixture output readers") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            if line.starts_with("GODOT_STDERR: ERROR:") {
                return Err(format!("Godot runtime error: {line}"));
            }
            match (phase, line.trim()) {
                (0, "FIXTURE INITIAL_READY") if selected => {
                    if app.world().resource::<Incoming>().acks != 0 {
                        return Err("WorldPortAck arrived before NewWorld".into());
                    }
                    send::<_, TransferChannel>(
                        app,
                        NewWorld {
                            map_id: 0,
                            map_directory: "azeroth".into(),
                            position: SECOND,
                            facing: 0.5,
                        },
                    );
                    phase = 1;
                }
                (1, "FIXTURE TRANSFER_LOADING") => {
                    send::<_, TransferChannel>(
                        app,
                        TransferAborted {
                            map_id: 0,
                            reason: TransferAbortReason::MaxPlayers,
                        },
                    );
                    phase = 2;
                }
                (2, "FIXTURE TRANSFER_WATER_READY") => phase = 3,
                (3, "FIXTURE TRANSFER_READY") => phase = 4,
                (_, line) if line.starts_with("FIXTURE ") => {
                    return Err(format!("out-of-order Godot fixture phase {phase}: {line}"));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if !status.success() || phase != 4 {
                return Err(format!("Godot exited {status} at phase {phase}"));
            }
            let acks = app.world().resource::<Incoming>().acks;
            if acks != 1 {
                return Err(format!("expected exactly one WorldPortAck, got {acks}"));
            }
            println!(
                "PASS: real Godot transfer released old Water and material, recreated advancing water, and sent one WorldPortAck"
            );
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for Godot transfer phase {phase}"
    ))
}

fn run_global_wmo_fixture(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = false;
    let mut phase = 0;
    let deadline = Instant::now() + Duration::from_secs(130);
    let mut readers = Some(readers);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut selected, true)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("fixture output readers") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            if line.starts_with("GODOT_STDERR: ERROR:") {
                return Err(format!("Godot runtime error: {line}"));
            }
            match (phase, line.trim()) {
                (0, "FIXTURE GLOBAL_WMO_METADATA") if selected => phase = 1,
                (1, "FIXTURE GLOBAL_WMO_READY") => phase = 2,
                (2, "FIXTURE GLOBAL_WMO_RESET") => phase = 3,
                (_, line) if line.starts_with("FIXTURE ") => {
                    return Err(format!("out-of-order global WMO phase {phase}: {line}"));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if !status.success() || phase != 3 {
                return Err(format!("Godot exited {status} at global WMO phase {phase}"));
            }
            if app.world().resource::<Incoming>().acks != 0 {
                return Err("unexpected WorldPortAck without a NewWorld transfer".into());
            }
            println!("PASS: real Godot Stockade WMO attached, grounded, and reset");
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out at global WMO phase {phase}; WDT metadata must precede native attachment"
    ))
}

fn main() {
    let global_wmo = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [arg] if arg == "--global-wmo" => true,
        _ => panic!("usage: native_transfer_fixture [--global-wmo]"),
    };
    let (mut app, address) = start_server();
    println!("FIXTURE ENDPOINT {address}");
    let (mut child, lines, readers) = launch_godot(address, global_wmo);
    let result = if global_wmo {
        run_global_wmo_fixture(&mut app, &mut child, lines, readers)
    } else {
        run_fixture(&mut app, &mut child, lines, readers)
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
        panic!("native transfer fixture: {error}");
    }
}
