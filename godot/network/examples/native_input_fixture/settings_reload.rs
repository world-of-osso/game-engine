//! Test-only two-process canonical settings reload and authoritative auto-loot peer.
use super::*;
use shared::protocol::{
    BagSlotItem, CorpseLootable, InventoryDelta, InventorySlotChange, ItemLocation, LootChannel,
    LootClosed, LootRelease, LootResponse, LootSlotRemoved, LootSlotRequest, LootUnit,
};

const FIRST_PASS_TIMEOUT: Duration = Duration::from_secs(300);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const INITIAL_COUNT: u32 = 3;
const FINAL_COUNT: u32 = 5;
const INITIAL_GOLD: u64 = 1_250;
const FINAL_GOLD: u64 = 11_752;

pub(super) struct FixtureContext<'a> {
    pub(super) root: &'a Path,
    pub(super) project: &'a Path,
    pub(super) config: &'a FixtureConfig,
    pub(super) address: SocketAddr,
}

#[derive(Resource, Default)]
struct Requests {
    units: Vec<LootUnit>,
    slots: Vec<LootSlotRequest>,
    releases: Vec<LootRelease>,
}

fn receive(
    mut units: Query<&mut MessageReceiver<LootUnit>>,
    mut slots: Query<&mut MessageReceiver<LootSlotRequest>>,
    mut releases: Query<&mut MessageReceiver<LootRelease>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut units {
        requests.units.extend(receiver.receive());
    }
    for mut receiver in &mut slots {
        requests.slots.extend(receiver.receive());
    }
    for mut receiver in &mut releases {
        requests.releases.extend(receiver.receive());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pass {
    Save,
    Load,
}

struct Session {
    player: Option<Entity>,
    remote: Option<Entity>,
    corpse: Option<u64>,
    units: usize,
    reload_selected: bool,
}

impl Session {
    fn new() -> Self {
        Self {
            player: None,
            remote: None,
            corpse: None,
            units: 0,
            reload_selected: false,
        }
    }

    fn respond_to_selection(&mut self, app: &mut App, pass: Pass) -> Result<(), String> {
        if pass == Pass::Save {
            respond_to_selection(
                app,
                StartupScreen::SettingsReload,
                &mut self.player,
                &mut self.remote,
            )?;
            if let Some(player) = self.player
                && self.corpse.is_none()
            {
                app.world_mut()
                    .entity_mut(player)
                    .insert((Gold(INITIAL_GOLD), UnitFactionTemplate(1)));
                self.corpse = Some(loot::spawn_corpse(app));
            }
            return Ok(());
        }
        let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().selections);
        if requests.len() > 1 {
            return Err(format!(
                "reload received duplicate selections: {requests:?}"
            ));
        }
        for request in requests {
            if self.reload_selected || request.character_id != 17 {
                return Err(format!(
                    "reload selected unexpected character: {}",
                    request.character_id
                ));
            }
            self.reload_selected = true;
            let player = self
                .player
                .ok_or("reload selection without original player")?;
            send::<_, AuthChannel>(
                app,
                EnterWorldResponse {
                    success: true,
                    player_entity: Some(player.to_bits()),
                    error: None,
                },
            );
        }
        Ok(())
    }

    fn corpse(&self) -> Result<u64, String> {
        self.corpse
            .ok_or_else(|| "settings marker before selected player and corpse".into())
    }

    fn send_ready(&self, app: &mut App) -> Result<(), String> {
        let corpse = self.corpse()?;
        send::<_, InventoryChannel>(
            app,
            InventorySnapshot {
                bags: vec![BagContents {
                    bag: 0,
                    size: 16,
                    items: vec![BagSlotItem {
                        slot: 0,
                        item: loot::candle_stack(INITIAL_COUNT),
                    }],
                }],
            },
        );
        send::<_, LootChannel>(
            app,
            CorpseLootable {
                corpse,
                lootable: true,
            },
        );
        Ok(())
    }

    fn respond_to_loot(&mut self, app: &mut App, pass: Pass, ready: bool) -> Result<(), String> {
        let requests = {
            let mut pending = app.world_mut().resource_mut::<Requests>();
            std::mem::take(&mut *pending)
        };
        if !requests.slots.is_empty() || !requests.releases.is_empty() {
            return Err(format!(
                "client requested server-owned loot slots/release: {:?} / {:?}",
                requests.slots, requests.releases
            ));
        }
        for request in requests.units {
            if pass != Pass::Load
                || !ready
                || self.units != 0
                || !request.auto
                || request.corpse != self.corpse()?
            {
                return Err(format!(
                    "unexpected settings loot request in {pass:?}: {request:?}"
                ));
            }
            self.units += 1;
            self.send_auto_loot(app, request.corpse)?;
        }
        Ok(())
    }

    fn send_auto_loot(&self, app: &mut App, corpse: u64) -> Result<(), String> {
        send::<_, LootChannel>(
            app,
            LootResponse {
                corpse,
                auto: true,
                slots: loot::slots(),
            },
        );
        for slot in [0, 1] {
            send::<_, LootChannel>(app, LootSlotRemoved { corpse, slot });
        }
        send::<_, InventoryChannel>(
            app,
            InventoryDelta {
                changes: vec![InventorySlotChange {
                    location: ItemLocation::Bag { bag: 0, slot: 0 },
                    item: Some(loot::candle_stack(FINAL_COUNT)),
                }],
            },
        );
        let player = self
            .player
            .ok_or("auto-loot without selected original player")?;
        app.world_mut().entity_mut(player).insert(Gold(FINAL_GOLD));
        send::<_, LootChannel>(
            app,
            CorpseLootable {
                corpse,
                lootable: false,
            },
        );
        send::<_, LootChannel>(app, LootClosed { corpse });
        Ok(())
    }
}

#[derive(Default)]
struct Progress {
    loading: bool,
    ready: bool,
    clicked: bool,
    done: bool,
    request_deadline: Option<Instant>,
}

fn reject_runtime_error(line: &str) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("settings reload Godot error: {line}"));
    }
    Ok(())
}

impl Progress {
    fn observe(
        &mut self,
        app: &mut App,
        session: &Session,
        pass: Pass,
        line: &str,
    ) -> Result<(), String> {
        reject_runtime_error(line)?;
        match line {
            "FIXTURE SETTINGS_LOADING" if !self.loading => {
                session.corpse()?;
                send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                );
                self.loading = true;
            }
            "FIXTURE SETTINGS_READY" if self.loading && !self.ready => {
                session.send_ready(app)?;
                self.ready = true;
            }
            "FIXTURE SETTINGS_SAVED" if pass == Pass::Save && self.ready && !self.done => {
                self.done = true
            }
            "FIXTURE SETTINGS_LOOT_CLICKED"
                if pass == Pass::Load && self.ready && !self.clicked =>
            {
                self.clicked = true;
                self.request_deadline = Some(Instant::now() + REQUEST_TIMEOUT);
            }
            "FIXTURE SETTINGS_LOADED" if pass == Pass::Load && self.clicked && !self.done => {
                self.done = true
            }
            other if other.starts_with("FIXTURE SETTINGS_") => {
                return Err(format!("out-of-order {pass:?} marker: {other}"));
            }
            _ => {}
        }
        Ok(())
    }

    fn require_requests(&self, session: &Session, pass: Pass) -> Result<(), String> {
        if self.done {
            let expected = usize::from(pass == Pass::Load);
            if session.units != expected {
                return Err(format!(
                    "{pass:?} completion requires {expected} LootUnit, got {}",
                    session.units
                ));
            }
        }
        if self
            .request_deadline
            .is_some_and(|end| Instant::now() >= end)
            && session.units == 0
        {
            return Err("settings corpse click produced no decoded LootUnit within 6s".into());
        }
        Ok(())
    }
}

fn run_pass(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
    session: &mut Session,
    pass: Pass,
) -> Result<(), String> {
    let timeout = if pass == Pass::Save {
        FIRST_PASS_TIMEOUT
    } else {
        TIMEOUT
    };
    let deadline = Instant::now() + timeout;
    let mut progress = Progress::default();
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::SettingsReload)?;
        session.respond_to_selection(app, pass)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect {pass:?} child: {error}"))?
        {
            return Err(format!(
                "settings {pass:?} child exited before owned cleanup: {status}"
            ));
        }
        for line in lines.try_iter() {
            progress.observe(app, session, pass, line.trim())?;
        }
        session.respond_to_loot(app, pass, progress.ready)?;
        progress.require_requests(session, pass)?;
        if progress.done {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "settings {pass:?} timed out after {}s",
        timeout.as_secs()
    ))
}

fn read_canonical(config: &FixtureConfig) -> Result<Vec<u8>, String> {
    let path = config.home.join("world-of-osso/options_settings.ron");
    let bytes =
        fs::read(&path).map_err(|error| format!("read canonical {}: {error}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("empty canonical settings: {}", path.display()));
    }
    Ok(bytes)
}

fn require_saved_bytes(config: &FixtureConfig, saved: &[u8], stage: &str) -> Result<(), String> {
    if read_canonical(config)? != saved {
        return Err(format!("canonical settings changed {stage}"));
    }
    Ok(())
}

fn cleanup_owned_child(
    child: &mut Child,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let pid = child.id();
    let kill = child.kill();
    let wait = child.wait();
    let mut reader_error = None;
    for reader in readers {
        if reader.join().is_err() {
            reader_error = Some("Godot output reader panicked");
        }
    }
    println!(
        "SETTINGS OWNED CLEANUP pid={pid} kill={kill:?} wait={wait:?} readers={reader_error:?}"
    );
    kill.map_err(|error| format!("kill owned child {pid}: {error}"))?;
    wait.map_err(|error| format!("reap owned child {pid}: {error}"))?;
    if let Some(error) = reader_error {
        return Err(error.into());
    }
    Ok(())
}

fn cleanup_pass_result<T>(
    result: Result<T, String>,
    child: &mut Child,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<T, String> {
    let cleanup = cleanup_owned_child(child, readers);
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
    context: FixtureContext<'_>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, receive);
    let mut session = Session::new();
    let first_pid = child.id();
    let save_result = run_pass(app, child, &lines, &mut session, Pass::Save)
        .and_then(|()| read_canonical(context.config));
    let saved = cleanup_pass_result(save_result, child, readers)?;
    fs::write(
        context
            .config
            .home
            .join("world-of-osso/settings-reload-phase"),
        "load",
    )
    .map_err(|error| format!("write settings reload load phase: {error}"))?;
    let (second, lines, readers) = launch_godot(
        context.root,
        context.project,
        context.config,
        context.address,
        StartupScreen::SettingsReload,
        false,
    );
    *child = second;
    let second_pid = child.id();
    let start_check = if first_pid == second_pid {
        Err(format!("settings restart reused child PID {first_pid}"))
    } else {
        require_saved_bytes(context.config, &saved, "after second child start")
    };
    if let Err(error) = start_check {
        let cleanup = cleanup_owned_child(child, readers);
        return Err(format!("{error}; owned cleanup={cleanup:?}"));
    }
    let load_result = run_pass(app, child, &lines, &mut session, Pass::Load)
        .and_then(|()| require_saved_bytes(context.config, &saved, "after loaded verification"));
    if load_result.is_ok() {
        println!(
            "PASS: SETTINGS_RELOAD functional saved/reloaded canonical bytes={} pid1={first_pid} pid2={second_pid} LootUnit=1 auto=true LootSlotRequest=0 LootRelease=0; awaiting deliberate owned cleanup",
            saved.len()
        );
    }
    cleanup_pass_result(load_result, child, readers)
}
