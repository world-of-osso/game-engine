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
            // This headless footstep fixture has no need for optional scenery textures.
            // Preserve the original diagnostic in the captured Godot log.
            if line.starts_with("GODOT_STDERR: ERROR: WorldObjects:")
                && line.contains("missing textures")
            {
                continue;
            }
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
                "FIXTURE FOOTSTEPS_OPTIONS" if stage == 1 && saw_forward => {
                    ensure_release_reported(app, stage)?;
                    stage = 2;
                }
                "FIXTURE FOOTSTEPS_REMOVE" if stage == 2 => {
                    saw_forward |= assert_forward_input(take_inputs(app))?;
                    app.world_mut()
                        .entity_mut(selected.expect("selected player for removal"))
                        .despawn();
                    stage = 3;
                }
                "FIXTURE FOOTSTEPS_DONE" if stage == 3 => stage = 4,
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
        if stage == 1 || stage == 2 {
            saw_forward |= assert_forward_input(take_inputs(app))?;
        } else if stage >= 3 {
            take_inputs(app);
        }
        if let Some(status) = status {
            if status.success() && stage == 4 && selected.is_some() {
                println!(
                    "PASS: owned UDP GameClient Run/idle/stop, repeated local Ogg 3D events, music-independent mute/volume and player-removal stop"
                );
                return Ok(());
            }
            return Err(format!("footstep fixture exited {status} at stage {stage}"));
        }
        thread::sleep(TICK);
    }
    Err(format!("timed out awaiting footsteps stage {stage}"))
}
