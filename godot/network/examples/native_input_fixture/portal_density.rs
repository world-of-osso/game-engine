//! Controlled density-sensitive portal copy: live Options only affects new placements.
use super::*;
use shared::protocol::{NewWorld, TransferChannel};

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut stage = 0;
    let mut readers = Some(readers);
    let deadline = Instant::now() + Duration::from_secs(1500);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::PortalDensity)?;
        respond_to_selection(
            app,
            StartupScreen::PortalDensity,
            &mut selected,
            &mut remote,
        )?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        join_readers_if_exited(status.is_some(), &mut readers)?;
        read_fixture_output(&lines, app, selected, &mut stage)?;
        if let Some(status) = status {
            return fixture_exit(status, stage, selected);
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for density fixture stage {stage}"
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
    selected: Option<Entity>,
    stage: &mut i32,
) -> Result<(), String> {
    for line in lines.after_selection(selected.is_some()) {
        let line = line.trim();
        portal_particles::reject_fixture_error(line)?;
        advance_fixture_stage(app, stage, line)?;
    }
    Ok(())
}

fn advance_fixture_stage(app: &mut App, stage: &mut i32, line: &str) -> Result<(), String> {
    match (*stage, line) {
        (0, "FIXTURE DENSITY_LOADING") => {
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
        (1, "FIXTURE DENSITY_TRANSFER") => {
            send::<_, TransferChannel>(
                app,
                NewWorld {
                    map_id: 0,
                    map_directory: "azeroth".into(),
                    position: [-8766.11, 88.5, -845.5],
                    facing: 0.0,
                },
            );
            *stage = 2;
        }
        (2, "FIXTURE DENSITY_DONE") => *stage = 3,
        (_, line) if line.starts_with("FIXTURE DENSITY_") => {
            return Err(format!(
                "out-of-order density marker at stage {stage}: {line}"
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
    if status.success() && stage == 3 && selected.is_some() {
        println!("PASS: controlled density-sensitive portal live Options/new placement proof");
        return Ok(());
    }
    Err(format!("density fixture exited {status} at stage {stage}"))
}
