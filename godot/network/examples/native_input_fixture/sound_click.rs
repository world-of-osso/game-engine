//! Authenticated native spell-button effects through an owned UDP server.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut loading = false;
    let mut passed = false;
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::SoundClick)?;
        respond_to_selection(app, StartupScreen::SoundClick, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            observe_line(
                app,
                line.trim(),
                selected.is_some(),
                &mut loading,
                &mut passed,
            )?;
        }
        if let Some(status) = status {
            if status.success() && passed {
                println!(
                    "PASS: replicated spell snapshots, authored spellbook/action-bar pointer effects and quiet controls"
                );
                return Ok(());
            }
            return Err(format!(
                "spell-click fixture exited {status}; loading={loading}, passed={passed}"
            ));
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out awaiting spell-click fixture; loading={loading}, passed={passed}"
    ))
}

fn observe_line(
    app: &mut App,
    line: &str,
    selected: bool,
    loading: &mut bool,
    passed: &mut bool,
) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("Godot spell-click runtime error: {line}"));
    }
    match line {
        "FIXTURE SPELL_CLICK_LOADING" if selected && !*loading => {
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
        "FIXTURE SPELL_CLICK_DONE" if *loading && selected => *passed = true,
        line if line.starts_with("FIXTURE SPELL_CLICK_") => {
            return Err(format!("out-of-order spell-click marker: {line}"));
        }
        _ => {}
    }
    Ok(())
}
