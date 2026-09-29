//! Authenticated local-player movement through authored native spatial footsteps.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut stage = 0;
    let mut saw_forward = false;
    let deadline = Instant::now() + TIMEOUT;
    let mut readers = Some(readers);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Footsteps)?;
        respond_to_selection(app, StartupScreen::Footsteps, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            let line = line.trim();
            if line.starts_with("GODOT_STDERR: ERROR:")
                || line.starts_with("GODOT_STDERR: SCRIPT ERROR:")
            {
                return Err(format!("Godot footstep runtime error: {line}"));
            }
            match line {
                "FIXTURE FOOTSTEPS_LOADING" if stage == 0 && selected.is_some() => {
                    if !take_inputs(app).is_empty() {
                        return Err("PlayerInput arrived during withheld terrain Loading".into());
                    }
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "azeroth".into(),
                            initial_tile_y: 32,
                            initial_tile_x: 48,
                        },
                    );
                    stage = 1;
                }
                "FIXTURE FOOTSTEPS_DONE" if stage == 1 => {
                    saw_forward |= assert_forward_input(take_inputs(app))?;
                    if !saw_forward {
                        return Err("no decoded forward-run PlayerInput on InputChannel".into());
                    }
                    stage = 2;
                }
                line if line.starts_with("FIXTURE FOOTSTEPS_") => {
                    return Err(format!(
                        "out-of-order footsteps marker at stage {stage}: {line}"
                    ));
                }
                _ => {}
            }
        }
        if stage == 0 && !take_inputs(app).is_empty() {
            return Err("PlayerInput arrived before Loading marker".into());
        }
        if stage == 1 {
            saw_forward |= assert_forward_input(take_inputs(app))?;
        }
        if let Some(status) = status {
            if status.success() && stage == 2 && selected.is_some() {
                ensure_release_reported(app, stage)?;
                println!(
                    "PASS: authenticated GameClient movement decoded by owned UDP server; selected Run phase, real Ogg catalog, surface and 3D player observed in Godot; quiet after stop"
                );
                return Ok(());
            }
            return Err(format!("footstep fixture exited {status} at stage {stage}"));
        }
        thread::sleep(TICK);
    }
    Err(format!("timed out awaiting footsteps stage {stage}"))
}
