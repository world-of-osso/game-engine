//! Persisted startup particle option against the actual placed Azeroth portal.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
    screen: StartupScreen,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut stage = 0;
    let mut readers = Some(readers);
    let deadline = Instant::now() + Duration::from_secs(480);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, screen)?;
        respond_to_selection(app, screen, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        join_readers_if_exited(status.is_some(), &mut readers)?;
        read_fixture_output(&lines, app, screen, selected, &mut stage)?;
        if let Some(status) = status {
            return fixture_exit(status, stage, selected);
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for portal particle fixture at stage {stage}"
    ))
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
    screen: StartupScreen,
    selected: Option<Entity>,
    stage: &mut i32,
) -> Result<(), String> {
    for line in lines.after_selection(selected.is_some()) {
        let line = line.trim();
        reject_fixture_error(line)?;
        advance_fixture_stage(app, screen, stage, line)?;
    }
    Ok(())
}

pub(super) fn reject_fixture_error(line: &str) -> Result<(), String> {
    // The full Godot log retains optional missing-scenery errors. Portal
    // texture failures are caught by the mesh/pool assertions below.
    let missing_scenery = line.starts_with("GODOT_STDERR: ERROR: WorldObjects:")
        && (line.contains("missing textures") || line.contains("particle textures missing"));
    if !missing_scenery
        && (line.starts_with("GODOT_STDERR: ERROR:")
            || line.starts_with("GODOT_STDERR: SCRIPT ERROR:"))
    {
        return Err(format!("Godot portal fixture error: {line}"));
    }
    Ok(())
}

fn advance_fixture_stage(
    app: &mut App,
    screen: StartupScreen,
    stage: &mut i32,
    line: &str,
) -> Result<(), String> {
    match line {
        "FIXTURE PORTAL_LOADING" if *stage == 0 => {
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 30,
                    initial_tile_x: 48,
                },
            );
            *stage = 1;
        }
        line if line.starts_with("FIXTURE PORTAL_DONE ") && *stage == 1 => {
            let expected = if screen == StartupScreen::PortalParticlesEnabled {
                "FIXTURE PORTAL_DONE enabled meshes=present pools="
            } else {
                "FIXTURE PORTAL_DONE disabled meshes=present pools=absent emitters=absent"
            };
            if !line.starts_with(expected) {
                return Err(format!("unexpected portal mode proof: {line}"));
            }
            *stage = 2;
        }
        line if line.starts_with("FIXTURE PORTAL_") => {
            return Err(format!(
                "out-of-order portal marker at stage {stage}: {line}"
            ));
        }
        _ => {}
    }
    Ok(())
}

fn fixture_exit(
    status: std::process::ExitStatus,
    stage: i32,
    selected: Option<Entity>,
) -> Result<(), String> {
    if status.success() && stage == 2 && selected.is_some() {
        println!("PASS: authenticated GameClient portal particle startup mode");
        return Ok(());
    }
    Err(format!("portal fixture exited {status} at stage {stage}"))
}
