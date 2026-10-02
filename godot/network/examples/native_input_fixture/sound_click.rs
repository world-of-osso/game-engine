//! Authenticated native spell-button effects through an owned UDP server.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
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
        for line in lines.after_selection(selected.is_some()) {
            observe_line(app, line.trim(), &mut loading, &mut passed)?;
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
    loading: &mut bool,
    passed: &mut bool,
) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("Godot spell-click runtime error: {line}"));
    }
    if !line.starts_with("FIXTURE SPELL_CLICK_") {
        return Ok(());
    }
    match line {
        "FIXTURE SPELL_CLICK_LOADING" if !*loading => {
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            *loading = true;
            Ok(())
        }
        "FIXTURE SPELL_CLICK_DONE" if *loading => {
            confirm_cast_requests(app)?;
            *passed = true;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Casting from the first bar, the book, the visible and hidden keys and the restored
/// bar each sent one Slam request.
fn confirm_cast_requests(app: &App) -> Result<(), String> {
    let casts = &app.world().resource::<Incoming>().casts;
    if casts.len() != 5 || casts.iter().any(|cast| cast.spell_id != Some(1464)) {
        return Err(format!(
            "expected five SLAM SpellCastIntents from first bar, book, visible key, hidden key and restored bar: {casts:?}"
        ));
    }
    Ok(())
}
