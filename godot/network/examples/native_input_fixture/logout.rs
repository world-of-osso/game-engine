//! Actual authored logout action against an owned UDP account/world.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut loaded = false;
    let mut complete = false;
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Logout)?;
        respond_to_selection(app, StartupScreen::Logout, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            match line.trim() {
                "FIXTURE LOGOUT_LOADING" if !loaded && selected.is_some() => {
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "azeroth".into(),
                            initial_tile_y: 32,
                            initial_tile_x: 48,
                        },
                    );
                    loaded = true;
                }
                "FIXTURE LOGOUT_DONE" if loaded => complete = true,
                line if line.starts_with("GODOT_STDERR: ERROR:")
                    || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") =>
                {
                    return Err(line.into());
                }
                line if line.starts_with("FIXTURE LOGOUT_") => {
                    return Err(format!("Unexpected logout marker: {line}"));
                }
                _ => {}
            }
        }
        if !take_inputs(app).is_empty() {
            return Err("Unexpected movement input during idle logout".into());
        }
        if let Some(status) = status {
            if !status.success() || !complete {
                return Err(format!(
                    "Logout fixture exited {status}; complete={complete}"
                ));
            }
            println!("PASS: authored logout countdown expires to Login");
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err("Timed out waiting for native logout".into())
}
