//! Owned UDP replication and isolated Godot assets for native ordinary creature visuals.
//! Run from godot/ after its GDExtension is built:
//! GODOT_BIN=/path/to/godot cargo run -p game-engine-network --example native_npc_visual_fixture

use std::{
    fs,
    io::{BufRead, BufReader},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
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
    components::{ModelDisplay, Npc, Player, Position},
    protocol::{
        AuthChannel, CharacterListEntry, EnterWorldResponse, LoginRequest, LoginResponse,
        SelectCharacter,
    },
};

const TICK: Duration = Duration::from_millis(5);
const NAME: &str = "Fixture Player";
const NPC: &str = "Fixture Creature";

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<LoginRequest>,
    selections: Vec<SelectCharacter>,
}

fn receive_requests(
    mut logins: Query<&mut MessageReceiver<LoginRequest>>,
    mut selections: Query<&mut MessageReceiver<SelectCharacter>>,
    mut incoming: ResMut<Incoming>,
) {
    for mut receiver in &mut logins {
        incoming.logins.extend(receiver.receive());
    }
    for mut receiver in &mut selections {
        incoming.selections.extend(receiver.receive());
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
    let reservation = UdpSocket::bind("127.0.0.1:0").expect("reserve fixture port");
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

struct FixtureProject {
    root: PathBuf,
    project: PathBuf,
}

impl FixtureProject {
    fn create() -> Result<Self, String> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Missing Godot project directory")?;
        let repo = source.parent().ok_or("Missing repository directory")?;
        // This subtree is untracked and disposable; res://../data is only this fixture's data.
        let root = repo
            .join("data")
            .join(format!("native-npc-fixture-{}", std::process::id()));
        let project = root.join("godot");
        let data = root.join("data");
        for folder in [
            project.as_path(),
            data.as_path(),
            &data.join("cache"),
            &data.join("models"),
            &data.join("textures"),
        ] {
            fs::create_dir_all(folder)
                .map_err(|error| format!("Create {}: {error}", folder.display()))?;
        }
        for entry in fs::read_dir(repo.join("data/textures"))
            .map_err(|error| format!("List authored UI textures: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("Read authored UI texture entry: {error}"))?;
            std::os::unix::fs::symlink(entry.path(), data.join("textures").join(entry.file_name()))
                .map_err(|error| format!("Link authored UI texture: {error}"))?;
        }
        for folder in ["glues", "fonts", "ui"] {
            std::os::unix::fs::symlink(repo.join("data").join(folder), data.join(folder))
                .map_err(|error| format!("Link authored {folder} assets: {error}"))?;
        }
        for name in ["project.godot", "scenes", "shaders", "tests"] {
            std::os::unix::fs::symlink(source.join(name), project.join(name))
                .map_err(|error| format!("Link {name}: {error}"))?;
        }
        fs::copy(
            source.join("game_engine.gdextension"),
            project.join("game_engine.gdextension"),
        )
        .map_err(|error| format!("Copy extension manifest: {error}"))?;
        fs::create_dir_all(project.join(".godot"))
            .map_err(|error| format!("Create Godot extension cache: {error}"))?;
        fs::write(
            project.join(".godot/extension_list.cfg"),
            "res://game_engine.gdextension\n",
        )
        .map_err(|error| format!("Register extension manifest: {error}"))?;
        std::os::unix::fs::symlink(repo.join("target"), root.join("target"))
            .map_err(|error| format!("Link existing extension: {error}"))?;
        let sql = "CREATE TABLE creature_displays (display_id INTEGER PRIMARY KEY, model_fdid INTEGER NOT NULL, skin_fdid_0 INTEGER NOT NULL, skin_fdid_1 INTEGER NOT NULL, skin_fdid_2 INTEGER NOT NULL, scale_milli INTEGER NOT NULL); \
            INSERT INTO creature_displays VALUES (910010,910010,910001,0,0,1500); \
            INSERT INTO creature_displays VALUES (910011,910011,910002,0,0,2000); \
            INSERT INTO creature_displays VALUES (910012,910011,910002,0,0,1);";
        let status = Command::new("sqlite3")
            .arg(data.join("cache/creature_display.sqlite"))
            .arg(sql)
            .status()
            .map_err(|error| format!("Run sqlite3 fixture setup: {error}"))?;
        if !status.success() {
            return Err(format!("sqlite3 fixture setup exited {status}"));
        }
        Ok(Self { root, project })
    }
}

impl Drop for FixtureProject {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("Cannot remove fixture {}: {error}", self.root.display());
        }
    }
}

fn launch_godot(
    project: &Path,
    address: SocketAddr,
) -> (Child, Receiver<String>, thread::JoinHandle<()>) {
    let binary = std::env::var("GODOT_BIN").expect("GODOT_BIN must name the fixture executable");
    let mut child = Command::new(binary)
        .args(["--headless", "--path"])
        .arg(project)
        .args(["--script", "res://tests/world_npc_visual_flow.gd"])
        .env("GODOT_TEST_SERVER", address.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
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

fn respond_to_login(app: &mut App) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
    for request in requests {
        if request.username != "fixture" || request.password != "fixture" || request.token.is_some()
        {
            return Err(format!("Unexpected login: {request:?}"));
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
                    equipment_appearance: Default::default(),
                }],
                error: None,
            },
        );
    }
    Ok(())
}

fn respond_to_selection(
    app: &mut App,
    player: &mut Option<Entity>,
    npc: &mut Option<Entity>,
) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
    for request in requests {
        if request.character_id != 17 || player.is_some() {
            return Err(format!("Unexpected selection: {request:?}"));
        }
        let spawned = app
            .world_mut()
            .spawn((
                Player {
                    name: NAME.into(),
                    race: 1,
                    class: 2,
                    appearance: Default::default(),
                },
                Position {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        *player = Some(spawned);
        *npc = Some(spawn_npc(app, 910010));
        send::<_, AuthChannel>(
            app,
            EnterWorldResponse {
                success: true,
                player_entity: Some(spawned.to_bits()),
                error: None,
            },
        );
    }
    Ok(())
}

fn spawn_npc(app: &mut App, display_id: u32) -> Entity {
    app.world_mut()
        .spawn((
            Npc {
                template_id: 8,
                name: NPC.into(),
            },
            ModelDisplay { display_id },
            Position {
                x: 5.0,
                y: 2.0,
                z: 3.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
}

fn run_fixture(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    reader: thread::JoinHandle<()>,
) -> Result<(), String> {
    let mut player = None;
    let mut npc = None;
    let mut phase = 0;
    let deadline = Instant::now() + Duration::from_secs(110);
    let mut reader = Some(reader);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut player, &mut npc)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            reader
                .take()
                .expect("fixture reader")
                .join()
                .map_err(|_| "Godot reader panicked")?;
        }
        for line in lines.try_iter() {
            match (phase, line.trim()) {
                (0, "FIXTURE INITIAL_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert((
                            ModelDisplay { display_id: 910010 },
                            Position {
                                x: 6.0,
                                y: 2.0,
                                z: 3.0,
                            },
                        ));
                    phase = 1;
                }
                (1, "FIXTURE SAME_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(ModelDisplay { display_id: 910011 });
                    phase = 2;
                }
                (2, "FIXTURE CHANGED_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(ModelDisplay { display_id: 910012 });
                    phase = 3;
                }
                (3, "FIXTURE CLAMP_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .remove::<ModelDisplay>();
                    phase = 4;
                }
                (4, "FIXTURE MODEL_REMOVED") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(ModelDisplay { display_id: 910010 });
                    phase = 5;
                }
                (5, "FIXTURE MODEL_RESTORED") => {
                    app.world_mut().despawn(npc.expect("spawned NPC"));
                    phase = 6;
                }
                (6, "FIXTURE NPC_REMOVED") => {
                    npc = Some(spawn_npc(app, 910010));
                    phase = 7;
                }
                (7, "FIXTURE NPC_RESTORED") => phase = 8,
                (8, "FIXTURE RESET_READY") => phase = 9,
                (_, line) if line.starts_with("FIXTURE ") => {
                    return Err(format!("Out-of-order phase {phase}: {line}"));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if !status.success() || phase != 9 {
                return Err(format!("Godot exited {status} at phase {phase}"));
            }
            println!("PASS: native UDP NPC visual lifecycle and isolated authored display assets");
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!("Timed out at NPC fixture phase {phase}"))
}

fn main() {
    let project = FixtureProject::create().expect("stage isolated fixture data and Godot project");
    let (mut app, address) = start_server();
    println!("FIXTURE ENDPOINT {address}");
    let (mut child, lines, reader) = launch_godot(&project.project, address);
    let result = run_fixture(&mut app, &mut child, lines, reader);
    if result.is_err() && child.try_wait().expect("inspect Godot status").is_none() {
        child.kill().expect("terminate failed Godot fixture");
        child.wait().expect("reap failed Godot fixture");
    }
    if let Err(error) = result {
        panic!("{error}");
    }
}
