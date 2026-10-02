//! Real GameClient sound pipeline through authenticated loopback world load.
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
    let deadline = Instant::now() + TIMEOUT;
    let mut readers = Some(readers);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Sound)?;
        respond_to_selection(app, StartupScreen::Sound, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.after_selection(selected.is_some()) {
            let line = line.trim();
            if line.starts_with("GODOT_STDERR: ERROR:")
                || line.starts_with("GODOT_STDERR: SCRIPT ERROR:")
            {
                return Err(format!("Godot sound runtime error: {line}"));
            }
            match line {
                "FIXTURE SOUND_LOADING" if stage == 0 => {
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
                "FIXTURE SOUND_WORLD area=9 zone=12 music=53492 ambient=none" if stage == 1 => {
                    stage = 2
                }
                "FIXTURE SOUND_OPTIONS" if stage == 2 => stage = 3,
                line if line.starts_with("FIXTURE SOUND_") => {
                    return Err(format!(
                        "out-of-order sound marker at stage {stage}: {line}"
                    ));
                }
                _ => {}
            }
        }
        if let Some(status) = status {
            if status.success() && stage == 3 && selected.is_some() {
                println!(
                    "PASS: GameClient authenticated/loaded MCNK 9 -> zone 12 -> music 53492; options changed native player volume/mute/music-enable; ambient has no zone 12 track"
                );
                return Ok(());
            }
            return Err(format!(
                "sound fixture exited {status} at stage {stage}; selected={}",
                selected.is_some()
            ));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out awaiting sound stage {stage}; selected={}",
        selected.is_some()
    ))
}
