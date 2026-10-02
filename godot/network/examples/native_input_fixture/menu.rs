//! Native menu modal proof against the real launcher and decoded loopback UDP.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    CharacterSelect,
    Loading,
    World,
    Blocked,
    BlockedQuiet,
    Resumed,
    Released,
    FinalQuiet,
    ExitRequested,
}

struct Progress {
    stage: Stage,
    selected: Option<Entity>,
    remote: Option<Entity>,
    quiet_at: Option<Instant>,
    saw_forward: bool,
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut progress = Progress {
        stage: Stage::CharacterSelect,
        selected: None,
        remote: None,
        quiet_at: None,
        saw_forward: false,
    };
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Menu)?;
        respond_to_selection(
            app,
            StartupScreen::Menu,
            &mut progress.selected,
            &mut progress.remote,
        )?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.after_selection(progress.selected.is_some()) {
            advance_marker(app, &mut progress, line.trim())?;
        }
        check_inputs(app, &mut progress)?;
        if let Some(status) = status {
            if !status.success() || progress.stage != Stage::ExitRequested {
                return Err(format!("Godot exited {status} at {:?}", progress.stage));
            }
            if !progress.saw_forward {
                return Err("no decoded forward input after closing menu".into());
            }
            if !progress
                .quiet_at
                .is_some_and(|at| at.elapsed() >= RELEASE_DRAIN + RELEASE_QUIET)
            {
                return Err("final release lacked bounded UDP quiet interval".into());
            }
            println!(
                "PASS: native charselect/world menu modal blocked roster and movement/UDP; Return restored movement, Exit closed client"
            );
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for native menu at {:?}",
        progress.stage
    ))
}

fn advance_marker(app: &mut App, progress: &mut Progress, line: &str) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("Godot menu runtime error: {line}"));
    }
    match (progress.stage, line) {
        (Stage::CharacterSelect, "FIXTURE MENU_CHARSELECT_DONE") => {
            if progress.selected.is_some() || !take_inputs(app).is_empty() {
                return Err("character-select menu selected character or sent input".into());
            }
            progress.stage = Stage::Loading;
        }
        (Stage::Loading, "FIXTURE MENU_LOADING") => {
            if !take_inputs(app).is_empty() {
                return Err("menu world loading sent input".into());
            }
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            progress.stage = Stage::World;
        }
        (Stage::World, "FIXTURE MENU_WORLD_OPEN") => progress.stage = Stage::Blocked,
        (Stage::Blocked, "FIXTURE MENU_BLOCK_QUIET_START") => {
            progress.quiet_at = Some(Instant::now());
            progress.stage = Stage::BlockedQuiet;
        }
        (Stage::BlockedQuiet, "FIXTURE MENU_BLOCK_DONE") => {
            ensure_quiet(progress.quiet_at, "blocked menu")?;
            progress.stage = Stage::Resumed;
        }
        (Stage::Resumed, "FIXTURE MENU_W_RELEASED") => {
            if !progress.saw_forward {
                return Err("no decoded W input after menu Return".into());
            }
            progress.quiet_at = Some(Instant::now());
            progress.stage = Stage::Released;
        }
        (Stage::Released, "FIXTURE MENU_FINAL_QUIET") => {
            ensure_quiet(progress.quiet_at, "movement release")?;
            ensure_release_reported(app, "movement release")?;
            progress.stage = Stage::FinalQuiet;
        }
        (Stage::FinalQuiet, "FIXTURE MENU_EXIT_READY") => {
            ensure_quiet(progress.quiet_at, "final menu")?;
            progress.stage = Stage::ExitRequested;
        }
        (_, line) if line.starts_with("FIXTURE MENU_") => {
            return Err(format!(
                "out-of-order menu marker {:?}: {line}",
                progress.stage
            ));
        }
        _ => {}
    }
    Ok(())
}

fn check_inputs(app: &mut App, progress: &mut Progress) -> Result<(), String> {
    let inputs = take_inputs(app);
    if progress.stage == Stage::Resumed {
        progress.saw_forward |= assert_forward_input(inputs)?;
        return Ok(());
    }
    let require_quiet = match progress.stage {
        Stage::Released | Stage::FinalQuiet | Stage::ExitRequested => progress
            .quiet_at
            .is_some_and(|at| at.elapsed() >= RELEASE_DRAIN),
        _ => true,
    };
    if require_quiet && !inputs.is_empty() {
        return Err(format!(
            "decoded PlayerInput during {:?}: {inputs:?}",
            progress.stage
        ));
    }
    Ok(())
}

fn ensure_quiet(since: Option<Instant>, stage: &str) -> Result<(), String> {
    if !since.is_some_and(|at| at.elapsed() >= RELEASE_DRAIN + RELEASE_QUIET) {
        return Err(format!("{stage} lacked bounded UDP quiet interval"));
    }
    Ok(())
}
