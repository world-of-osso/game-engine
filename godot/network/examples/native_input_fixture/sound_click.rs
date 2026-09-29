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
    let mut cast_stage = 0;
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
                selected,
                &mut loading,
                &mut cast_stage,
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

fn set_cast(app: &mut App, selected: Option<Entity>, spell_id: Option<u32>) -> Result<(), String> {
    let player = selected.ok_or("no selected player for CastState")?;
    let mut entity = app.world_mut().entity_mut(player);
    if let Some(spell_id) = spell_id {
        entity.insert(shared::casting::CastState::normal(spell_id, 0, 2.0, true));
    } else {
        entity.remove::<shared::casting::CastState>();
    }
    Ok(())
}

fn observe_line(
    app: &mut App,
    line: &str,
    selected: Option<Entity>,
    loading: &mut bool,
    cast_stage: &mut u8,
    passed: &mut bool,
) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("Godot spell-click runtime error: {line}"));
    }
    match line {
        "FIXTURE SPELL_CLICK_LOADING" if selected.is_some() && !*loading => {
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
        "FIXTURE SPELL_CLICK_CAST_REQUEST_QUIET" if *loading && *cast_stage == 0 => {
            let casts = &app.world().resource::<Incoming>().casts;
            if !casts.iter().any(|cast| cast.spell_id == Some(1464)) {
                return Err(format!(
                    "no actual SLAM SpellCastIntent before confirmation: {casts:?}"
                ));
            }
            set_cast(app, selected, Some(1464))?;
            *cast_stage = 1;
        }
        "FIXTURE SPELL_CLICK_CAST_REPEAT" if *cast_stage == 1 => {
            let player = selected.ok_or("no selected player for repeated cast")?;
            app.world_mut()
                .entity_mut(player)
                .get_mut::<shared::casting::CastState>()
                .ok_or("no active cast to repeat")?
                .elapsed = 0.25;
            *cast_stage = 2;
        }
        "FIXTURE SPELL_CLICK_CAST_INACTIVE" if *cast_stage == 2 => {
            set_cast(app, selected, None)?;
            *cast_stage = 3;
        }
        "FIXTURE SPELL_CLICK_CAST_RETRIGGER" if *cast_stage == 3 => {
            set_cast(app, selected, Some(1464))?;
            *cast_stage = 4;
        }
        "FIXTURE SPELL_CLICK_CAST_MUTING" if *cast_stage == 4 => {
            set_cast(app, selected, None)?;
            *cast_stage = 5;
        }
        "FIXTURE SPELL_CLICK_CAST_MUTED" if *cast_stage == 5 => {
            set_cast(app, selected, Some(1464))?;
            *cast_stage = 6;
        }
        "FIXTURE SPELL_CLICK_CAST_REMOVAL" if *cast_stage == 6 => {
            let player = selected.ok_or("no selected player to remove")?;
            app.world_mut().despawn(player);
            *cast_stage = 7;
        }
        "FIXTURE SPELL_CLICK_DONE" if *loading && *cast_stage == 7 => *passed = true,
        line if line.starts_with("FIXTURE SPELL_CLICK_") => {
            return Err(format!("out-of-order spell-click marker: {line}"));
        }
        _ => {}
    }
    Ok(())
}
