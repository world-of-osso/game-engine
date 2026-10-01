//! Unchanged debug/login.js through root startup, authored Login and real loopback auth.
//! Optional positional mode: timeout-continuation (missing-frame deadline then live dump).
//! MAIN builds/runs this fixture; requires an existing root launcher and GODOT_BIN.
//! No external server. Passwords are masked before writing persistent diagnostics.

#[path = "fixture_support/mod.rs"]
mod fixture_support;

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use lightyear::prelude::{self as network, MessageReceiver, MessageSender, server};
use shared::protocol::{AuthChannel, CharacterListEntry, LoginRequest, LoginResponse};

const USERNAME: &str = "native-js-owned-user";
const PASSWORD: &str = "native-js-owned-secret";
const CHARACTER: &str = "Automation Fixture";
const TIMEOUT: Duration = Duration::from_secs(180);
const TIMEOUT_SCRIPT: &str = "ui.waitForFrame(\"NativeJsMissingFrame\", 0.05); ui.dumpUiTree();\n";
const FRAME_TIMEOUT: &str =
    "native JS automation: timed out waiting for frame 'NativeJsMissingFrame' after 0.05s";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Login,
    TimeoutContinuation,
}

impl Mode {
    fn from_args() -> Result<Self, String> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        match args.as_slice() {
            [] => Ok(Self::Login),
            [mode] if mode == "timeout-continuation" => Ok(Self::TimeoutContinuation),
            _ => Err("SETUP: usage: native_js_automation_fixture [timeout-continuation]".into()),
        }
    }

    fn observer_mode(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::TimeoutContinuation => "timeout-continuation",
        }
    }
}

#[derive(Resource, Default)]
struct Incoming(Vec<(Entity, LoginRequest)>);

fn receive_logins(
    mut receivers: Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
    mut incoming: ResMut<Incoming>,
) {
    for (entity, mut receiver) in &mut receivers {
        incoming
            .0
            .extend(receiver.receive().map(|request| (entity, request)));
    }
}

fn start_server() -> Result<(App, SocketAddr), String> {
    // Same loopback reservation boundary as the existing native input/auth fixtures.
    let reservation = UdpSocket::bind("127.0.0.1:0")
        .map_err(|error| format!("SETUP: reserve loopback UDP: {error}"))?;
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
    app.add_systems(Update, receive_logins);
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

fn authenticate(app: &mut App, count: &mut usize, mode: Mode) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().0);
    for (link, request) in requests {
        if mode == Mode::TimeoutContinuation {
            *count += 1;
            return Err("FEATURE: timeout-continuation decoded unexpected LoginRequest; expected zero auth (password masked)".into());
        }
        if request.username != USERNAME || request.password != PASSWORD || request.token.is_some() {
            return Err("FEATURE: decoded LoginRequest did not contain exact owned credentials and no token (password masked)".into());
        }
        *count += 1;
        if *count != 1 {
            return Err("FEATURE: unchanged script issued duplicate LoginRequest".into());
        }
        let response = LoginResponse {
            success: true,
            token: "native-js-owned-token".into(),
            characters: vec![CharacterListEntry {
                character_id: 17,
                name: CHARACTER.into(),
                level: 10,
                race: 1,
                class: 2,
                appearance: Default::default(),
                equipment_appearance: Default::default(),
            }],
            error: None,
        };
        app.world_mut()
            .get_mut::<MessageSender<LoginResponse>>(link)
            .ok_or("SETUP: protocol fixture missing LoginResponse sender")?
            .send::<AuthChannel>(response);
        println!("AUTH: exact decoded owned credentials; password=***; roster=1");
    }
    Ok(())
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("Own child status: {error}"),
        }
        // Own PID only. Forced cleanup is not shutdown or leak-freedom proof.
        if let Err(error) = self.0.kill() {
            eprintln!("Own child forced cleanup: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("Own child reap: {error}");
        }
    }
}

struct Output {
    line: String,
    password_leaked: bool,
    stdout: bool,
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
                    password_leaked: line.contains(PASSWORD),
                    line: line.replace(PASSWORD, "***"),
                    stdout,
                })
                .map_err(|error| format!("SETUP: read child diagnostics: {error}"));
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
    address: SocketAddr,
    artifacts: &Path,
    mode: Mode,
) -> Result<(OwnedChild, Receiver<Result<Output, String>>), String> {
    let launcher = require_file(
        root.join("target/debug/game-engine-launcher"),
        "root launcher",
    )?;
    let godot = require_file(
        std::env::var_os("GODOT_BIN")
            .map(PathBuf::from)
            .ok_or("SETUP: GODOT_BIN must select an existing executable")?,
        "Godot executable",
    )?;
    let script = match mode {
        Mode::Login => require_file(root.join("debug/login.js"), "unchanged login JS")?,
        Mode::TimeoutContinuation => {
            let path = artifacts.join("timeout-continuation.js");
            fs::write(&path, TIMEOUT_SCRIPT)
                .map_err(|error| format!("SETUP: write timeout-continuation script: {error}"))?;
            path
        }
    };
    require_file(
        root.join("godot/.godot/extension_list.cfg"),
        "imported native extension cache",
    )?;
    for directory in ["config", "user-data"] {
        fs::create_dir_all(artifacts.join(directory))
            .map_err(|error| format!("SETUP: artifacts: {error}"))?;
    }
    let mut child = OwnedChild(
        Command::new(launcher)
            .current_dir(root)
            .args(["--headless", "--audio-driver", "Dummy", "--path"])
            .arg(root.join("godot"))
            .args([
                "--script",
                "res://tests/native_js_automation_flow.gd",
                "--screen",
                "login",
                "--server",
            ])
            .arg(address.to_string())
            // Intentionally BEFORE the separator: exercise production root flag routing.
            .arg("--run-js-ui-script")
            .arg(script)
            .env("GODOT_BIN", godot)
            .env("GAME_ENGINE_ROOT", root)
            .env("GAME_ENGINE_SHARED_ROOT", root)
            .env("GAME_ENGINE_SHARED_DATA_DIR", root.join("data"))
            // Preserve inherited warm XDG_CACHE_HOME / ASSET_RESOLVER_CACHE_DIR.
            .env("XDG_CONFIG_HOME", artifacts.join("config"))
            .env("XDG_DATA_HOME", artifacts.join("user-data"))
            .env("LOGIN_USER", USERNAME)
            .env("LOGIN_PASS", PASSWORD)
            .env("NATIVE_JS_ARTIFACTS", artifacts)
            .env("NATIVE_JS_MODE", mode.observer_mode())
            .env("NATIVE_JS_CHARACTER", CHARACTER)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("SETUP: launch root client: {error}"))?,
    );
    let (sender, receiver) = mpsc::channel();
    collect_output(
        child.0.stdout.take().ok_or("SETUP: missing stdout pipe")?,
        sender.clone(),
        true,
    );
    collect_output(
        child.0.stderr.take().ok_or("SETUP: missing stderr pipe")?,
        sender,
        false,
    );
    Ok((child, receiver))
}

fn drain_output(
    receiver: &Receiver<Result<Output, String>>,
    log: &mut fs::File,
    lines: &mut Vec<Output>,
) -> Result<(), String> {
    for output in receiver.try_iter() {
        let output = output?;
        writeln!(log, "{}", output.line)
            .map_err(|error| format!("SETUP: write diagnostics: {error}"))?;
        if output.password_leaked {
            return Err("FEATURE: child exposed raw LOGIN_PASS; diagnostic copy masked".into());
        }
        lines.push(output);
    }
    Ok(())
}

fn assert_dump(lines: &[&str]) -> Result<(), String> {
    // Fixture never prints these frame lines or invokes a diagnostic method.
    // These must come from unchanged login.js's final production dumpUiTree action.
    for name in ["CharSelectRoot", "BackToLogin", "CharSelectCharacterName"] {
        if !lines.iter().any(|line| {
            line.contains(name) && line.contains("alpha=") && line.contains(" visible ")
        }) {
            return Err(format!(
                "FEATURE: final real stdout UI dump missing visible {name}"
            ));
        }
    }
    if !lines
        .iter()
        .any(|line| line.contains("CharSelectCharacterName") && line.contains(CHARACTER))
    {
        return Err("FEATURE: final UI dump lacks authoritative selected roster name".into());
    }
    Ok(())
}

fn assert_timeout_continuation(
    lines: &[Output],
    artifacts: &Path,
    auth_count: usize,
) -> Result<(), String> {
    if auth_count != 0
        || !artifacts.join("login-ready").is_file()
        || !artifacts.join("observed-timeout-login").is_file()
    {
        return Err(
            "FEATURE: timeout-continuation requires observed visible Login and zero decoded auth"
                .into(),
        );
    }
    if !lines
        .iter()
        .any(|output| output.line.contains(FRAME_TIMEOUT))
    {
        return Err("SETUP/UNCLASSIFIED: real native 0.05s missing-frame deadline not logged; not timeout-continuation RED".into());
    }
    // Only the queued JS successor can emit these live formatter records. Neither
    // observer nor parent calls a dump. Do not infer cross-pipe order from reader scheduling.
    for frame in ["UsernameInput [EditBox]", "ConnectButton [Button]"] {
        if !lines.iter().any(|output| {
            output.stdout
                && output.line.trim_start().starts_with(frame)
                && output.line.contains(" visible ")
                && output.line.contains("alpha=")
        }) {
            return Err(format!(
                "FEATURE RED: real missing-frame deadline logged but subsequent live stdout dump lacks visible {frame} with alpha; timeout queue continuation missing"
            ));
        }
    }
    println!(
        "PASS: timeout-continuation, real native missing-frame deadline, successor live stdout Login UI dump, zero decoded auth and normal observed child exit; bounded feature only"
    );
    Ok(())
}

fn run_fixture() -> Result<(), String> {
    let mode = Mode::from_args()?;
    let root = fixture_support::checkout_root_from_executable("native_js_automation_fixture")
        .map_err(|error| format!("SETUP: {error}"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("SETUP: diagnostics clock: {error}"))?
        .as_nanos();
    let artifacts = root.join(format!(
        "data/diagnostics/native-js-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&artifacts).map_err(|error| format!("SETUP: artifacts: {error}"))?;
    let mut log =
        fs::File::create(artifacts.join("native.log")).map_err(|error| error.to_string())?;
    let (mut app, address) = start_server()?;
    let (mut child, receiver) = launch(&root, address, &artifacts, mode)?;
    println!(
        "ARTIFACTS: {} own client PID={} loopback={address}",
        artifacts.display(),
        child.0.id()
    );
    let deadline = Instant::now() + TIMEOUT;
    let mut lines = Vec::new();
    let mut auth_count = 0;
    let mut exited = None;
    loop {
        app.update();
        authenticate(&mut app, &mut auth_count, mode)?;
        drain_output(&receiver, &mut log, &mut lines)?;
        if exited.is_none() {
            exited = child
                .0
                .try_wait()
                .map_err(|error| format!("SETUP: own child status: {error}"))?;
        }
        if exited.is_some() {
            // Readers can still be draining after waitpid; require their channel closure.
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(output) => {
                    let output = output?;
                    writeln!(log, "{}", output.line).map_err(|error| error.to_string())?;
                    if output.password_leaked {
                        return Err(
                            "FEATURE: child exposed raw password; diagnostic copy masked".into(),
                        );
                    }
                    lines.push(output);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "{}: 180-second bounded deadline; forced own-child cleanup is not shutdown proof",
                if artifacts.join("login-ready").is_file() {
                    "FEATURE/UNCLASSIFIED"
                } else {
                    "SETUP/UNCLASSIFIED"
                }
            ));
        }
        thread::sleep(Duration::from_millis(5));
    }
    let status = exited.ok_or("SETUP: missing own child exit status")?;
    if !status.success() {
        let unknown_flag = lines.iter().any(|line| {
            line.line
                .contains("unknown client option '--run-js-ui-script'")
                || line.line.contains("Unknown option '--run-js-ui-script'")
        });
        if unknown_flag {
            return Err("FEATURE RED: production startup rejects --run-js-ui-script; no JS execution/auth/dump proof".into());
        }
        let feature = artifacts.join("feature-failure").is_file();
        return Err(format!(
            "{}: child exited {status}; inspect masked native.log",
            if feature {
                "FEATURE RED"
            } else {
                "SETUP/UNCLASSIFIED (not feature RED)"
            }
        ));
    }
    if mode == Mode::TimeoutContinuation {
        return assert_timeout_continuation(&lines, &artifacts, auth_count);
    }
    if auth_count != 1 || !artifacts.join("observed-charselect").is_file() {
        return Err(
            "FEATURE: missing exact authenticated request or authored CharSelect observation"
                .into(),
        );
    }
    let stdout: Vec<_> = lines
        .iter()
        .filter(|output| output.stdout)
        .map(|output| output.line.as_str())
        .collect();
    assert_dump(&stdout)?;
    println!(
        "PASS: unchanged login.js, authored credentials/click, exact protocol auth, real CharSelect and stdout UI dump; bounded feature only"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run_fixture() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
