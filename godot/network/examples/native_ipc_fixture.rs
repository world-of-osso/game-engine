//! Public CLI/process-boundary diagnostics fixture. MAIN builds and runs this example.
//! Requires GODOT_BIN and GAME_ENGINE_CLI pointing to existing executables.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[path = "fixture_support/mod.rs"]
mod fixture_support;

struct NativeProcess(Child);

impl Drop for NativeProcess {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            // Failure cleanup only; never evidence of normal native shutdown.
            if let Err(error) = self.0.kill() {
                eprintln!("Fixture failure cleanup: {error}");
            }
            if let Err(error) = self.0.wait() {
                eprintln!("Fixture child reap: {error}");
            }
        }
    }
}

fn executable(variable: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("SETUP: {variable} must identify an existing executable"))?;
    if !path.is_file() {
        return Err(format!("SETUP: {} does not exist", path.display()));
    }
    Ok(path)
}

fn wait_file(path: &Path, child: &mut NativeProcess) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if path.is_file() {
            return Ok(());
        }
        if let Some(status) = child.0.try_wait().map_err(|error| error.to_string())? {
            return Err(format!(
                "Native child exited {status} before {}",
                path.display()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(format!("Timed out waiting for {}", path.display()))
}

fn call_cli(
    cli: &Path,
    socket: &Path,
    artifacts: &Path,
    name: &str,
    args: &[&str],
) -> Result<String, String> {
    let stdout = artifacts.join(format!("{name}.stdout"));
    let stderr = artifacts.join(format!("{name}.stderr"));
    let mut child = NativeProcess(
        Command::new(cli)
            .arg("--socket")
            .arg(socket)
            .args(args)
            .stdout(Stdio::from(
                fs::File::create(&stdout).map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(&stderr).map_err(|error| error.to_string())?,
            ))
            .spawn()
            .map_err(|error| format!("SETUP: launch CLI: {error}"))?,
    );
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(status) = child.0.try_wait().map_err(|error| error.to_string())? {
            let out = fs::read_to_string(stdout).map_err(|error| error.to_string())?;
            let err = fs::read_to_string(stderr).map_err(|error| error.to_string())?;
            if !status.success() {
                return Err(format!("CLI {name} exited {status}: {err}; stdout={out}"));
            }
            return Ok(out);
        }
        if Instant::now() >= deadline {
            return Err(format!("CLI {name} exceeded 8-second request deadline"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_fixture() -> Result<(), String> {
    let repo = fixture_support::checkout_root_from_executable("native_ipc_fixture")?;
    let godot = executable("GODOT_BIN")?;
    let cli = executable("GAME_ENGINE_CLI")?;
    // Missing/unrunnable CLI is setup failure, never missing-feature RED.
    let help = Command::new(&cli)
        .arg("--help")
        .output()
        .map_err(|error| format!("SETUP: CLI: {error}"))?;
    if !help.status.success() {
        return Err("SETUP: existing game-engine-cli --help failed".into());
    }
    let artifacts = repo.join(format!(
        "data/diagnostics/native-ipc-{}",
        std::process::id()
    ));
    for directory in ["config", "user-data"] {
        fs::create_dir_all(artifacts.join(directory)).map_err(|error| error.to_string())?;
    }
    let mut native = NativeProcess(
        Command::new(godot)
            .args(["--audio-driver", "Dummy", "--path"])
            .arg(repo.join("godot"))
            .args([
                "-s",
                "res://tests/native_ipc_flow.gd",
                "--",
                "--screen",
                "login",
            ])
            .env("GODOT_IPC_ARTIFACTS", &artifacts)
            .env("XDG_CONFIG_HOME", artifacts.join("config"))
            .env("XDG_DATA_HOME", artifacts.join("user-data"))
            .stdout(Stdio::from(
                fs::File::create(artifacts.join("native.log"))
                    .map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(artifacts.join("native.stderr"))
                    .map_err(|error| error.to_string())?,
            ))
            .spawn()
            .map_err(|error| format!("SETUP: native Godot: {error}"))?,
    );
    wait_file(&artifacts.join("ready"), &mut native)?;
    let socket = PathBuf::from(format!("/tmp/game-engine-{}.sock", native.0.id()));
    println!(
        "READY native PID={} socket={} artifacts={}",
        native.0.id(),
        socket.display(),
        artifacts.display()
    );
    let pong = call_cli(&cli, &socket, &artifacts, "ping", &["ping"])?;
    if pong.trim() != "pong" {
        return Err(format!("CLI ping changed existing plain output: {pong:?}"));
    }
    for (name, args) in [
        ("ping-json", vec!["--json", "ping"]),
        ("tree", vec!["--json", "dump-tree"]),
        (
            "tree-filter",
            vec!["--json", "dump-tree", "--filter", "ipcfixturemesh"],
        ),
        (
            "tree-empty",
            vec!["--json", "dump-tree", "--filter", "NoSuchIpcFixtureNode"],
        ),
        ("ui", vec!["--json", "dump-ui-tree"]),
        (
            "ui-filter",
            vec!["--json", "dump-ui-tree", "--filter", "ConnectButton"],
        ),
        ("scene", vec!["--json", "dump-scene"]),
        ("performance", vec!["--json", "performance"]),
    ] {
        call_cli(&cli, &socket, &artifacts, name, &args)?;
    }
    for phase in ["red", "green"] {
        let image = artifacts.join(format!("{phase}.webp"));
        let image_path = image.to_str().ok_or("Non-UTF8 fixture image path")?;
        call_cli(
            &cli,
            &socket,
            &artifacts,
            phase,
            &["screenshot", image_path],
        )?;
        fs::write(artifacts.join(format!("verify-{phase}")), "")
            .map_err(|error| error.to_string())?;
        wait_file(&artifacts.join(format!("verified-{phase}")), &mut native)?;
    }
    fs::write(artifacts.join("finish"), "").map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = native.0.try_wait().map_err(|error| error.to_string())? {
            if !status.success() {
                return Err(format!("Native assertions exited {status}"));
            }
            if socket.exists() {
                return Err(format!(
                    "Own PID socket survived native exit: {}",
                    socket.display()
                ));
            }
            println!(
                "PASS: existing public CLI/native diagnostics, live WebP frames and own socket cleanup"
            );
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("Native fixture did not exit normally within 10 seconds".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    if let Err(error) = run_fixture() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
