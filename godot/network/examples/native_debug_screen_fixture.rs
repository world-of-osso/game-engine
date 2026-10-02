//! Debug-screen process fixture: `native_debug_screen_fixture <screen> [client args...]`.
//!
//! Launches the real client with `--screen <screen>` under the observation script
//! `res://tests/<screen>_screen_flow.gd`, then drives it with the public
//! `game-engine-cli` over the client's own-PID socket: `ping`, `dump-scene`,
//! `dump-tree`, `dump-ui-tree`, `screenshot` and `export-scene`. The script asserts on the CLI's
//! stdout files and the screenshot it wrote, then exits; the fixture requires a
//! normal exit and the socket's removal.
//! Requires GODOT_BIN and GAME_ENGINE_CLI pointing to existing executables.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[path = "fixture_support/mod.rs"]
mod fixture_support;
use fixture_support::FixtureChild;

const READY_TIMEOUT: Duration = Duration::from_secs(90);
const CLI_TIMEOUT: Duration = Duration::from_secs(20);
const EXIT_TIMEOUT: Duration = Duration::from_secs(60);

fn executable(variable: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("SETUP: {variable} must identify an existing executable"))?;
    if !path.is_file() {
        return Err(format!("SETUP: {} does not exist", path.display()));
    }
    Ok(path)
}

fn wait_file(path: &Path, child: &mut FixtureChild, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.is_file() {
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Err(format!(
                "Native child exited {status} before {}",
                path.display()
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!("Timed out waiting for {}", path.display()))
}

fn call_cli(
    cli: &Path,
    socket: &Path,
    artifacts: &Path,
    name: &str,
    args: &[&str],
) -> Result<(), String> {
    let stdout = artifacts.join(format!("{name}.stdout"));
    let stderr = artifacts.join(format!("{name}.stderr"));
    let mut child = FixtureChild::spawn(
        Command::new(cli)
            .arg("--socket")
            .arg(socket)
            .args(args)
            .stdout(Stdio::from(
                fs::File::create(&stdout).map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(&stderr).map_err(|error| error.to_string())?,
            )),
    )
    .map_err(|error| format!("SETUP: launch CLI: {error}"))?;
    let deadline = Instant::now() + CLI_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            if !status.success() {
                let err = fs::read_to_string(stderr).map_err(|error| error.to_string())?;
                return Err(format!("CLI {name} {args:?} exited {status}: {err}"));
            }
            println!("CLI {name} {args:?} -> {}", stdout.display());
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("CLI {name} exceeded its {CLI_TIMEOUT:?} deadline"));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn launch(
    godot: &Path,
    repo: &Path,
    screen: &str,
    client_args: &[String],
    artifacts: &Path,
) -> Result<FixtureChild, String> {
    for directory in ["config", "user-data"] {
        fs::create_dir_all(artifacts.join(directory)).map_err(|error| error.to_string())?;
    }
    let script = format!("res://tests/{screen}_screen_flow.gd");
    FixtureChild::spawn(
        Command::new(godot)
            .args(["--audio-driver", "Dummy", "--path"])
            .arg(repo.join("godot"))
            .args(["-s", &script, "--", "--screen", screen])
            .args(client_args)
            .env("GODOT_DEBUG_SCREEN_ARTIFACTS", artifacts)
            .env("XDG_CONFIG_HOME", artifacts.join("config"))
            .env("XDG_DATA_HOME", artifacts.join("user-data"))
            .stdout(Stdio::from(
                fs::File::create(artifacts.join("native.log"))
                    .map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(artifacts.join("native.stderr"))
                    .map_err(|error| error.to_string())?,
            )),
    )
    .map_err(|error| format!("SETUP: native Godot: {error}"))
}

fn wait_exit(native: &mut FixtureChild, socket: &Path) -> Result<(), String> {
    let deadline = Instant::now() + EXIT_TIMEOUT;
    loop {
        if let Some(status) = native.try_wait().map_err(|error| error.to_string())? {
            if !status.success() {
                return Err(format!("Native assertions exited {status}"));
            }
            if socket.exists() {
                return Err(format!(
                    "Own PID socket survived native exit: {}",
                    socket.display()
                ));
            }
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Native fixture did not exit within {EXIT_TIMEOUT:?}"
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn run_fixture(screen: &str, client_args: &[String]) -> Result<PathBuf, String> {
    let repo = fixture_support::checkout_root_from_executable("native_debug_screen_fixture")?;
    if !repo
        .join(format!("godot/tests/{screen}_screen_flow.gd"))
        .is_file()
    {
        return Err(format!("SETUP: no godot/tests/{screen}_screen_flow.gd"));
    }
    let godot = executable("GODOT_BIN")?;
    let cli = executable("GAME_ENGINE_CLI")?;
    let artifacts = repo.join(format!(
        "data/diagnostics/debug-screen-{screen}-{}",
        std::process::id()
    ));
    let mut native = launch(&godot, &repo, screen, client_args, &artifacts)?;
    wait_file(&artifacts.join("ready"), &mut native, READY_TIMEOUT)?;
    let socket = PathBuf::from(format!("/tmp/game-engine-{}.sock", native.id()));
    println!(
        "READY native PID={} socket={}",
        native.id(),
        socket.display()
    );
    let screenshot = artifacts.join("screen.webp");
    let screenshot = screenshot.to_str().ok_or("Non-UTF8 artifact path")?;
    let export = artifacts.join("scene-export.json");
    let export = export.to_str().ok_or("Non-UTF8 artifact path")?;
    for (name, args) in [
        ("ping", vec!["ping"]),
        ("scene", vec!["--json", "dump-scene"]),
        ("tree", vec!["--json", "dump-tree"]),
        ("ui", vec!["--json", "dump-ui-tree"]),
        ("screenshot", vec!["screenshot", screenshot]),
        ("export", vec!["export-scene", export]),
    ] {
        call_cli(&cli, &socket, &artifacts, name, &args)?;
    }
    fs::write(artifacts.join("verify"), "").map_err(|error| error.to_string())?;
    wait_exit(&mut native, &socket)?;
    Ok(artifacts)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(screen) = args.next() else {
        eprintln!("usage: native_debug_screen_fixture <screen> [client args...]");
        std::process::exit(2);
    };
    let client_args: Vec<String> = args.collect();
    match run_fixture(&screen, &client_args) {
        Ok(artifacts) => println!("PASS: --screen {screen} ({})", artifacts.display()),
        Err(error) => {
            eprintln!("FAIL: {error}");
            std::process::exit(1);
        }
    }
}
