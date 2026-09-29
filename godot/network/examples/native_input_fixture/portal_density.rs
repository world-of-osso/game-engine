//! Controlled density-sensitive portal copy: live Options only affects new placements.
use super::*;
use shared::protocol::{NewWorld, TransferChannel};

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
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
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            let line = line.trim();
            portal_particles::reject_fixture_error(line)?;
            match (stage, line) {
                (0, "FIXTURE DENSITY_LOADING") if selected.is_some() => {
                    send::<_, TerrainChannel>(
                        app,
                        LoadTerrain {
                            map_name: "azeroth".into(),
                            initial_tile_y: 30,
                            initial_tile_x: 48,
                        },
                    );
                    stage = 1;
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
                    stage = 2;
                }
                (2, "FIXTURE DENSITY_DONE") => stage = 3,
                (_, line) if line.starts_with("FIXTURE DENSITY_") => {
                    return Err(format!(
                        "out-of-order density marker at stage {stage}: {line}"
                    ));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if status.success() && stage == 3 && selected.is_some() {
                println!(
                    "PASS: controlled density-sensitive portal live Options/new placement proof"
                );
                return Ok(());
            }
            return Err(format!("density fixture exited {status} at stage {stage}"));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for density fixture stage {stage}"
    ))
}
