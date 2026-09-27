//! Owned UDP replication and isolated Godot assets for native ordinary creature visuals.
//! Run from godot/ after its GDExtension is built:
//! GODOT_BIN=/path/to/godot cargo run -p game-engine-network --example native_npc_visual_fixture

use std::{
    fs,
    io::{BufRead, BufReader, Read},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{
    self as network, server, LinkOf, MessageReceiver, MessageSender, NetworkTarget, Replicate,
    ReplicationSender,
};
use shared::{
    components::{Health, ModelDisplay, MovementControl, Npc, Player, Position},
    protocol::{
        AuthChannel, CharacterListEntry, EnterWorldResponse, LoadTerrain, LoginRequest,
        LoginResponse, SelectCharacter, TerrainChannel,
    },
};

const TICK: Duration = Duration::from_millis(5);
const NAME: &str = "Fixture Player";
const NPC: &str = "Fixture Creature";
const DEAD_ON_SPAWN: &str = "Fixture Dead on Spawn";
const APPEARANCE_NPC: &str = "Fixture Appearance";
const MISSING_TYPE6_NPC: &str = "Fixture Missing Type6";
const HAIR_TYPE6_NPC: &str = "Fixture Hair Type6";
const TYPE19_EFFECT_NPC: &str = "Fixture Type19 Effect";

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
            &data.join("terrain"),
        ] {
            fs::create_dir_all(folder)
                .map_err(|error| format!("Create {}: {error}", folder.display()))?;
        }
        for entry in fs::read_dir(repo.join("data/textures"))
            .map_err(|error| format!("List authored UI textures: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("Read authored UI texture entry: {error}"))?;
            let name = entry.file_name();
            if name.to_str().is_some_and(|name| {
                [
                    "910001.blp",
                    "910002.blp",
                    "910020.blp",
                    "910021.blp",
                    "910022.blp",
                    "910023.blp",
                    "910024.blp",
                    "910025.blp",
                    "3484643.blp",
                ]
                .contains(&name)
            }) {
                continue;
            }
            std::os::unix::fs::symlink(entry.path(), data.join("textures").join(name))
                .map_err(|error| format!("Link authored UI texture: {error}"))?;
        }
        for folder in ["glues", "fonts", "ui"] {
            std::os::unix::fs::symlink(repo.join("data").join(folder), data.join(folder))
                .map_err(|error| format!("Link authored {folder} assets: {error}"))?;
        }
        for name in [
            "WarbandScene.csv",
            "WarbandScenePlacement.csv",
            "WarbandScenePlacementOption.csv",
        ] {
            std::os::unix::fs::symlink(repo.join("data").join(name), data.join(name))
                .map_err(|error| format!("Link authored {name}: {error}"))?;
        }
        for name in ["project.godot", "scenes", "shaders", "tests", "ui"] {
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
            INSERT INTO creature_displays VALUES (910015,910011,910002,0,0,1); \
            INSERT INTO creature_displays VALUES (910012,910013,910001,0,0,1000); \
            INSERT INTO creature_displays VALUES (910013,910013,910001,0,0,1000); \
            INSERT INTO creature_displays VALUES (910014,910014,910001,0,0,1000); \
            INSERT INTO creature_displays VALUES (910016,910016,910001,0,0,1000); \
            INSERT INTO creature_displays VALUES (910017,910017,910001,0,0,1000);";
        let status = Command::new("sqlite3")
            .arg(data.join("cache/creature_display.sqlite"))
            .arg(sql)
            .status()
            .map_err(|error| format!("Run sqlite3 fixture setup: {error}"))?;
        if !status.success() {
            return Err(format!("sqlite3 fixture setup exited {status}"));
        }
        stage_lighting(&data)?;
        stage_npc_appearance(&data)?;
        Ok(Self { root, project })
    }
}

fn write_sqlite_fixture(data: &Path, name: &str, sql: &str) -> Result<(), String> {
    let path = data.join("cache").join(name);
    let output = Command::new("sqlite3")
        .arg(&path)
        .arg(sql)
        .output()
        .map_err(|error| format!("Create {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Create {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn stage_npc_appearance(data: &Path) -> Result<(), String> {
    write_sqlite_fixture(data, "npc_appearance.sqlite", "
        CREATE TABLE display_coverage (display_id INTEGER PRIMARY KEY, requires_appearance INTEGER NOT NULL);
        CREATE TABLE appearances (display_id INTEGER PRIMARY KEY, race INTEGER NOT NULL, sex INTEGER NOT NULL, class INTEGER NOT NULL, baked_texture_fdid INTEGER NOT NULL);
        CREATE TABLE choices (display_id INTEGER NOT NULL, choice_id INTEGER NOT NULL, PRIMARY KEY(display_id, choice_id));
        CREATE TABLE geosets (display_id INTEGER NOT NULL, geoset_index INTEGER NOT NULL, geoset_value INTEGER NOT NULL, PRIMARY KEY(display_id, geoset_index));
        INSERT INTO display_coverage VALUES (910010,0),(910011,0),(910015,0),(910012,1),(910013,1),(910014,1),(910016,1),(910017,1);
        INSERT INTO appearances VALUES (910012,1,0,2,910020),(910013,1,0,2,0),(910014,1,0,2,0),(910016,2,0,2,0),(910017,1,0,2,0);
        INSERT INTO choices VALUES (910012,910030),(910012,910031),(910013,910030),(910013,910031),(910014,910030),(910014,910031),(910016,910032),(910016,910033),(910016,910034),(910016,910035),(910017,910030),(910017,910036);
        INSERT INTO geosets VALUES (910012,1,2),(910013,1,1),(910017,1,1);
    ")?;
    write_sqlite_fixture(data, "customization.sqlite", "
        CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
        CREATE TABLE chr_models (id INTEGER PRIMARY KEY, layout_id INTEGER NOT NULL, customize_scale REAL NOT NULL, camera_distance_offset REAL NOT NULL);
        CREATE TABLE options (id INTEGER PRIMARY KEY, name TEXT NOT NULL, chr_model_id INTEGER NOT NULL, category_id INTEGER NOT NULL, order_index INTEGER NOT NULL, ui_type INTEGER NOT NULL, requirement_id INTEGER NOT NULL);
        CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL, order_index INTEGER NOT NULL, icon INTEGER NOT NULL, selected_icon INTEGER NOT NULL);
        CREATE TABLE choices (id INTEGER PRIMARY KEY, option_id INTEGER NOT NULL, name TEXT NOT NULL, requirement_id INTEGER NOT NULL, order_index INTEGER NOT NULL, visibility_requirement_id INTEGER NOT NULL, swatch_color_0 INTEGER NOT NULL, swatch_color_1 INTEGER NOT NULL);
        CREATE TABLE elements (choice_id INTEGER NOT NULL, related_choice_id INTEGER NOT NULL, geoset_id INTEGER NOT NULL, material_id INTEGER NOT NULL, has_unsupported_effects INTEGER NOT NULL);
        CREATE TABLE materials (id INTEGER PRIMARY KEY, texture_target_id INTEGER NOT NULL, material_resources_id INTEGER NOT NULL);
        CREATE TABLE geosets (id INTEGER PRIMARY KEY, geoset_type INTEGER NOT NULL, geoset_id INTEGER NOT NULL);
        CREATE TABLE hair_geosets (model_id INTEGER NOT NULL, geoset_type INTEGER NOT NULL, geoset_id INTEGER NOT NULL, shows_scalp INTEGER NOT NULL, PRIMARY KEY(model_id,geoset_type,geoset_id));
        CREATE TABLE texture_fdids (material_resources_id INTEGER PRIMARY KEY, file_data_id INTEGER NOT NULL);
        INSERT INTO chr_models VALUES (910040,910041,1.0,0.0),(910042,910043,1.0,0.0);
        INSERT INTO options VALUES (910050,'Skin',910040,910060,0,0,0),(910051,'Body',910040,910060,1,0,0),
            (910052,'Skin',910042,910060,0,0,0),(910053,'Body',910042,910060,1,0,0),
            (910054,'Head',910042,910060,2,0,0),(910055,'Hair',910042,910060,3,0,0),(910056,'Eye',910040,910060,2,0,0);
        INSERT INTO categories VALUES (910060,'Appearance',0,0,0);
        INSERT INTO choices VALUES (910030,910050,'Base skin',0,0,0,0,0),(910031,910051,'Body color',0,0,0,0,0),
            (910032,910052,'Base skin',0,0,0,0,0),(910033,910053,'Body color',0,0,0,0,0),
            (910034,910054,'Head color',0,0,0,0,0),(910035,910055,'Hair color',0,0,0,0,0),(910036,910056,'Eye color',0,0,0,0,0);
        INSERT INTO elements VALUES (910030,0,0,910070,0),(910031,910030,910080,910071,0),
            (910032,0,0,910070,0),(910033,0,0,910071,0),(910034,0,0,910074,0),(910035,0,0,910075,0),(910036,0,0,910076,0);
        INSERT INTO materials VALUES (910070,1,910072),(910071,2,910073),(910074,9,910076),(910075,10,910077),(910076,11,910078);
        INSERT INTO geosets VALUES (910080,1,2);
        INSERT INTO texture_fdids VALUES (910072,910021),(910073,910022),(910076,910023),(910077,910024),(910078,910025);
    ")?;
    write_sqlite_fixture(data, "char_texture.sqlite", "
        CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
        CREATE TABLE layers (texture_type INTEGER NOT NULL, layer INTEGER NOT NULL, blend_mode INTEGER NOT NULL, section_bitmask INTEGER NOT NULL, target_id INTEGER NOT NULL, layout_id INTEGER NOT NULL);
        CREATE TABLE sections (layout_id INTEGER NOT NULL, section_type INTEGER NOT NULL, x INTEGER NOT NULL, y INTEGER NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL, PRIMARY KEY(layout_id,section_type));
        CREATE TABLE layouts (id INTEGER PRIMARY KEY, width INTEGER NOT NULL, height INTEGER NOT NULL);
        INSERT INTO layouts VALUES (910041,2,2),(910043,2048,1024);
        INSERT INTO layers VALUES (1,0,0,-1,1,910041),(1,1,0,-1,2,910041),
            (1,0,0,-1,1,910043),(1,1,0,-1,2,910043),
            (19,2,0,-1,11,910041),
            (6,2,0,512,9,910043),(6,3,0,-1,10,910043);
        INSERT INTO sections VALUES (910043,9,0,0,1024,1024),(910043,10,1024,0,1024,1024);
    ")?;
    fs::write(
        data.join("ChrRaceXChrModel.csv"),
        "ChrRacesID,Sex,ChrModelID\n1,0,910040\n2,0,910042\n",
    )
    .map_err(|error| format!("Write fixture race-model CSV: {error}"))?;
    fs::write(
        data.join("ChrRaces.csv"),
        "ID,UnalteredVisualRaceID\n1,0\n2,0\n",
    )
    .map_err(|error| format!("Write fixture races CSV: {error}"))
}

fn stage_lighting(data: &Path) -> Result<(), String> {
    let listfile =
        "910090;world/maps/azeroth/azeroth.wdt\n910091;world/maps/kalimdor/kalimdor.wdt\n";
    fs::write(data.join("community-listfile.csv"), listfile)
        .map_err(|error| format!("Write fixture map listfile: {error}"))?;
    let mut wdt = Vec::new();
    wdt.extend_from_slice(b"REVM");
    wdt.extend_from_slice(&4u32.to_le_bytes());
    wdt.extend_from_slice(&18u32.to_le_bytes());
    wdt.extend_from_slice(b"DHPM");
    wdt.extend_from_slice(&32u32.to_le_bytes());
    wdt.extend_from_slice(&[0; 32]);
    for fdid in [910090, 910091] {
        fs::write(data.join(format!("terrain/{fdid}.wdt")), &wdt)
            .map_err(|error| format!("Write fixture WDT {fdid}: {error}"))?;
    }
    let lights = "ID,GameCoords_0,GameCoords_1,GameCoords_2,GameFalloffStart,GameFalloffEnd,ContinentID,LightParamsID_0,LightParamsID_1,LightParamsID_2,LightParamsID_3,LightParamsID_4,LightParamsID_5,LightParamsID_6,LightParamsID_7\n\
1,0,0,0,0,0,0,1,0,0,0,0,0,0,0\n\
2,60,-3,2,10,15,0,2,0,0,0,0,0,0,0\n\
3,0,0,0,0,0,1,3,0,0,0,0,0,0,0\n";
    fs::write(data.join("Light.csv"), lights)
        .map_err(|error| format!("Write fixture Light.csv: {error}"))?;
    let header = "ID,LightParamID,Time,DirectColor,AmbientColor,SkyTopColor,SkyMiddleColor,SkyBand1Color,SkyBand2Color,SkySmogColor,SkyFogColor,SunColor,CloudSunColor,CloudEmissiveColor,CloudLayer1AmbientColor,CloudLayer2AmbientColor,OceanCloseColor,OceanFarColor,RiverCloseColor,RiverFarColor,HorizonAmbientColor,GroundAmbientColor,FogEnd,FogScaler,SunFogStrength,CloudDensity,Field_10_0_0_44649_042,Field_12_0_0_63854_043\n";
    let mut keyframes = header.to_owned();
    for (id, ambient, direct) in [
        (1, 0x336699, 0x775533),
        (2, 0x995533, 0x337799),
        (3, 0x226688, 0x994433),
    ] {
        keyframes.push_str(&format!(
            "{id},{id},0,{direct},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},{ambient},1000,0.2,0.5,0.2,0,0\n"
        ));
    }
    fs::write(data.join("LightData.csv"), keyframes)
        .map_err(|error| format!("Write fixture LightData.csv: {error}"))
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
) -> (Child, Receiver<String>, Vec<thread::JoinHandle<()>>) {
    let binary = std::env::var("GODOT_BIN").expect("GODOT_BIN must name the fixture executable");
    let mut child = Command::new(binary)
        .args(["--headless", "--path"])
        .arg(project)
        .args(["--script", "res://tests/world_npc_visual_flow.gd"])
        .env("GODOT_TEST_SERVER", address.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native Godot fixture process");
    let stdout = child.stdout.take().expect("read Godot stdout");
    let stderr = child.stderr.take().expect("read Godot stderr");
    let (sender, receiver) = mpsc::channel();
    let readers = vec![
        read_godot_output(stdout, sender.clone()),
        read_godot_output(stderr, sender),
    ];
    (child, receiver, readers)
}

fn read_godot_output(
    output: impl Read + Send + 'static,
    sender: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let line = line.expect("read Godot fixture output");
            println!("godot: {line}");
            if sender.send(line).is_err() {
                return;
            }
        }
    })
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
        send::<_, TerrainChannel>(
            app,
            LoadTerrain {
                map_name: "azeroth".into(),
                initial_tile_y: 32,
                initial_tile_x: 48,
            },
        );
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
    spawn_named_npc(app, display_id, NPC)
}

fn spawn_named_npc(app: &mut App, display_id: u32, name: &str) -> Entity {
    app.world_mut()
        .spawn((
            Npc {
                template_id: 8,
                name: name.into(),
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
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut player = None;
    let mut npc = None;
    let mut phase = 0;
    let deadline = Instant::now() + Duration::from_secs(110);
    let mut readers = Some(readers);
    let mut saw_missing_type6_error = false;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut player, &mut npc)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("fixture readers") {
                reader.join().map_err(|_| "Godot reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            if line.contains("display 910014:")
                && line.contains("missing NPC replacement texture type 6")
            {
                saw_missing_type6_error = true;
            }
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
                        .entity_mut(player.expect("spawned player"))
                        .insert((
                            Position {
                                x: 60.0,
                                y: 2.0,
                                z: 3.0,
                            },
                            MovementControl {
                                epoch: 1,
                                controlled: true,
                            },
                        ));
                    phase = 3;
                }
                (3, "FIXTURE LIGHT_UPDATED") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(ModelDisplay { display_id: 910015 });
                    phase = 4;
                }
                (4, "FIXTURE CLAMP_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .remove::<ModelDisplay>();
                    phase = 5;
                }
                (5, "FIXTURE MODEL_REMOVED") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(ModelDisplay { display_id: 910010 });
                    phase = 6;
                }
                (6, "FIXTURE MODEL_RESTORED") => {
                    app.world_mut().despawn(npc.expect("spawned NPC"));
                    phase = 7;
                }
                (7, "FIXTURE NPC_REMOVED") => {
                    npc = Some(spawn_npc(app, 910010));
                    phase = 8;
                }
                (8, "FIXTURE NPC_RESTORED") => {
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "kalimdor".into(),
                            initial_tile_y: 32,
                            initial_tile_x: 48,
                        },
                    );
                    phase = 9;
                }
                (9, "FIXTURE MAP_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(Npc {
                            template_id: 32820,
                            name: NPC.into(),
                        });
                    phase = 10;
                }
                (10, "FIXTURE HIDDEN_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert((
                            Npc {
                                template_id: 6491,
                                name: NPC.into(),
                            },
                            Position {
                                x: 7.0,
                                y: 2.0,
                                z: 3.0,
                            },
                        ));
                    phase = 11;
                }
                (11, "FIXTURE DEAD_ONLY_ALIVE_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert((
                            Health {
                                current: 0.0,
                                max: 10.0,
                            },
                            Position {
                                x: 8.0,
                                y: 2.0,
                                z: 3.0,
                            },
                        ));
                    phase = 12;
                }
                (12, "FIXTURE REMOTE_DEAD_READY") => {
                    app.world_mut()
                        .entity_mut(player.expect("spawned player"))
                        .insert(Health {
                            current: 0.0,
                            max: 10.0,
                        });
                    phase = 13;
                }
                (13, "FIXTURE LOCAL_DEAD_READY") => {
                    app.world_mut()
                        .entity_mut(player.expect("spawned player"))
                        .remove::<Health>();
                    phase = 14;
                }
                (14, "FIXTURE HEALTH_REMOVED_READY") => {
                    app.world_mut()
                        .entity_mut(player.expect("spawned player"))
                        .insert(Health {
                            current: 10.0,
                            max: 10.0,
                        });
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(Position {
                            x: 9.0,
                            y: 2.0,
                            z: 3.0,
                        });
                    phase = 15;
                }
                (15, "FIXTURE RESURRECTED_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(Npc {
                            template_id: 12783,
                            name: NPC.into(),
                        });
                    phase = 16;
                }
                (16, "FIXTURE DAY_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(Npc {
                            template_id: 918,
                            name: NPC.into(),
                        });
                    phase = 17;
                }
                (17, "FIXTURE NIGHT_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned NPC"))
                        .insert(Npc {
                            template_id: 8,
                            name: NPC.into(),
                        });
                    phase = 18;
                }
                (18, "FIXTURE ALWAYS_READY") => {
                    app.world_mut().spawn((
                        Npc {
                            template_id: 8,
                            name: DEAD_ON_SPAWN.into(),
                        },
                        ModelDisplay { display_id: 910010 },
                        Position {
                            x: 5.0,
                            y: 2.0,
                            z: 3.0,
                        },
                        Health {
                            current: 0.0,
                            max: 10.0,
                        },
                        Replicate::to_clients(NetworkTarget::All),
                    ));
                    phase = 19;
                }
                (19, "FIXTURE DEAD_ON_SPAWN_READY") => {
                    npc = Some(spawn_named_npc(app, 910012, APPEARANCE_NPC));
                    phase = 20;
                }
                (20, "FIXTURE BAKED_READY") => {
                    app.world_mut()
                        .entity_mut(npc.expect("spawned baked NPC"))
                        .insert(ModelDisplay { display_id: 910013 });
                    phase = 21;
                }
                (21, "FIXTURE COMPOSED_READY") => {
                    spawn_named_npc(app, 910014, MISSING_TYPE6_NPC);
                    phase = 22;
                }
                (22, "FIXTURE TYPE6_MISSING_READY") => {
                    spawn_named_npc(app, 910016, HAIR_TYPE6_NPC);
                    phase = 23;
                }
                (23, "FIXTURE TYPE6_HAIR_READY") => {
                    spawn_named_npc(app, 910017, TYPE19_EFFECT_NPC);
                    phase = 24;
                }
                (24, "FIXTURE TYPE19_READY") => phase = 25,
                (25, "FIXTURE EFFECT_ISOLATED_READY") => phase = 26,
                (26, "FIXTURE RESET_READY") => phase = 27,
                (_, line) if line.starts_with("FIXTURE ") => {
                    return Err(format!("Out-of-order phase {phase}: {line}"));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if !status.success() || phase != 27 {
                return Err(format!("Godot exited {status} at phase {phase}"));
            }
            if !saw_missing_type6_error {
                return Err(
                    "Missing display 910014 required type-6 error from native model loading".into(),
                );
            }
            println!(
                "PASS: native UDP NPC visual lifecycle, authored lighting, visibility, missing type 6, bound hair type 6, type 19 and effect isolation"
            );
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
