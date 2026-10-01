//! `export-scene` on character select through the public CLI: once the observing script
//! has the campsite and the roster's first character loaded, the parent calls the real
//! `game-engine-cli` (GAME_ENGINE_CLI) on the client's own-PID socket and checks the
//! original reply; the script checks the written snapshot against the live scene.
use super::*;

const MARKER_WAIT: Duration = Duration::from_secs(120);
const CLI_DEADLINE: Duration = Duration::from_secs(8);

struct Run<'a> {
    app: &'a mut App,
    child: &'a mut Child,
    lines: Receiver<String>,
    markers: Vec<String>,
}

impl Run<'_> {
    fn pump(&mut self) -> Result<(), String> {
        self.app.update();
        respond_to_login(self.app, StartupScreen::CharSelectExport)?;
        for line in self.lines.try_iter() {
            let line = line.trim().trim_start_matches("GODOT_STDERR: ");
            if line.starts_with("SCRIPT ERROR") || line.contains("res://tests/charselect_export") {
                return Err(format!("Godot char-select export flow: {line}"));
            }
            if let Some(marker) = line.strip_prefix("FIXTURE CHARSELECT_EXPORT_") {
                self.markers.push(marker.to_owned());
            }
        }
        if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())?
            && !self.markers.iter().any(|marker| marker == "DONE")
        {
            return Err(format!(
                "Godot exited {status} before DONE: {:?}",
                self.markers
            ));
        }
        Ok(())
    }

    fn wait_marker(&mut self, wanted: &str) -> Result<(), String> {
        let deadline = Instant::now() + MARKER_WAIT;
        while Instant::now() < deadline {
            self.pump()?;
            if let Some(index) = self.markers.iter().position(|marker| marker == wanted) {
                self.markers.remove(index);
                return Ok(());
            }
            thread::sleep(TICK);
        }
        Err(format!(
            "timed out waiting for CHARSELECT_EXPORT_{wanted}: {:?}",
            self.markers
        ))
    }

    /// The real CLI's stdout; the server keeps stepping while the client answers.
    fn cli(&mut self, cli: &Path, args: &[&str], artifacts: &Path) -> Result<String, String> {
        let stdout = artifacts.join("export.stdout");
        let stderr = artifacts.join("export.stderr");
        let mut process = Command::new(cli)
            .arg("--socket")
            .arg(format!("/tmp/game-engine-{}.sock", self.child.id()))
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
            if let Some(status) = process.try_wait().map_err(|error| error.to_string())? {
                break status;
            }
            if Instant::now() >= deadline {
                process.kill().map_err(|error| error.to_string())?;
                process.wait().map_err(|error| error.to_string())?;
                return Err(format!("CLI {args:?} exceeded its request deadline"));
            }
            self.pump()?;
            thread::sleep(TICK);
        };
        let out = fs::read_to_string(&stdout).map_err(|error| error.to_string())?;
        if !status.success() {
            let err = fs::read_to_string(&stderr).map_err(|error| error.to_string())?;
            return Err(format!("CLI {args:?} exited {status}: {err}"));
        }
        Ok(out)
    }
}

/// The real CLI's `export-scene` into `artifacts`, with its original reply.
fn export(run: &mut Run, cli: &Path, artifacts: &Path) -> Result<(), String> {
    let path = artifacts.join("scene-export.json");
    let path = path.to_str().ok_or("non-UTF-8 artifact path")?.to_owned();
    let reply = run.cli(cli, &["export-scene", &path], artifacts)?;
    println!("CLI export-scene -> {}", reply.trim());
    if reply.trim() != format!("scene exported to {path}") {
        return Err(format!("export-scene answered {reply}"));
    }
    Ok(())
}

/// The script's checks end with DONE and a normal exit.
fn finish(run: &mut Run, readers: Vec<thread::JoinHandle<()>>) -> Result<(), String> {
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
) -> Result<(), String> {
    let cli = std::env::var_os("GAME_ENGINE_CLI")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .ok_or("SETUP: GAME_ENGINE_CLI must name the existing game-engine-cli")?;
    let artifacts = root.join(format!("data/diagnostics/charselect-export-{}", child.id()));
    fs::create_dir_all(&artifacts).map_err(|error| error.to_string())?;
    let mut run = Run {
        app,
        child,
        lines,
        markers: Vec::new(),
    };
    run.wait_marker("READY")?;
    export(&mut run, &cli, &artifacts)?;
    finish(&mut run, readers)?;
    println!(
        "PASS: public CLI export-scene wrote the live character-select scene ({})",
        artifacts.display()
    );
    Ok(())
}
