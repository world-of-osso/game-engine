//! Dev-tool IPC through the public CLI against the live native client: `status
//! network|sound|terrain`, `map position`, `hover`, `camera set`, `export-scene` and
//! `movement forward|stop`. The parent calls the real `game-engine-cli` (GAME_ENGINE_CLI) on the
//! client's own-PID socket and checks each answer against the fixture server's state,
//! the decoded `PlayerInput`s and the markers the observing GDScript prints.
use std::f32::consts::FRAC_PI_2;

use super::*;

const MARKER_WAIT: Duration = Duration::from_secs(60);
const CLI_DEADLINE: Duration = Duration::from_secs(8);

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
        ],
    )?;
    match field(&terrain, "process_rss_kb")?.parse::<u64>() {
        Ok(kb) if kb > 0 => Ok(()),
        _ => Err(format!("status terrain has no process RSS:\n{terrain}")),
    }
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
    check_hover(&mut run)?;
    check_camera(&mut run)?;
    check_export(&mut run)?;
    check_timed_forward(&mut run)?;
    check_stopped_forward(&mut run)?;
    finish(&mut run, readers)?;
    println!(
        "PASS: public CLI status network/sound/terrain, map position, hover, camera set, export-scene and scripted movement forward/stop drove the live native client"
    );
    Ok(())
}
