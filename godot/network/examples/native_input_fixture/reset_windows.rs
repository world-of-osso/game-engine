//! Authenticated authored Options reset and canonical config persistence boundary.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
    config: &Path,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut readers = Some(readers);
    let mut loading = false;
    let mut passed = false;
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::ResetWindows)?;
        respond_to_selection(app, StartupScreen::ResetWindows, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            observe_reset_line(app, &line, selected.is_some(), &mut loading, &mut passed)?;
        }
        if let Some(status) = status {
            if !status.success() || !passed {
                return Err(format!(
                    "Godot reset fixture exited {status}; pass={passed}"
                ));
            }
            if !loading || selected.is_none() {
                return Err("Reset fixture did not authenticate and load world".into());
            }
            verify_saved_layout(config)?;
            return verify_fresh_process(config);
        }
        thread::sleep(TICK);
    }
    Err("timed out waiting for authenticated Reset Window Positions".into())
}

fn observe_reset_line(
    app: &mut App,
    line: &str,
    selected: bool,
    loading: &mut bool,
    passed: &mut bool,
) -> Result<(), String> {
    if line.trim() == "FIXTURE RESET_LOADING" && selected && !*loading {
        send::<_, TerrainChannel>(
            app,
            LoadTerrain {
                map_name: "azeroth".into(),
                initial_tile_y: 32,
                initial_tile_x: 48,
            },
        );
        *loading = true;
    }
    let missing_optional_texture = line.contains("GODOT_STDERR: ERROR: WorldObjects:")
        && line.contains("particle textures missing");
    if !missing_optional_texture
        && (line.contains("GODOT_STDERR: ERROR:") || line.contains("GODOT_STDERR: SCRIPT ERROR:"))
    {
        return Err(format!("Godot reset runtime error: {line}"));
    }
    if line
        .trim()
        .starts_with("PASS: authenticated authored reset")
    {
        *passed = true;
    }
    Ok(())
}

fn verify_saved_layout(config: &Path) -> Result<(), String> {
    let path = config.join("world-of-osso/ui_layout.ron");
    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if raw.contains("\"17\"") || !raw.contains("\"18\"") || !raw.contains("active_layout") {
        return Err(format!("Reset file not character-scoped: {raw}"));
    }
    Ok(())
}

fn verify_fresh_process(config: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("fixture checkout root");
    let binary = std::env::var_os("GODOT_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME for pinned Godot"))
                .join(".cache/game-engine/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64")
        });
    let output = Command::new(binary)
        .args(["--headless", "--path"])
        .arg(root.join("godot"))
        .args(["--script", "res://tests/options_reset_windows.gd"])
        .env("XDG_CONFIG_HOME", config)
        .env("GODOT_TEST_RESET_VERIFY", "1")
        .output()
        .map_err(|error| format!("relaunch Reset verification: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stdout.contains("PASS: fresh process read reset layout") {
        return Err(format!(
            "fresh-process layout read failed: {stdout}\n{stderr}"
        ));
    }
    println!("{stdout}");
    Ok(())
}
