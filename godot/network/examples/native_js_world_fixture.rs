//! Test-only production root-launcher JS RightClick/ShiftClick world slice.
//! MAIN owns building/running this installed example; no external game server.
//! Observer writes evidence only; JS owns all input. Peer owns all inventory/Gold.

#[path = "fixture_support/mod.rs"]
mod fixture_support;
use fixture_support::FixtureChild;

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender},
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
        AuthChannel, BagContents, BuyItem, CharacterListEntry, DestroyItem, EnterWorldResponse,
        EquipItem, InteractionChannel, InteractionKind, InteractionOpened, InventoryChannel,
        InventoryDelta, InventorySlotChange, InventorySnapshot, ItemLocation, ItemStack,
        LoadTerrain, LoginRequest, LoginResponse, MerchantChannel, NpcFlags, NpcRole,
        SelectCharacter, SellItem, SplitItem, SwapItem, TerrainChannel, UseItem, VendorInventory,
        VendorItem,
    },
};

const USERNAME: &str = "native-js-world-user";
const PASSWORD: &str = "native-js-world-secret";
const CHARACTER: &str = "JS World Fixture";
const VENDOR: &str = "Fixture Vendor";
const FIRST: [f32; 3] = [-8949.0, 112.879_913, 0.0];
const TIMEOUT: Duration = Duration::from_secs(300);
// EnterWorld is the actual shared char_select_component::ENTER_WORLD_BUTTON name.
// Fixed waits give the observer time to prove withholding, never generate actions.
const WORLD_SCRIPT: &str = concat!(
    "ui.waitForFrame(\"EnterWorld\", 30.0);\n",
    "ui.wait(2.0);\n",
    "ui.click(\"EnterWorld\");\n",
    "ui.waitForState(\"InWorld\", 180.0);\n",
    "ui.waitForFrame(\"MerchantItem1\", 60.0);\n",
    "ui.wait(2.0);\n",
    "ui.rightClick(\"MerchantItem1\");\n",
    "ui.wait(5.0);\n",
    "ui.shiftClick(\"MerchantItem1\");\n",
    "ui.waitForFrame(\"StackSplitFrame\", 6.0);\n",
    "ui.wait(2.0);\n",
    "ui.key(\"2\");\n",
    "ui.wait(2.0);\n",
    "ui.key(\"Enter\");\n",
    "ui.wait(5.0);\n",
    "ui.dumpUiTree();\n",
);

#[derive(Resource, Default)]
struct Incoming {
    logins: Vec<(Entity, LoginRequest)>,
    selections: Vec<SelectCharacter>,
    buys: Vec<BuyItem>,
    forbidden: Vec<String>,
}

fn receive_requests(
    mut logins: Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
    mut selections: Query<&mut MessageReceiver<SelectCharacter>>,
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
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
    for mut receiver in &mut buys {
        incoming.buys.extend(receiver.receive());
    }
}

fn reject_other_inventory<M: network::Message + std::fmt::Debug>(
    mut receivers: Query<&mut MessageReceiver<M>>,
    mut incoming: ResMut<Incoming>,
) {
    for mut receiver in &mut receivers {
        incoming
            .forbidden
            .extend(receiver.receive().map(|request| format!("{request:?}")));
    }
}

fn start_peer() -> Result<(App, SocketAddr), String> {
    let reservation = UdpSocket::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let address = reservation
        .local_addr()
        .map_err(|error| error.to_string())?;
    drop(reservation);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.init_resource::<Incoming>();
    app.add_systems(
        Update,
        (
            receive_requests,
            reject_other_inventory::<SellItem>,
            reject_other_inventory::<SwapItem>,
            reject_other_inventory::<SplitItem>,
            reject_other_inventory::<DestroyItem>,
            reject_other_inventory::<EquipItem>,
            reject_other_inventory::<UseItem>,
        ),
    );
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
    Ok((app, address))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Login,
    Selection,
    Loading,
    World,
    Npc,
    FirstBuy,
    FirstCommit,
    FirstApplied,
    SecondBuy,
    SecondCommit,
    SecondApplied,
    Done,
}

struct Peer {
    phase: Phase,
    link: Option<Entity>,
    player: Option<Entity>,
    vendor: Option<Entity>,
    buys: usize,
    decoded_at: Option<Instant>,
}

impl Peer {
    fn new() -> Self {
        Self {
            phase: Phase::Login,
            link: None,
            player: None,
            vendor: None,
            buys: 0,
            decoded_at: None,
        }
    }

    fn send<M: network::Message, C: network::Channel>(
        &self,
        app: &mut App,
        message: M,
    ) -> Result<(), String> {
        let link = self.link.ok_or("SETUP: response before owned login")?;
        app.world_mut()
            .get_mut::<MessageSender<M>>(link)
            .ok_or("SETUP: shared protocol missing response sender")?
            .send::<C>(message);
        Ok(())
    }

    fn authenticate(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
        for (link, request) in requests {
            if self.phase != Phase::Login
                || request.username != USERNAME
                || request.password != PASSWORD
                || request.token.is_some()
            {
                return Err(
                    "FEATURE: unexpected/duplicate owned LoginRequest; password masked".into(),
                );
            }
            self.link = Some(link);
            self.send::<_, AuthChannel>(
                app,
                LoginResponse {
                    success: true,
                    token: "native-js-world-token".into(),
                    error: None,
                    characters: vec![CharacterListEntry {
                        character_id: 17,
                        name: CHARACTER.into(),
                        level: 10,
                        race: 1,
                        class: 2,
                        appearance: Default::default(),
                        equipment_appearance: starter_equipment(),
                    }],
                },
            )?;
            self.phase = Phase::Selection;
            println!("AUTH exact owned credentials password=*** roster17 race1/class2");
        }
        Ok(())
    }

    fn select(&mut self, app: &mut App, artifacts: &Path) -> Result<(), String> {
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
        for request in requests {
            if self.phase != Phase::Selection
                || request.character_id != 17
                || !artifacts.join("charselect").is_file()
            {
                return Err(format!(
                    "FEATURE: unexpected selection {} in {:?} or missing authored CharSelect",
                    request.character_id, self.phase
                ));
            }
            let player = app
                .world_mut()
                .spawn((
                    Player {
                        name: CHARACTER.into(),
                        race: 1,
                        class: 2,
                        appearance: Default::default(),
                    },
                    starter_equipment(),
                    Gold(1000),
                    UnitFactionTemplate(1),
                    Position {
                        x: FIRST[0],
                        y: FIRST[1],
                        z: FIRST[2],
                    },
                    Replicate::to_clients(NetworkTarget::All),
                ))
                .id();
            self.player = Some(player);
            self.send::<_, AuthChannel>(
                app,
                EnterWorldResponse {
                    success: true,
                    player_entity: Some(player.to_bits()),
                    error: None,
                },
            )?;
            self.phase = Phase::Loading;
            println!(
                "SELECT exact17 player={} authored starter equipment; terrain withheld",
                player.to_bits()
            );
        }
        Ok(())
    }

    fn send_terrain_spawn_npc_and_open_vendor(
        &mut self,
        app: &mut App,
        artifacts: &Path,
    ) -> Result<(), String> {
        match self.phase {
            Phase::Loading if artifacts.join("loading").is_file() => {
                self.send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                )?;
                self.phase = Phase::World;
            }
            Phase::World if artifacts.join("world-ready").is_file() => {
                self.vendor = Some(
                    app.world_mut()
                        .spawn((
                            Npc {
                                template_id: 1213,
                                name: VENDOR.into(),
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
                        .id(),
                );
                self.phase = Phase::Npc;
            }
            Phase::Npc if artifacts.join("npc-ready").is_file() => {
                self.open_vendor(app)?;
                self.phase = Phase::FirstBuy;
            }
            _ => {}
        }
        Ok(())
    }

    fn open_vendor(&self, app: &mut App) -> Result<(), String> {
        let npc = self
            .vendor
            .ok_or("SETUP: missing replicated vendor")?
            .to_bits();
        // Authorized fixture session, not a claim about server proximity/pricing rules.
        self.send::<_, InteractionChannel>(
            app,
            InteractionOpened {
                npc,
                kind: InteractionKind::Role(NpcRole::Vendor),
            },
        )?;
        self.send::<_, InventoryChannel>(
            app,
            InventorySnapshot {
                bags: vec![BagContents {
                    bag: 0,
                    size: 16,
                    items: vec![],
                }],
            },
        )?;
        self.send::<_, MerchantChannel>(
            app,
            VendorInventory {
                npc,
                can_repair: false,
                guild_repair_money: None,
                items: vec![VendorItem {
                    slot: 0,
                    item_id: 2589,
                    name: "Linen Cloth".into(),
                    quality: 1,
                    price: 25,
                    stack_count: 1,
                    max_stack: 1000,
                    num_available: None,
                    usable: true,
                    max_durability: None,
                }],
            },
        )
    }

    fn receive_buys(&mut self, app: &mut App, artifacts: &Path) -> Result<(), String> {
        let forbidden = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().forbidden);
        if !forbidden.is_empty() {
            return Err(format!("FEATURE: unexpected transactions {forbidden:?}"));
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().buys);
        for request in requests {
            let (count, marker, next) = match self.phase {
                Phase::FirstBuy if artifacts.join("merchant-ready").is_file() => {
                    (1, "decoded1", Phase::FirstCommit)
                }
                Phase::SecondBuy
                    if artifacts.join("picker1").is_file()
                        && artifacts.join("picker2").is_file() =>
                {
                    (2, "decoded2", Phase::SecondCommit)
                }
                _ => {
                    return Err(format!(
                        "FEATURE: duplicate/out-of-order Buy {request:?} in {:?}",
                        self.phase
                    ));
                }
            };
            let expected = BuyItem {
                npc: self.vendor.ok_or("SETUP: buy without vendor")?.to_bits(),
                slot: 0,
                item_id: 2589,
                count,
                destination: None,
            };
            if request != expected {
                return Err(format!(
                    "FEATURE: expected {expected:?}, decoded {request:?}"
                ));
            }
            self.buys += 1;
            self.decoded_at = Some(Instant::now());
            self.phase = next;
            mark(artifacts, marker)?;
            println!("DECODED exact {request:?}; authority withheld until observer COMMIT");
        }
        Ok(())
    }

    fn commit_buys(&mut self, app: &mut App, artifacts: &Path) -> Result<(), String> {
        let commit = match self.phase {
            Phase::FirstCommit if artifacts.join("commit1").is_file() => {
                Some((1, 975, Phase::FirstApplied))
            }
            Phase::SecondCommit if artifacts.join("commit2").is_file() => {
                Some((3, 925, Phase::SecondApplied))
            }
            _ => None,
        };
        if let Some((count, gold, phase)) = commit {
            if self
                .decoded_at
                .ok_or("SETUP: commit without decoded Buy")?
                .elapsed()
                < Duration::from_millis(900)
            {
                return Err("FEATURE: observer committed before 900ms withholding".into());
            }
            self.send::<_, InventoryChannel>(
                app,
                InventoryDelta {
                    changes: vec![InventorySlotChange {
                        location: ItemLocation::Bag { bag: 0, slot: 0 },
                        item: Some(ItemStack {
                            item_guid: 9_182_589,
                            item_id: 2589,
                            count,
                            durability: None,
                            soulbound: false,
                        }),
                    }],
                },
            )?;
            let player = self.player.ok_or("SETUP: commit without selected player")?;
            app.world_mut().entity_mut(player).insert(Gold(gold));
            self.phase = phase;
            println!(
                "AUTHORITY Linen2589 guid9182589 bag0/slot0 count{count} Gold{gold}; fixture price25 only"
            );
        }
        if self.phase == Phase::FirstApplied && artifacts.join("applied1").is_file() {
            self.phase = Phase::SecondBuy;
        }
        if self.phase == Phase::SecondApplied && artifacts.join("done").is_file() {
            self.phase = Phase::Done;
        }
        Ok(())
    }

    fn tick(&mut self, app: &mut App, artifacts: &Path) -> Result<(), String> {
        self.authenticate(app)?;
        self.select(app, artifacts)?;
        self.send_terrain_spawn_npc_and_open_vendor(app, artifacts)?;
        self.receive_buys(app, artifacts)?;
        self.commit_buys(app, artifacts)
    }
}

fn mark(artifacts: &Path, name: &str) -> Result<(), String> {
    fs::write(artifacts.join(name), "decoded\n")
        .map_err(|error| format!("SETUP: marker {name}: {error}"))
}

struct Output {
    line: String,
    stdout: bool,
    leaked: bool,
}
fn collect_output(
    stream: impl Read + Send + 'static,
    sender: Sender<Result<Output, String>>,
    stdout: bool,
) {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let result = line
                .map(|line| Output {
                    leaked: line.contains(PASSWORD),
                    line: line.replace(PASSWORD, "***"),
                    stdout,
                })
                .map_err(|error| format!("SETUP: child diagnostics: {error}"));
            if sender.send(result).is_err() {
                return;
            }
        }
    });
}

fn require_file(path: PathBuf, label: &str) -> Result<PathBuf, String> {
    if !path.is_file() {
        return Err(format!("SETUP: missing {label}: {}", path.display()));
    }
    Ok(path)
}

fn launch(
    root: &Path,
    artifacts: &Path,
    address: SocketAddr,
) -> Result<(FixtureChild, Receiver<Result<Output, String>>), String> {
    let launcher = require_file(
        root.join("target/debug/game-engine-launcher"),
        "production root launcher",
    )?;
    let godot = require_file(
        std::env::var_os("GODOT_BIN")
            .map(PathBuf::from)
            .ok_or("SETUP: existing GODOT_BIN required")?,
        "Godot executable",
    )?;
    require_file(
        root.join("godot/.godot/extension_list.cfg"),
        "imported native extension",
    )?;
    let login = fs::read_to_string(root.join("debug/login.js"))
        .map_err(|error| format!("SETUP: read unchanged login script: {error}"))?;
    let script = artifacts.join("login-and-world.js");
    fs::write(&script, format!("{login}\n{WORLD_SCRIPT}"))
        .map_err(|error| format!("SETUP: owned script: {error}"))?;
    for directory in ["config/world-of-osso", "user-data"] {
        fs::create_dir_all(artifacts.join(directory)).map_err(|error| error.to_string())?;
    }
    fs::write(
        artifacts.join("config/world-of-osso/options_settings.ron"),
        "()",
    )
    .map_err(|error| format!("SETUP: isolated settings: {error}"))?;
    let mut child = FixtureChild::spawn(
        Command::new(launcher)
            .current_dir(root)
            .args(["--headless", "--audio-driver", "Dummy", "--path"])
            .arg(root.join("godot"))
            .args([
                "--script",
                "res://tests/native_js_world_flow.gd",
                "--screen",
                "login",
                "--server",
            ])
            .arg(address.to_string())
            // No separator here: normal production ROOTLAUNCHER must route these flags.
            .arg("--run-js-ui-script")
            .arg(script)
            .env("GODOT_BIN", godot)
            .env("GAME_ENGINE_ROOT", root)
            .env("GAME_ENGINE_SHARED_ROOT", root)
            .env("GAME_ENGINE_SHARED_DATA_DIR", root.join("data"))
            // Preserve inherited warm CASC cache; never reset shared cache or download.
            .env("XDG_CONFIG_HOME", artifacts.join("config"))
            .env("XDG_DATA_HOME", artifacts.join("user-data"))
            .env("LOGIN_USER", USERNAME)
            .env("LOGIN_PASS", PASSWORD)
            .env("NATIVE_JS_WORLD_ARTIFACTS", artifacts)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .map_err(|error| format!("SETUP: root launch: {error}"))?;
    let (sender, receiver) = mpsc::channel();
    collect_output(
        child.stdout.take().ok_or("SETUP: stdout pipe")?,
        sender.clone(),
        true,
    );
    collect_output(
        child.stderr.take().ok_or("SETUP: stderr pipe")?,
        sender,
        false,
    );
    Ok((child, receiver))
}

fn record(
    output: Result<Output, String>,
    log: &mut fs::File,
    lines: &mut Vec<Output>,
) -> Result<(), String> {
    let output = output?;
    writeln!(log, "{}", output.line).map_err(|error| error.to_string())?;
    if output.leaked {
        return Err("FEATURE: raw owned password leaked; saved log masked".into());
    }
    if output.line.trim_start().starts_with("SCRIPT ERROR:") {
        return Err(format!(
            "SETUP/UNCLASSIFIED: GDScript failure, not feature RED: {}",
            output.line
        ));
    }
    if output.line.contains("native JS automation:") {
        return Err(format!(
            "FEATURE: production JS error/deadline: {}",
            output.line
        ));
    }
    lines.push(output);
    Ok(())
}

fn create_artifacts_and_log(root: &Path) -> Result<(PathBuf, fs::File), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let artifacts = root.join(format!(
        "data/diagnostics/native-js-world-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&artifacts).map_err(|error| error.to_string())?;
    let log = fs::File::create(artifacts.join("native.log")).map_err(|error| error.to_string())?;
    Ok((artifacts, log))
}

fn poll_child_exit_and_drain_output(
    child: &mut FixtureChild,
    exited: &mut Option<ExitStatus>,
    receiver: &Receiver<Result<Output, String>>,
    log: &mut fs::File,
    lines: &mut Vec<Output>,
) -> Result<bool, String> {
    if exited.is_none() {
        *exited = child.try_wait().map_err(|error| error.to_string())?;
    }
    if exited.is_some() {
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(output) => record(output, log, lines)?,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(true),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    Ok(false)
}

fn pump_peer_and_child_output_until_readers_close(
    app: &mut App,
    peer: &mut Peer,
    artifacts: &Path,
    child: &mut FixtureChild,
    receiver: &Receiver<Result<Output, String>>,
    log: &mut fs::File,
) -> Result<(Option<ExitStatus>, Vec<Output>), String> {
    let mut lines = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    let mut exited = None;
    loop {
        app.update();
        peer.tick(app, artifacts)?;
        for output in receiver.try_iter() {
            record(output, log, &mut lines)?;
        }
        if poll_child_exit_and_drain_output(child, &mut exited, receiver, log, &mut lines)? {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "SETUP/FEATURE UNCLASSIFIED: bounded 300s deadline in {:?}; inspect native.log and observation.jsonl; own PID cleanup only",
                peer.phase
            ));
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok((exited, lines))
}

fn assert_child_exit_and_peer_result(
    exited: Option<ExitStatus>,
    peer: &Peer,
) -> Result<(), String> {
    let status = exited.ok_or("SETUP: missing child exit")?;
    if !status.success() || peer.phase != Phase::Done || peer.buys != 2 {
        return Err(format!(
            "FEATURE/SETUP: child={status} phase={:?} buys={}; inspect artifacts; no automatic feature RED classification",
            peer.phase, peer.buys
        ));
    }
    Ok(())
}

fn assert_production_stdout_frames(lines: &[Output]) -> Result<(), String> {
    for frame in ["CharSelectCharacterName", "MerchantItem1", "MerchantFrame"] {
        if !lines.iter().any(|output| {
            output.stdout
                && output.line.contains(frame)
                && output.line.contains(" visible ")
                && output.line.contains("alpha=")
        }) {
            return Err(format!(
                "FEATURE: production JS stdout dump lacks visible {frame}"
            ));
        }
    }
    Ok(())
}

fn run_fixture() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("SETUP: usage: native_js_world_fixture".into());
    }
    let root = fixture_support::checkout_root_from_executable("native_js_world_fixture")?;
    let (artifacts, mut log) = create_artifacts_and_log(&root)?;
    let (mut app, address) = start_peer()?;
    let (mut child, receiver) = launch(&root, &artifacts, address)?;
    println!(
        "ARTIFACTS {} owned client PID={} loopback={address}",
        artifacts.display(),
        child.id()
    );
    let mut peer = Peer::new();
    let (exited, lines) = pump_peer_and_child_output_until_readers_close(
        &mut app, &mut peer, &artifacts, &mut child, &receiver, &mut log,
    )?;
    assert_child_exit_and_peer_result(exited, &peer)?;
    assert_production_stdout_frames(&lines)?;
    println!(
        "PASS bounded JS world: real owned auth/selection/terrain; exact Buy1 None + Shift/key2/Enter Buy2 None; both authority barriers; not server economy/global ownership/full conversion/shutdown proof"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run_fixture() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
