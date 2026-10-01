//! Dev-tool IPC through the public CLI against the live native client: `status
//! network|sound|terrain`, `map position`, `hover`, `camera set`, `export-scene` and
//! `movement forward|stop`. The parent calls the real `game-engine-cli` (GAME_ENGINE_CLI) on the
//! client's own-PID socket and checks each answer against the fixture server's state,
//! the decoded `PlayerInput`s and the markers the observing GDScript prints.
use std::f32::consts::FRAC_PI_2;

use shared::{
    components::PresenceStatus,
    protocol::{
        AcceptTrade, BagSlotItem, CancelTrade, CombatChannel, CombatEvent, CombatEventType,
        EmoteIntent, EmoteKind, GroupInviteIntent, GroupUninviteIntent, InitiateTrade, ItemStack,
        QuestChannel, QuestEntrySnapshot, QuestLogSnapshot, QuestObjectiveKind,
        QuestObjectiveSnapshot, QuestRepeatability, StopSpellCast, TradeChannel,
        TradePartySnapshot, TradePhase, TradeSnapshot, TradeStateUpdate,
    },
};

use super::*;

/// Group, emote and stop-cast messages the client sent, as the server decoded them.
#[derive(Resource, Default)]
struct Requests {
    invites: Vec<String>,
    uninvites: Vec<String>,
    emotes: Vec<EmoteKind>,
    stops: usize,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(Update, (receive, answer_trades));
}

fn trade_party(name: &str) -> TradePartySnapshot {
    TradePartySnapshot {
        name: name.into(),
        accepted: false,
        gold: 0,
        slots: vec![None; 7],
    }
}

/// A scripted trade server: an initiate opens a pending outgoing request, a cancel
/// ends it, an accept without a trade is refused.
fn answer_trades(
    mut initiates: Query<&mut MessageReceiver<InitiateTrade>>,
    mut cancels: Query<&mut MessageReceiver<CancelTrade>>,
    mut accepts: Query<&mut MessageReceiver<AcceptTrade>>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let mut updates = Vec::new();
    for mut receiver in &mut initiates {
        updates.extend(receiver.receive().map(|initiate| TradeStateUpdate {
            trade: Some(TradeSnapshot {
                phase: TradePhase::PendingOutgoing,
                player: trade_party(NAME),
                other: trade_party(&initiate.target_name),
            }),
            message: None,
            error: None,
        }));
    }
    for mut receiver in &mut cancels {
        updates.extend(receiver.receive().map(|_| TradeStateUpdate {
            trade: None,
            message: Some("Trade cancelled.".into()),
            error: None,
        }));
    }
    for mut receiver in &mut accepts {
        updates.extend(receiver.receive().map(|_| TradeStateUpdate {
            trade: None,
            message: None,
            error: Some("You are not trading.".into()),
        }));
    }
    for update in updates {
        for mut sender in &mut senders {
            sender.send::<TradeChannel>(update.clone());
        }
    }
}

fn receive(
    mut invites: Query<&mut MessageReceiver<GroupInviteIntent>>,
    mut uninvites: Query<&mut MessageReceiver<GroupUninviteIntent>>,
    mut emotes: Query<&mut MessageReceiver<EmoteIntent>>,
    mut stops: Query<&mut MessageReceiver<StopSpellCast>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut invites {
        requests
            .invites
            .extend(receiver.receive().map(|intent| intent.name));
    }
    for mut receiver in &mut uninvites {
        requests
            .uninvites
            .extend(receiver.receive().map(|intent| intent.name));
    }
    for mut receiver in &mut emotes {
        requests
            .emotes
            .extend(receiver.receive().map(|intent| intent.emote));
    }
    for mut receiver in &mut stops {
        requests.stops += receiver.receive().count();
    }
}

const MARKER_WAIT: Duration = Duration::from_secs(60);
// The original engine sets no response deadline; this bounds a hung client. Shared
// agent hosts (load average 30+) stall a busy client frame for several seconds.
const CLI_DEADLINE: Duration = Duration::from_secs(30);

struct Run<'a> {
    app: &'a mut App,
    child: &'a mut Child,
    lines: Receiver<String>,
    cli: PathBuf,
    socket: PathBuf,
    artifacts: PathBuf,
    markers: Vec<String>,
    calls: usize,
}

impl Run<'_> {
    /// One server step plus the Godot output since the last step.
    fn pump(&mut self) -> Result<(), String> {
        self.app.update();
        for line in self.lines.try_iter() {
            let line = line.trim().trim_start_matches("GODOT_STDERR: ");
            if line.starts_with("SCRIPT ERROR") || line.contains("res://tests/world_dev_ipc") {
                return Err(format!("Godot dev IPC flow: {line}"));
            }
            if let Some(marker) = line.strip_prefix("FIXTURE DEV_IPC_") {
                self.markers.push(marker.to_owned());
            }
        }
        if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
            if !self.markers.iter().any(|marker| marker == "DONE") {
                return Err(format!(
                    "Godot exited {status} before DONE: {:?}",
                    self.markers
                ));
            }
        }
        Ok(())
    }

    fn wait_marker(&mut self, prefix: &str) -> Result<String, String> {
        let deadline = Instant::now() + MARKER_WAIT;
        while Instant::now() < deadline {
            self.pump()?;
            if let Some(index) = self
                .markers
                .iter()
                .position(|marker| marker.starts_with(prefix))
            {
                return Ok(self.markers.remove(index));
            }
            thread::sleep(TICK);
        }
        Err(format!(
            "timed out waiting for DEV_IPC_{prefix}: {:?}",
            self.markers
        ))
    }

    fn pump_for(&mut self, duration: Duration) -> Result<(), String> {
        let until = Instant::now() + duration;
        while Instant::now() < until {
            self.pump()?;
            thread::sleep(TICK);
        }
        Ok(())
    }

    /// The real CLI's stdout, or its stderr when it exits unsuccessfully. The server keeps
    /// stepping while the client answers.
    fn cli(&mut self, args: &[&str]) -> Result<Result<String, String>, String> {
        self.calls += 1;
        let name = format!(
            "{:02}-{}",
            self.calls,
            args.join("_").replace(['/', ' '], "-")
        );
        let stdout = self.artifacts.join(format!("{name}.stdout"));
        let stderr = self.artifacts.join(format!("{name}.stderr"));
        let mut cli = Command::new(&self.cli)
            .arg("--socket")
            .arg(&self.socket)
            .args(args)
            .stdout(Stdio::from(
                fs::File::create(&stdout).map_err(|e| e.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(&stderr).map_err(|e| e.to_string())?,
            ))
            .spawn()
            .map_err(|error| format!("SETUP: launch CLI: {error}"))?;
        let deadline = Instant::now() + CLI_DEADLINE;
        let status = loop {
            if let Some(status) = cli.try_wait().map_err(|error| error.to_string())? {
                break status;
            }
            if Instant::now() >= deadline {
                cli.kill().map_err(|error| error.to_string())?;
                cli.wait().map_err(|error| error.to_string())?;
                return Err(format!("CLI {args:?} exceeded its request deadline"));
            }
            self.pump()?;
            thread::sleep(TICK);
        };
        let out = fs::read_to_string(&stdout).map_err(|error| error.to_string())?;
        let err = fs::read_to_string(&stderr).map_err(|error| error.to_string())?;
        println!("CLI {args:?} -> {status}: {}{}", out.trim(), err.trim());
        Ok(if status.success() { Ok(out) } else { Err(err) })
    }

    fn expect_text(&mut self, args: &[&str]) -> Result<String, String> {
        self.cli(args)?
            .map_err(|error| format!("CLI {args:?} failed: {}", error.trim()))
    }
}

fn expect_lines(what: &str, text: &str, expected: &[String]) -> Result<(), String> {
    let lines: Vec<&str> = text.lines().collect();
    for line in expected {
        if !lines.contains(&line.as_str()) {
            return Err(format!("{what} lacks `{line}`:\n{text}"));
        }
    }
    Ok(())
}

fn field<'t>(text: &'t str, key: &str) -> Result<&'t str, String> {
    text.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
        .ok_or_else(|| format!("no `{key}` in:\n{text}"))
}

fn position(text: &str) -> Result<(f32, f32), String> {
    let value = field(text, "position")?;
    let (x, z) = value.split_once(',').ok_or("position is not x,z")?;
    let parse = |axis: &str| axis.parse::<f32>().map_err(|error| error.to_string());
    Ok((parse(x)?, parse(z)?))
}

fn check_network(run: &mut Run, address: SocketAddr) -> Result<(), String> {
    let network = run.expect_text(&["status", "network"])?;
    // The fixture replicates the selected player, the remote player and the vendor.
    expect_lines(
        "status network",
        &network,
        &[
            format!("server_addr: {address}"),
            "game_state: InWorld".into(),
            "connected: true".into(),
            "connected_links: 1".into(),
            "zone_id: 12".into(),
            "remote_entities: 3".into(),
            "local_players: 1".into(),
        ],
    )?;
    match field(&network, "local_client_id")?.parse::<u64>() {
        Ok(_) => Ok(()),
        Err(_) => Err(format!(
            "status network has no netcode client id:\n{network}"
        )),
    }
}

fn check_sound(run: &mut Run) -> Result<(), String> {
    let sound = run.expect_text(&["status", "sound"])?;
    // Seeded options; zone 12 plays catalogued music 53492 and has no ambience.
    expect_lines(
        "status sound",
        &sound,
        &[
            "enabled: true".into(),
            "muted: false".into(),
            "master_volume: 1.00".into(),
            "ambient_volume: 0.30".into(),
            "ambient_entities: 0".into(),
            "active_sinks: 1".into(),
        ],
    )
}

fn check_terrain(run: &mut Run, tiles: &str) -> Result<(), String> {
    let terrain = run.expect_text(&["status", "terrain"])?;
    expect_lines(
        "status terrain",
        &terrain,
        &[
            "map_name: azeroth".into(),
            "initial_tile: 32,48".into(),
            "initial_tiles: 9".into(),
            format!("loaded_tiles: {tiles}"),
            "pending_tiles: 0".into(),
            "failed_tiles: 0".into(),
            "load_radius: 1".into(),
            "server_requested_tiles: 0".into(),
            format!("heightmap_tiles: {tiles}"),
        ],
    )?;
    for counter in ["m2_model_cache_entries", "terrain_material_assets"] {
        match field(&terrain, counter)?.parse::<u64>() {
            Ok(count) if count > 0 => {}
            _ => return Err(format!("status terrain has no {counter}:\n{terrain}")),
        }
    }
    match field(&terrain, "process_rss_kb")?.parse::<u64>() {
        Ok(kb) if kb > 0 => Ok(()),
        _ => Err(format!("status terrain has no process RSS:\n{terrain}")),
    }
}

/// Runs `args`, which must answer exactly `expected`.
fn expect_exact(run: &mut Run, args: &[&str], expected: &str) -> Result<(), String> {
    let text = run.expect_text(args)?;
    if text.trim_end() != expected {
        return Err(format!(
            "CLI {args:?} answered\n{text}\nexpected\n{expected}"
        ));
    }
    Ok(())
}

/// Steps the server until `seen` holds, or fails after a few seconds.
fn wait_server(run: &mut Run, what: &str, seen: impl Fn(&Requests) -> bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !seen(run.app.world().resource::<Requests>()) {
        if Instant::now() >= deadline {
            return Err(format!("the server never decoded {what}"));
        }
        run.pump()?;
        thread::sleep(TICK);
    }
    Ok(())
}

/// `map target` without a target and `map waypoint add|clear` around the spawn.
fn check_map_requests(run: &mut Run) -> Result<(), String> {
    expect_exact(run, &["map", "target"], "map_target: none\ndistance: -")?;
    expect_exact(
        run,
        &["map", "waypoint", "add", "--x=-8940.5", "--y=12.25"],
        "zone_id: 12\nposition: -8949.00,0.00\nwaypoint: -8940.50,12.25\ngraveyard_marker: -",
    )?;
    expect_exact(
        run,
        &["map", "waypoint", "clear"],
        "zone_id: 12\nposition: -8949.00,0.00\nwaypoint: -\ngraveyard_marker: -",
    )
}

/// Group, emote and spell requests reach the server as the original intents.
fn check_social_and_combat(run: &mut Run) -> Result<(), String> {
    expect_exact(run, &["group", "roster"], "group_roster: 0\n-")?;
    expect_exact(
        run,
        &["group", "status"],
        "in_group: false\nis_raid: false\nmembers: 0\nready_check: -\npending_invite: -\nlast_message: -",
    )?;
    expect_exact(
        run,
        &["group", "invite", "--name", "Bob"],
        "group invite submitted for Bob",
    )?;
    wait_server(run, "GroupInviteIntent Bob", |r| r.invites == ["Bob"])?;
    expect_exact(
        run,
        &["group", "uninvite", "--name", "Bob"],
        "group uninvite submitted for Bob",
    )?;
    wait_server(run, "GroupUninviteIntent Bob", |r| r.uninvites == ["Bob"])?;
    expect_exact(run, &["emote", "wave"], "emote submitted Wave")?;
    wait_server(run, "EmoteIntent Wave", |r| r.emotes == [EmoteKind::Wave])?;
    match run.cli(&["spell", "cast", "--spell", "1464"])? {
        Err(error) if error.contains("no current target selected") => {}
        other => return Err(format!("untargeted spell cast answered {other:?}")),
    }
    let casts_before = run.app.world().resource::<Incoming>().casts.len();
    expect_exact(
        run,
        &["spell", "cast", "--spell", "1464", "--target", "none"],
        "spell cast submitted spell=1464 target=-",
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let casts = &run.app.world().resource::<Incoming>().casts;
        if let Some(cast) = casts.get(casts_before) {
            if cast.spell_id != Some(1464) || cast.target_entity.is_some() {
                return Err(format!("spell cast decoded as {cast:?}"));
            }
            break;
        }
        if Instant::now() >= deadline {
            return Err("the server never decoded the spell cast".into());
        }
        run.pump()?;
        thread::sleep(TICK);
    }
    expect_exact(run, &["spell", "stop"], "spell stop submitted")?;
    wait_server(run, "StopSpellCast", |r| r.stops == 1)
}

/// Bags (a Melted Candle stack and a Linen Cloth stack), one watched quest and Away
/// presence, as the server sends them.
fn seed_items_and_quests(app: &mut App) {
    let stack = |item_guid, item_id| ItemStack {
        item_guid,
        item_id,
        count: 3,
        durability: None,
        soulbound: false,
    };
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![BagContents {
                bag: 0,
                size: 16,
                items: vec![
                    BagSlotItem {
                        slot: 0,
                        item: stack(755_001, 755),
                    },
                    BagSlotItem {
                        slot: 1,
                        item: stack(9_180_001, 2589),
                    },
                ],
            }],
        },
    );
    send::<_, QuestChannel>(
        app,
        QuestLogSnapshot {
            entries: vec![QuestEntrySnapshot {
                quest_id: 7,
                title: "Fixture Quest".into(),
                zone: "Elwynn Forest".into(),
                completed: false,
                repeatability: QuestRepeatability::Normal,
                objectives: vec![QuestObjectiveSnapshot {
                    text: "Kobold Vermin slain".into(),
                    current: 3,
                    required: 10,
                    completed: false,
                    kind: QuestObjectiveKind::Monster,
                    object_id: 6,
                }],
                level: 1,
                sort_id: 12,
                objectives_text: String::new(),
                completion_text: String::new(),
                watched: true,
                pois: Vec::new(),
            }],
            watched_quest_ids: vec![7],
        },
    );
    let mut players = app.world_mut().query::<(Entity, &Player)>();
    let local = players
        .iter(app.world())
        .find(|(_, player)| player.name == NAME)
        .map(|(entity, _)| entity)
        .expect("the selected player is spawned");
    app.world_mut()
        .entity_mut(local)
        .insert(PresenceStatus::Afk);
}

/// Runs `args` until it answers exactly `expected`; the server's messages may still be
/// on their way.
fn expect_eventually(run: &mut Run, args: &[&str], expected: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = run.expect_text(args)?;
        if text.trim_end() == expected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "CLI {args:?} answered\n{text}\nexpected\n{expected}"
            ));
        }
        run.pump_for(Duration::from_millis(200))?;
    }
}

/// Quest log, bags, stored items, item info and presence from the seeded state.
fn check_items_and_quests(run: &mut Run) -> Result<(), String> {
    seed_items_and_quests(run.app);
    expect_eventually(
        run,
        &["quest", "list"],
        "quests: 1\n7 Fixture Quest zone=Elwynn Forest repeat=normal completed=false objectives=1",
    )?;
    expect_exact(
        run,
        &["quest", "watch"],
        "quest_watch: 1\n7 Fixture Quest [Kobold Vermin slain 3/10]",
    )?;
    expect_exact(
        run,
        &["quest", "show", "--id", "7"],
        "quest_id: 7\ntitle: Fixture Quest\nzone: Elwynn Forest\nrepeatability: normal\ncompleted: false\nobjectives:\nKobold Vermin slain 3/10 completed=false",
    )?;
    expect_exact(run, &["quest", "show", "--id", "99"], "quest 99: not found")?;
    // ItemSparse 12.1.0.69933: Melted Candle is Poor (0), Linen Cloth Common (1); both
    // require level 0.
    expect_eventually(
        run,
        &["status", "bags"],
        "bags: 2\ngold: 1250\n0 755001 Melted Candle x3 q0 lvl0\n1 9180001 Linen Cloth x3 q1 lvl0",
    )?;
    expect_exact(
        run,
        &["inventory", "list"],
        "inventory: 2\nbags:0 755001 755 Melted Candle x3\nbags:1 9180001 2589 Linen Cloth x3",
    )?;
    expect_exact(
        run,
        &["inventory", "search", "--text", "LINEN"],
        "inventory search text=LINEN: 1\n[bags]\n1 9180001 2589 Linen Cloth x3",
    )?;
    expect_exact(
        run,
        &["inventory", "whereis", "--item-id", "755"],
        "inventory whereis item_id=755: 1\nbags:0 755001 Melted Candle x3",
    )?;
    expect_exact(run, &["status", "guild-vault"], "guild_vault: 0\n-")?;
    expect_exact(run, &["status", "warbank"], "warbank: 0\n-")?;
    // ItemSparse 2589: item level 10, sells for 13, stacks to 1000, no binding.
    expect_exact(
        run,
        &["item", "info", "--item-id", "2589"],
        "item_id: 2589\nname: Linen Cloth\nquality: 1\nitem_level: 10\nrequired_level: 0\ninventory_type: 0\nsell_price: 13\nstackable: 1000\nbonding: 0\nexpansion_id: 0\nappearance_known: true",
    )?;
    expect_eventually(run, &["presence", "status"], "presence: afk")
}

/// `quest interact` right-clicks the vendor (interact) and targets the remote player;
/// `map target` follows the target.
fn check_interact(run: &mut Run) -> Result<(), String> {
    let vendor = run
        .app
        .world()
        .resource::<Incoming>()
        .vendor
        .ok_or("vendor missing")?;
    let interactions = run.app.world().resource::<Incoming>().interactions.len();
    expect_exact(
        run,
        &["quest", "interact", "--npc", "fixture vendor"],
        "interact Fixture Vendor",
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while run.app.world().resource::<Incoming>().interactions.len() == interactions {
        if Instant::now() >= deadline {
            return Err("the server never decoded InteractNpc".into());
        }
        run.pump()?;
        thread::sleep(TICK);
    }
    let npc = run.app.world().resource::<Incoming>().interactions[interactions].npc;
    if npc != vendor.to_bits() {
        return Err(format!(
            "InteractNpc {npc} is not the vendor {}",
            vendor.to_bits()
        ));
    }
    // The vendor stands 2 yd west and 3 yd south of the player: sqrt(13) yd away.
    expect_exact(
        run,
        &["map", "target"],
        &format!(
            "map_target: Fixture Vendor\nentity: {}\nposition: -8951.00,3.00\ndistance: 3.61",
            vendor.to_bits()
        ),
    )?;
    expect_exact(
        run,
        &["quest", "interact", "--npc", "remote fixture"],
        "target Remote Fixture",
    )?;
    let target = run.expect_text(&["map", "target"])?;
    expect_lines(
        "map target",
        &target,
        &[
            "map_target: Remote Fixture".into(),
            "position: -8946.00,0.00".into(),
            "distance: 3.00".into(),
        ],
    )?;
    match run.cli(&["quest", "interact", "--npc", "Nobody"])? {
        Err(error) if error.contains("no NPC, game object or player named Nobody") => Ok(()),
        other => Err(format!("interact with nobody answered {other:?}")),
    }
}

/// Trade actions answer with the server's next update: an opened request's status, a
/// cancel's message, a refusal's error; `trade status` keeps the last error.
fn check_trade(run: &mut Run) -> Result<(), String> {
    expect_exact(run, &["trade", "status"], "trade: inactive")?;
    expect_exact(
        run,
        &["trade", "initiate", "--name", "Remote Fixture"],
        "trade: pending-outgoing\nyou: Input Fixture copper=0 accepted=no items=none\nother: Remote Fixture copper=0 accepted=no items=none",
    )?;
    expect_exact(run, &["trade", "cancel"], "Trade cancelled.")?;
    match run.cli(&["trade", "accept"])? {
        Err(error) if error.contains("You are not trading.") => {}
        other => return Err(format!("trade accept without a trade answered {other:?}")),
    }
    expect_exact(
        run,
        &["trade", "status"],
        "trade: inactive\nerror: You are not trading.",
    )
}

/// A melee hit the server reports: `combat log` and `combat recap` by the attacker.
fn check_combat_log(run: &mut Run) -> Result<(), String> {
    expect_exact(run, &["combat", "log"], "combat_log: 0\n-")?;
    let vendor = run
        .app
        .world()
        .resource::<Incoming>()
        .vendor
        .ok_or("vendor missing")?
        .to_bits();
    let mut players = run.app.world_mut().query::<(Entity, &Player)>();
    let player = players
        .iter(run.app.world())
        .find(|(_, player)| player.name == NAME)
        .map(|(entity, _)| entity.to_bits())
        .ok_or("the selected player is not spawned")?;
    send::<_, CombatChannel>(
        run.app,
        CombatEvent {
            attacker: vendor,
            target: player,
            amount: 12.4,
            spell_id: 0,
            event_type: CombatEventType::MeleeDamage,
        },
    );
    let line = format!(
        "damage src={vendor} dst={player} spell=- amount=12 aura=- text={vendor} hit {player} for 12"
    );
    expect_eventually(
        run,
        &["combat", "log", "--lines", "5"],
        &format!("combat_log: 1\n{line}"),
    )?;
    expect_exact(
        run,
        &["combat", "recap", "--target", &vendor.to_string()],
        &format!("combat_recap target={vendor}: 1\n{line}"),
    )?;
    expect_exact(
        run,
        &["combat", "recap", "--target", "nobody"],
        "combat_recap target=nobody: 0\n-",
    )
}

fn check_spawn_position(run: &mut Run) -> Result<(), String> {
    let map = run.expect_text(&["map", "position"])?;
    expect_lines(
        "map position",
        &map,
        &[
            "zone_id: 12".into(),
            "position: -8949.00,0.00".into(),
            "waypoint: -".into(),
            "graveyard_marker: -".into(),
        ],
    )
}

fn check_hover(run: &mut Run) -> Result<(), String> {
    let npc = run.expect_text(&["hover", "--npc", "fixture vendor"])?;
    if !npc.trim().starts_with("cursor at (") {
        return Err(format!("hover --npc answered {npc}"));
    }
    run.wait_marker("HOVER_NPC Fixture Vendor")?;
    let point = run.expect_text(&["hover", "--x", "640", "--y", "40"])?;
    if point.trim() != "cursor at (640, 40)" {
        return Err(format!("hover --x 640 --y 40 answered {point}"));
    }
    run.wait_marker("HOVER_POINT")?;
    match run.cli(&["hover", "--npc", "Nobody"])? {
        Err(error) if error.contains("no NPC named Nobody in view") => Ok(()),
        other => Err(format!("hover of an absent NPC answered {other:?}")),
    }
}

fn check_camera(run: &mut Run) -> Result<(), String> {
    let set = run.expect_text(&[
        "camera",
        "set",
        "--yaw-degrees",
        "90",
        "--pitch-degrees",
        "-20",
    ])?;
    if set.trim() != "camera yaw=90.000 pitch=-20.000 degrees" {
        return Err(format!("camera set answered {set}"));
    }
    run.wait_marker("CAMERA")?;
    match run.cli(&["camera", "set", "--pitch-degrees", "120"])? {
        Err(error) if error.contains("camera pitch must be between") => Ok(()),
        other => Err(format!("out-of-range pitch answered {other:?}")),
    }
}

/// `export-scene` answers with the original reply; the observing script checks the
/// written snapshot against the live scene.
fn check_export(run: &mut Run) -> Result<(), String> {
    let path = run.artifacts.join("scene-export.json");
    let path = path.to_str().ok_or("non-UTF-8 artifact path")?.to_owned();
    let reply = run.expect_text(&["export-scene", &path])?;
    if reply.trim() != format!("scene exported to {path}") {
        return Err(format!("export-scene answered {reply}"));
    }
    run.wait_marker("EXPORT")?;
    Ok(())
}

/// Moving inputs since the last call must head east (yaw 90 degrees) at run speed.
fn eastward_inputs(app: &mut App) -> Result<usize, String> {
    let inputs = take_inputs(app);
    for input in &inputs {
        let [x, y, z] = input.direction;
        if (x - 1.0).abs() > 0.01 || y.abs() > 0.01 || z.abs() > 0.01 {
            return Err(format!(
                "scripted movement direction is not east: {input:?}"
            ));
        }
        if (input.facing_yaw - FRAC_PI_2).abs() > 1e-3 || !input.running || input.jumping {
            return Err(format!("scripted movement facing/run state: {input:?}"));
        }
    }
    Ok(inputs.len())
}

fn start_forward(run: &mut Run, seconds: &str) -> Result<(), String> {
    let started = run.expect_text(&[
        "movement",
        "forward",
        "--seconds",
        seconds,
        "--yaw-degrees",
        "90",
    ])?;
    if started.trim() != "scripted movement started" {
        return Err(format!("movement forward answered {started}"));
    }
    Ok(())
}

fn stops(run: &Run) -> u32 {
    run.app.world().resource::<Incoming>().stops
}

/// `movement forward --seconds 1 --yaw-degrees 90` runs east for one second and stops.
fn check_timed_forward(run: &mut Run) -> Result<(), String> {
    take_inputs(run.app);
    let before = stops(run);
    match run.cli(&["movement", "forward", "--seconds", "0"])? {
        Err(error) if error.contains("duration must be finite, positive") => {}
        other => return Err(format!("zero-second movement answered {other:?}")),
    }
    start_forward(run, "1")?;
    let mut moving = 0;
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        run.pump()?;
        moving += eastward_inputs(run.app)?;
        thread::sleep(TICK);
    }
    if moving == 0 || stops(run) != before + 1 {
        return Err(format!(
            "1 s forward: {moving} moving inputs, {} stops",
            stops(run) - before
        ));
    }
    ensure_release_reported(run.app, "scripted 1 s forward")?;
    let map = run.expect_text(&["map", "position"])?;
    let (x, z) = position(&map)?;
    // RUN_SPEED 7 yd/s for one second, from (-8949, 0), heading +X.
    if !(-8943.0..=-8941.5).contains(&x) || z.abs() > 0.2 {
        return Err(format!("1 s at yaw 90 ended at {x},{z}:\n{map}"));
    }
    // The zone stays the last one found, even on the edge of a tile not loaded.
    expect_lines("map position after moving", &map, &["zone_id: 12".into()])?;
    Ok(())
}

/// `movement stop` ends a 30 s scripted run with one stop input.
fn check_stopped_forward(run: &mut Run) -> Result<(), String> {
    let before = stops(run);
    start_forward(run, "30")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while eastward_inputs(run.app)? == 0 {
        if Instant::now() >= deadline {
            return Err("30 s forward sent no moving input".into());
        }
        run.pump()?;
        thread::sleep(TICK);
    }
    let halted = run.expect_text(&["movement", "stop"])?;
    if halted.trim() != "scripted movement stopped" {
        return Err(format!("movement stop answered {halted}"));
    }
    run.pump_for(RELEASE_DRAIN)?;
    take_inputs(run.app);
    run.pump_for(RELEASE_QUIET)?;
    if !take_inputs(run.app).is_empty() || stops(run) != before + 1 {
        return Err(format!(
            "movement stop: moving inputs continued or {} stops",
            stops(run) - before
        ));
    }
    ensure_release_reported(run.app, "movement stop")
}

/// Answers login and selection, sends the terrain on Loading, and returns the READY
/// marker's loaded tile count.
fn wait_ready(run: &mut Run) -> Result<String, String> {
    let mut selected = None;
    let mut remote = None;
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        run.pump()?;
        respond_to_login(run.app, StartupScreen::DevIpc)?;
        respond_to_selection(run.app, StartupScreen::DevIpc, &mut selected, &mut remote)?;
        if let Some(index) = run.markers.iter().position(|marker| marker == "LOADING") {
            run.markers.remove(index);
            selected.ok_or("Loading before character selection")?;
            send::<_, TerrainChannel>(
                run.app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
        }
        if let Some(index) = run.markers.iter().position(|m| m.starts_with("READY")) {
            let ready = run.markers.remove(index);
            return ready
                .strip_prefix("READY tiles=")
                .map(str::to_owned)
                .ok_or_else(|| format!("malformed {ready}"));
        }
        thread::sleep(TICK);
    }
    Err(format!("timed out awaiting world: {:?}", run.markers))
}

/// Removing the vendor ends the observing script, which exits normally.
fn finish(run: &mut Run, readers: Vec<thread::JoinHandle<()>>) -> Result<(), String> {
    let vendor = run
        .app
        .world()
        .resource::<Incoming>()
        .vendor
        .ok_or("vendor missing")?;
    run.app.world_mut().despawn(vendor);
    run.wait_marker("DONE")?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = run.child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            return Err("Godot did not exit after DONE".into());
        }
        run.app.update();
        thread::sleep(TICK);
    };
    for reader in readers {
        reader.join().map_err(|_| "Godot output reader panicked")?;
    }
    if !status.success() {
        return Err(format!("Godot exited {status}"));
    }
    Ok(())
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
    root: &Path,
    address: SocketAddr,
) -> Result<(), String> {
    let cli = std::env::var_os("GAME_ENGINE_CLI")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .ok_or("SETUP: GAME_ENGINE_CLI must name the existing game-engine-cli")?;
    let artifacts = root.join(format!("data/diagnostics/dev-ipc-{}", child.id()));
    fs::create_dir_all(&artifacts).map_err(|error| error.to_string())?;
    let mut run = Run {
        socket: PathBuf::from(format!("/tmp/game-engine-{}.sock", child.id())),
        app,
        child,
        lines,
        cli,
        artifacts,
        markers: Vec::new(),
        calls: 0,
    };
    let tiles = wait_ready(&mut run)?;
    println!(
        "READY native PID={} socket={}",
        run.child.id(),
        run.socket.display()
    );
    check_network(&mut run, address)?;
    check_sound(&mut run)?;
    check_terrain(&mut run, &tiles)?;
    check_spawn_position(&mut run)?;
    check_map_requests(&mut run)?;
    check_social_and_combat(&mut run)?;
    check_items_and_quests(&mut run)?;
    check_interact(&mut run)?;
    check_trade(&mut run)?;
    check_combat_log(&mut run)?;
    check_hover(&mut run)?;
    check_camera(&mut run)?;
    check_export(&mut run)?;
    check_timed_forward(&mut run)?;
    check_stopped_forward(&mut run)?;
    finish(&mut run, readers)?;
    println!(
        "PASS: public CLI status network/sound/terrain, map position/target/waypoint, group, emote, spell cast/stop, quests, bags, inventory, storage, item info, presence, quest interact, trade, combat log/recap, hover, camera set, export-scene and scripted movement forward/stop drove the live native client"
    );
    Ok(())
}
