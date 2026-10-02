//! Authenticated local-player movement through authored native spatial footsteps.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
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
        join_readers_if_exited(status.is_some(), &mut readers)?;
        read_fixture_output(&lines, app, selected, &mut stage, &mut saw_forward)?;
        observe_fixture_inputs(app, stage, &mut saw_forward)?;
        if let Some(status) = status {
            return fixture_exit(status, stage, selected);
        }
        thread::sleep(TICK);
    }
    Err(format!("timed out awaiting footsteps stage {stage}"))
}

fn join_readers_if_exited(
    exited: bool,
    readers: &mut Option<Vec<thread::JoinHandle<()>>>,
) -> Result<(), String> {
    if exited {
        for reader in readers.take().expect("join output once") {
            reader.join().map_err(|_| "Godot output reader panicked")?;
        }
    }
    Ok(())
}

fn read_fixture_output(
    lines: &ClientLines,
    app: &mut App,
    selected: Option<Entity>,
    stage: &mut u8,
    saw_forward: &mut bool,
) -> Result<(), String> {
    for line in lines.after_selection(selected.is_some()) {
        let line = line.trim();
        reject_fixture_error(line)?;
        advance_fixture_stage(app, selected, stage, saw_forward, line)?;
    }
    Ok(())
}

fn observe_fixture_inputs(app: &mut App, stage: u8, saw_forward: &mut bool) -> Result<(), String> {
    if stage == 0 && !take_inputs(app).is_empty() {
        return Err("PlayerInput arrived before Loading marker".into());
    }
    if stage == 1 || stage == 2 {
        *saw_forward |= assert_forward_input(take_inputs(app))?;
    } else if stage >= 3 {
        take_inputs(app);
    }
    Ok(())
}

fn fixture_exit(
    status: std::process::ExitStatus,
    stage: u8,
    selected: Option<Entity>,
) -> Result<(), String> {
    if status.success() && stage == 4 && selected.is_some() {
        println!(
            "PASS: owned UDP GameClient Run/idle/stop, repeated local Ogg 3D events, music-independent mute/volume and player-removal stop"
        );
        return Ok(());
    }
    Err(format!("footstep fixture exited {status} at stage {stage}"))
}

fn reject_fixture_error(line: &str) -> Result<(), String> {
    // Optional scenery assets are absent in this worktree; the full Godot log retains errors.
    let missing_scenery =
        line.starts_with("GODOT_STDERR: ERROR: WorldObjects:") && line.contains("missing textures");
    if !missing_scenery
        && (line.starts_with("GODOT_STDERR: ERROR:")
            || line.starts_with("GODOT_STDERR: SCRIPT ERROR:"))
    {
        return Err(format!("Godot footstep runtime error: {line}"));
    }
    Ok(())
}

fn advance_fixture_stage(
    app: &mut App,
    selected: Option<Entity>,
    stage: &mut u8,
    saw_forward: &mut bool,
    line: &str,
) -> Result<(), String> {
    match line {
        "FIXTURE FOOTSTEPS_LOADING" if *stage == 0 => {
            if !take_inputs(app).is_empty() {
                return Err("PlayerInput arrived during withheld terrain Loading".into());
            }
            send_fixture_terrain(app);
            *stage = 1;
        }
        "FIXTURE FOOTSTEPS_OPTIONS" if *stage == 1 && *saw_forward => {
            ensure_release_reported(app, *stage)?;
            *stage = 2;
        }
        "FIXTURE FOOTSTEPS_REMOVE" if *stage == 2 => {
            *saw_forward |= assert_forward_input(take_inputs(app))?;
            app.world_mut()
                .entity_mut(selected.expect("selected player for removal"))
                .despawn();
            *stage = 3;
        }
        "FIXTURE FOOTSTEPS_DONE" if *stage == 3 => *stage = 4,
        line if line.starts_with("FIXTURE FOOTSTEPS_") => {
            return Err(format!(
                "out-of-order footsteps marker at stage {stage}: {line}"
            ));
        }
        _ => {}
    }
    Ok(())
}

fn send_fixture_terrain(app: &mut App) {
    send::<_, TerrainChannel>(
        app,
        LoadTerrain {
            map_name: "azeroth".into(),
            initial_tile_y: 32,
            initial_tile_x: 48,
        },
    );
}
