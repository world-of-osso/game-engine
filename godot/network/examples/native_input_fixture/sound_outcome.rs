//! Original CombatEvent outcomes through the authenticated Godot client and owned UDP server.
use super::*;

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let (mut selected, mut remote) = (None, None);
    let (mut loading, mut stage) = (false, 0);
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::SoundOutcome)?;
        respond_to_selection(app, StartupScreen::SoundOutcome, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            observe_line(app, &line, selected, remote, &mut loading, &mut stage)?;
        }
        if let Some(status) = status {
            if status.success() && stage == 6 {
                println!(
                    "PASS: original CombatEvent outcomes, 65 UDP events once, ignored/unresolved, spatial cleanup"
                );
                return Ok(());
            }
            return Err(format!("outcome fixture exited {status} at stage {stage}"));
        }
        thread::sleep(TICK);
    }
    Err(format!("outcome fixture timed out at stage {stage}"))
}

fn observe_line(
    app: &mut App,
    line: &str,
    selected: Option<Entity>,
    remote: Option<Entity>,
    loading: &mut bool,
    stage: &mut u8,
) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("outcome fixture runtime error: {line}"));
    }
    let marker = line.trim();
    match (*stage, marker) {
        (0, "FIXTURE OUTCOME_LOADING") if selected.is_some() => {
            begin_outcome_loading(app, loading);
        }
        (0, "FIXTURE OUTCOME_READY") if *loading => {
            send_batch(app, selected, remote)?;
            *stage = 1;
        }
        (1, "FIXTURE OUTCOME_BATCH") => {
            send_outcome(app, selected, remote, CombatEventType::SpellHeal, 66.0)?;
            *stage = 2;
        }
        (2, "FIXTURE OUTCOME_MUTED") => {
            send_outcome(app, selected, remote, CombatEventType::SpellHeal, 67.0)?;
            send_outcome(app, selected, remote, CombatEventType::Interrupt, 68.0)?;
            *stage = 3;
        }
        (3, "FIXTURE OUTCOME_REMOVE") => {
            app.world_mut()
                .despawn(remote.ok_or("no remote to remove")?);
            *stage = 4;
        }
        (4, "FIXTURE OUTCOME_RESET") => {
            request_outcome_reset(app);
            *stage = 5;
        }
        (5, "FIXTURE OUTCOME_DONE") => *stage = 6,
        (_, marker) if marker.starts_with("FIXTURE OUTCOME_") => {
            return Err(format!(
                "out-of-order outcome marker at stage {stage}: {marker}"
            ));
        }
        _ => {}
    }
    Ok(())
}

fn begin_outcome_loading(app: &mut App, loading: &mut bool) {
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

fn request_outcome_reset(app: &mut App) {
    send::<_, AuthChannel>(
        app,
        shared::protocol::ForcedDisconnect {
            message: "fixture completed".into(),
            reconnect_allowed: false,
        },
    );
}

fn send_outcome(
    app: &mut App,
    selected: Option<Entity>,
    remote: Option<Entity>,
    event_type: CombatEventType,
    amount: f32,
) -> Result<(), String> {
    send::<_, CombatChannel>(
        app,
        CombatEvent {
            attacker: selected.ok_or("no local attacker")?.to_bits(),
            target: remote.ok_or("no remote target")?.to_bits(),
            amount,
            spell_id: 133,
            event_type,
        },
    );
    Ok(())
}

fn send_batch(
    app: &mut App,
    selected: Option<Entity>,
    remote: Option<Entity>,
) -> Result<(), String> {
    let attacker = selected
        .ok_or("no local player for outcome fixture")?
        .to_bits();
    let target = remote
        .ok_or("no remote player for outcome fixture")?
        .to_bits();
    send_original_outcomes(app, attacker, target);
    send_ignored_outcomes(app, attacker, target);
    Ok(())
}

fn send_original_outcomes(app: &mut App, attacker: u64, target: u64) {
    let kinds = [
        CombatEventType::SpellDamage,
        CombatEventType::SpellHeal,
        CombatEventType::Miss,
        CombatEventType::Interrupt,
    ];
    for index in 0..65 {
        send::<_, CombatChannel>(
            app,
            CombatEvent {
                attacker,
                target,
                amount: index as f32,
                spell_id: 133,
                event_type: kinds[index % kinds.len()].clone(),
            },
        );
    }
}

fn send_ignored_outcomes(app: &mut App, attacker: u64, target: u64) {
    for (kind, spell_id, emitter) in [
        (CombatEventType::Death, 133, target),
        (CombatEventType::SpellDamage, 0, target),
        (CombatEventType::SpellDamage, 133, u64::MAX),
    ] {
        send::<_, CombatChannel>(
            app,
            CombatEvent {
                attacker,
                target: emitter,
                amount: 10.0,
                spell_id,
                event_type: kind,
            },
        );
    }
}
