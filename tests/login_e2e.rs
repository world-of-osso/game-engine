//! End-to-end login test: spawns the game server, launches the client with
//! auto-login, and verifies the client reaches the CharSelect screen.
//!
//! Run: `cargo test --test login_e2e -- --ignored`
//! Requires: game-server binary built, windowing or xvfb-run.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(45);

/// Spawns the game-server, connects the client with `--screen charselect`
/// (auto-login admin/admin), waits for the IPC socket, then uses the CLI
/// to dump the UI tree and verify the client reached CharSelect.
#[test]
#[ignore = "requires game-server binary and windowing; run with: cargo test --test login_e2e -- --ignored"]
fn login_reaches_char_select() {
    let engine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let server_dir = engine_dir.join("../game-server");
    let server_bin = server_dir.join("target/debug/game-server");
    if !server_bin.exists() {
        panic!(
            "game-server binary not found at {}; build it first: cd ../game-server && cargo build",
            server_bin.display()
        );
    }

    let cli_bin = engine_dir.join("target/debug/game-engine-cli");
    if !cli_bin.exists() {
        panic!(
            "game-engine-cli binary not found at {}; build it first: cargo build --bin game-engine-cli",
            cli_bin.display()
        );
    }

    // 1. Start the game server
    let mut server = Command::new(&server_bin)
        .current_dir(&server_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn game-server");

    if !wait_for_udp_port(5000) {
        let _ = server.kill();
        let _ = server.wait();
        panic!("game-server did not start listening on UDP 5000 within {SERVER_STARTUP_TIMEOUT:?}");
    }

    // 2. Start the client with auto-login to CharSelect
    let client_bin = PathBuf::from(env!("CARGO_BIN_EXE_game-engine"));
    let temp_config = temp_dir("login-e2e");
    pre_accept_eula(&temp_config);

    let mut cmd = build_client_command(&client_bin);
    cmd.current_dir(&engine_dir)
        .env("XDG_CONFIG_HOME", &temp_config)
        .env("SKIP_EULA", "1")
        .args(["--screen", "charselect", "--server", "127.0.0.1:5000"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut client = cmd.spawn().expect("failed to spawn game-engine client");
    let client_pid = client.id();

    // 3. Wait for the IPC socket to appear
    let sock_pattern = format!("/tmp/game-engine-{client_pid}.sock");
    if !wait_for_file(&sock_pattern) {
        let _ = client.kill();
        let _ = client.wait();
        let _ = server.kill();
        let _ = server.wait();
        panic!("IPC socket {sock_pattern} did not appear within {SERVER_STARTUP_TIMEOUT:?}");
    }

    // 4. Wait for the login flow to complete (poll with dump-ui-tree)
    let deadline = Instant::now() + CLIENT_TIMEOUT;
    let mut last_output = String::new();
    let mut reached_charselect = false;
    let mut client_failure = None;

    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(2));
        // Check if client is still alive
        if let Some(status) = client.try_wait().ok().flatten() {
            let stderr = read_all(client.stderr.take());
            client_failure = Some(format!(
                "client exited with {status}\nstderr tail:\n{}",
                tail_with_limit(&stderr, 2000)
            ));
            eprintln!("e2e: {}", client_failure.as_deref().unwrap_or_default());
            break;
        }
        match Command::new(&cli_bin)
            .args(["--socket", &sock_pattern, "dump-ui-tree"])
            .output()
        {
            Ok(output) => {
                last_output = String::from_utf8_lossy(&output.stdout).to_string();
                if last_output.contains("CharSelectRoot")
                    || last_output.contains("EnterWorldButton")
                {
                    reached_charselect = true;
                    break;
                }
            }
            Err(_) => {}
        }
    }

    // 5. Cleanup
    let _ = client.kill();
    let _ = client.wait();
    let _ = server.kill();
    let _ = server.wait();

    // 6. Assert
    assert!(
        reached_charselect,
        "client did not reach CharSelect within {CLIENT_TIMEOUT:?};\nlast UI tree:\n{}\nclient failure:\n{}",
        tail_with_limit(&last_output, 2000),
        client_failure
            .as_deref()
            .unwrap_or("<client still running when timeout elapsed>")
    );
}

// --- Helpers ---

fn build_client_command(client_bin: &PathBuf) -> Command {
    if should_wrap_in_xvfb() {
        let mut cmd = Command::new("xvfb-run");
        cmd.arg("-a").arg(client_bin);
        cmd
    } else {
        Command::new(client_bin)
    }
}

fn pre_accept_eula(xdg_config: &std::path::Path) {
    let config_dir = xdg_config.join("game-engine");
    std::fs::create_dir_all(&config_dir).expect("create config dir");
    std::fs::write(
        config_dir.join("options_settings.ron"),
        "(accepted_eula: true)\n",
    )
    .expect("write options_settings.ron");
}

fn wait_for_file(path: &str) -> bool {
    let deadline = Instant::now() + SERVER_STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if std::path::Path::new(path).exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

fn wait_for_udp_port(port: u16) -> bool {
    let needle = format!(":{port}");
    let deadline = Instant::now() + SERVER_STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(output) = Command::new("ss").args(["-ulnp"]).output() {
            if String::from_utf8_lossy(&output.stdout).contains(&needle) {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    false
}

fn read_all(handle: Option<impl std::io::Read>) -> String {
    handle
        .map(|h| {
            BufReader::new(h)
                .lines()
                .filter_map(|l| l.ok())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn tail_with_limit(text: &str, max_chars: usize) -> &str {
    let total_chars = text.chars().count();
    if total_chars <= max_chars {
        return text;
    }
    let skip = total_chars - max_chars;
    let start = text
        .char_indices()
        .nth(skip)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    &text[start..]
}

fn should_wrap_in_xvfb() -> bool {
    std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && Command::new("sh")
            .arg("-c")
            .arg("command -v xvfb-run >/dev/null 2>&1")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "game-engine-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).expect("temp dir should be creatable");
    path
}
