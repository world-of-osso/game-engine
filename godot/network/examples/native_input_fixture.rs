//! Real Godot UI/input → native movement → owned loopback UDP PlayerInput proof.
//! After building the Godot GDExtension, run:
//! GODOT_BIN=<godot executable> cargo run -p game-engine-network --example native_input_fixture

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
    components::{
        EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player, Position,
    },
    protocol::{
        AuthChannel, CharacterListEntry, EnterWorldResponse, LoadTerrain, LoginRequest,
        LoginResponse, PlayerInput, SelectCharacter, TerrainChannel,
    },
};

const NAME: &str = "Input Fixture";
// Authored terrain height; the transfer-only fixture's Y=83 is below this surface.
const FIRST: [f32; 3] = [-8949.0, 112.879_913, 0.0];
const TICK: Duration = Duration::from_millis(5);
const TIMEOUT: Duration = Duration::from_secs(90);
const RELEASE_DRAIN: Duration = Duration::from_millis(250);
const RELEASE_QUIET: Duration = Duration::from_millis(400);

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<LoginRequest>,
    selections: Vec<SelectCharacter>,
    inputs: Vec<PlayerInput>,
}

fn receive_requests(
    mut logins: Query<&mut MessageReceiver<LoginRequest>>,
    mut selections: Query<&mut MessageReceiver<SelectCharacter>>,
    mut inputs: Query<&mut MessageReceiver<PlayerInput>>,
    mut incoming: ResMut<Incoming>,
) {
    for mut receiver in &mut logins {
        incoming.logins.extend(receiver.receive());
    }
    for mut receiver in &mut selections {
        incoming.selections.extend(receiver.receive());
    }
    for mut receiver in &mut inputs {
        incoming.inputs.extend(receiver.receive());
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

fn launch_godot(address: SocketAddr) -> (Child, Receiver<String>, Vec<thread::JoinHandle<()>>) {
    let binary = std::env::var("GODOT_BIN").expect("GODOT_BIN must name the fixture executable");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Godot workspace directory");
    let display_args: &[&str] = if std::env::var("GODOT_TEST_VISUAL").as_deref() == Ok("1") {
        &["--display-driver", "wayland", "--audio-driver", "Dummy"]
    } else {
        &["--headless"]
    };
    let mut child = Command::new(binary)
        .args(display_args)
        .args([
            "--path",
            project.to_str().expect("UTF-8 Godot project path"),
            "--script",
            "res://tests/world_input_flow.gd",
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

fn respond_to_login(app: &mut App) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
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
    send::<_, AuthChannel>(
        app,
        LoginResponse {
            success: true,
            token: "fixture-only-token".into(),
            characters: vec![CharacterListEntry {
                character_id: 17,
                name: NAME.into(),
                level: 10,
                race: 1,
                class: 2,
                appearance: Default::default(),
                equipment_appearance: starter_equipment(),
            }],
            error: None,
        },
    );
    Ok(())
}

fn respond_to_selection(app: &mut App, selected: &mut bool) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
    for request in requests {
        if *selected || request.character_id != 17 {
            return Err(format!(
                "unexpected SelectCharacter: {}",
                request.character_id
            ));
        }
        *selected = true;
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
                    x: FIRST[0],
                    y: FIRST[1],
                    z: FIRST[2],
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
        // Intentionally withhold LoadTerrain until Loading input has been observed.
    }
    Ok(())
}

fn take_inputs(app: &mut App) -> Vec<PlayerInput> {
    std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().inputs)
}

fn assert_forward_input(inputs: Vec<PlayerInput>) -> Result<bool, String> {
    for input in &inputs {
        if !input.elapsed_secs.is_finite() || input.elapsed_secs <= 0.0 {
            return Err(format!(
                "W input lost its applied frame duration: {input:?}"
            ));
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

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    AwaitLoading,
    AwaitWorld,
    Held,
    Released,
    Stopped,
}

fn accept_phase_line(app: &mut App, phase: &mut Phase, line: &str) -> Result<(), String> {
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
        (Phase::AwaitWorld, "FIXTURE WORLD_READY") => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived before native InWorld readiness".into());
            }
            *phase = Phase::Held;
        }
        (Phase::Held, "FIXTURE RELEASED") => *phase = Phase::Released,
        (Phase::Released, "FIXTURE STOPPED") => *phase = Phase::Stopped,
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
) -> Result<(), String> {
    let mut selected = false;
    let mut phase = Phase::AwaitLoading;
    let mut saw_forward = false;
    let mut released_at = None;
    let deadline = Instant::now() + TIMEOUT;
    let mut reader = Some(reader);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut selected)?;
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
                && !selected
            {
                return Err("Loading was observed before character selection".into());
            }
            accept_phase_line(app, &mut phase, line.trim())?;
            if phase == Phase::Released && released_at.is_none() {
                released_at = Some(Instant::now());
            }
        }
        match phase {
            Phase::AwaitLoading | Phase::AwaitWorld => {
                if !take_inputs(app).is_empty() {
                    return Err(format!("PlayerInput arrived during {phase:?}"));
                }
            }
            Phase::Held => saw_forward |= assert_forward_input(take_inputs(app))?,
            Phase::Released | Phase::Stopped => {
                // Discard in-flight held-key messages before asserting a quiet interval.
                let released_for = released_at.expect("release marker recorded").elapsed();
                let inputs = take_inputs(app);
                if released_for >= RELEASE_DRAIN && !inputs.is_empty() {
                    return Err(format!(
                        "movement packets continued after release: {inputs:?}"
                    ));
                }
            }
        }
        if let Some(status) = status {
            if !status.success() || phase != Phase::Stopped {
                return Err(format!("Godot exited {status} at phase {phase:?}"));
            }
            if !saw_forward {
                return Err("no decoded held-W PlayerInput on InputChannel".into());
            }
            let released_for = released_at.expect("release marker recorded").elapsed();
            if released_for >= RELEASE_DRAIN + RELEASE_QUIET {
                println!(
                    "PASS: Loading blocked input; native W moved player and sent UDP; release stopped both"
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

fn main() {
    let (mut app, address) = start_server();
    println!("FIXTURE ENDPOINT {address}");
    let (mut child, lines, reader) = launch_godot(address);
    let result = run_fixture(&mut app, &mut child, lines, reader);
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
