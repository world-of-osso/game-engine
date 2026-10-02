//! Authored logout decisions driven by replicated combat and real UDP rest/input state.
use super::*;
use shared::{
    components::CombatStatus,
    protocol::{RestChannel, RestSnapshot, RestStateUpdate},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Loading,
    World,
    Combat,
    Blocked,
    ClearCombat,
    Rest,
    ClearRest,
    Countdown,
    Cancelled,
    Repeated,
    Expired,
    LoginQuiet,
    LiveConnection,
    Relogin,
    Reloading,
    RestFinal,
    Done,
}

struct Progress {
    stage: Stage,
    selected: Option<Entity>,
    remote: Option<Entity>,
    saw_cancel_input: bool,
    saw_token_login: bool,
    saved_token: Option<Vec<u8>>,
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
    root: &Path,
    address: SocketAddr,
) -> Result<(), String> {
    let mut progress = Progress {
        stage: Stage::Loading,
        selected: None,
        remote: None,
        saw_cancel_input: false,
        saw_token_login: false,
        saved_token: None,
    };
    // Production persists tokens under data; the endpoint-keyed filename belongs only to this fixture.
    let token_path = root.join(format!(
        "data/auth_token.{}",
        address.to_string().replace(':', "_")
    ));
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        check_login_requests(app, &mut progress)?;
        respond_to_selection(
            app,
            StartupScreen::Logout,
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
            advance_marker(app, &mut progress, line.trim(), &token_path)?;
        }
        check_inputs(app, &mut progress)?;
        if let Some(status) = status {
            if !status.success()
                || progress.stage != Stage::Done
                || !progress.saw_cancel_input
                || !progress.saw_token_login
            {
                return Err(format!(
                    "Godot exited {status} at {:?}; cancellation input={} token relogin={}",
                    progress.stage, progress.saw_cancel_input, progress.saw_token_login
                ));
            }
            verify_token(&token_path, progress.saved_token.as_deref())?;
            fs::remove_file(&token_path)
                .map_err(|error| format!("Remove fixture-only saved token: {error}"))?;
            println!(
                "PASS: replicated combat/rest, cancelled and retained 20s countdown, Login world/connection and token relogin over UDP"
            );
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "Timed out waiting for native logout at {:?}",
        progress.stage
    ))
}

fn check_login_requests(app: &mut App, progress: &mut Progress) -> Result<(), String> {
    let requests = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().logins);
    for (connection, request) in requests {
        let credentials = request.username == "fixture"
            && request.password == "fixture"
            && request.token.is_none();
        let token = request.username.is_empty()
            && request.password.is_empty()
            && request.token.as_deref() == Some("fixture-only-token");
        if progress.stage == Stage::Loading && credentials {
            // Initial startup still uses the original fixture credentials.
        } else if matches!(progress.stage, Stage::LiveConnection | Stage::Relogin) && token {
            progress.saw_token_login = true;
        } else {
            return Err(format!(
                "Unexpected authentication in {:?}: token={}, credentials={}",
                progress.stage, token, credentials
            ));
        }
        let mut incoming = app.world_mut().resource_mut::<Incoming>();
        incoming.logins.push((connection, request));
        respond_to_login(app, StartupScreen::Logout)?;
    }
    Ok(())
}

fn send_rest(app: &mut App, snapshot: Option<RestSnapshot>) {
    send::<_, RestChannel>(
        app,
        RestStateUpdate {
            snapshot,
            message: None,
            error: None,
        },
    );
}

fn verify_token(path: &Path, expected: Option<&[u8]>) -> Result<(), String> {
    let actual = fs::read(path)
        .map_err(|error| format!("Read fixture-only saved token {}: {error}", path.display()))?;
    if expected != Some(actual.as_slice()) || actual != b"fixture-only-token" {
        return Err("Saved token changed across logout".into());
    }
    Ok(())
}

fn advance_marker(
    app: &mut App,
    progress: &mut Progress,
    line: &str,
    token_path: &Path,
) -> Result<(), String> {
    if line.starts_with("GODOT_STDERR: ERROR:") || line.starts_with("GODOT_STDERR: SCRIPT ERROR:") {
        return Err(format!("Godot logout runtime error: {line}"));
    }
    match (progress.stage, line) {
        (Stage::Loading, "FIXTURE LOGOUT_LOADING") => {
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
        (Stage::World, "FIXTURE LOGOUT_COMBAT_TRUE") => {
            let player = progress
                .selected
                .ok_or("Combat update before player selection")?;
            app.world_mut()
                .entity_mut(player)
                .insert(CombatStatus(true));
            progress.stage = Stage::Combat;
        }
        (Stage::Combat, "FIXTURE LOGOUT_COMBAT_BLOCKED") => progress.stage = Stage::Blocked,
        (Stage::Blocked, "FIXTURE LOGOUT_COMBAT_FALSE") => {
            app.world_mut()
                .entity_mut(progress.selected.ok_or("Player missing")?)
                .insert(CombatStatus(false));
            progress.stage = Stage::ClearCombat;
        }
        (Stage::ClearCombat, "FIXTURE LOGOUT_REST_TRUE") => {
            send_rest(
                app,
                Some(RestSnapshot {
                    in_rest_area: true,
                    rest_area_kind: None,
                    rested_xp: 42,
                    rested_xp_max: 100,
                }),
            );
            progress.stage = Stage::Rest;
        }
        (Stage::Rest, "FIXTURE LOGOUT_REST_NONE") => {
            send_rest(app, None);
            progress.stage = Stage::ClearRest;
        }
        (Stage::ClearRest, "FIXTURE LOGOUT_COUNTDOWN") => {
            let saved = fs::read(token_path).map_err(|error| {
                format!(
                    "Read fixture-only saved token {}: {error}",
                    token_path.display()
                )
            })?;
            if saved != b"fixture-only-token" {
                return Err("Unexpected fixture-only saved token".into());
            }
            progress.saved_token = Some(saved);
            progress.stage = Stage::Countdown;
        }
        (Stage::Countdown, "FIXTURE LOGOUT_CANCELLED") if progress.saw_cancel_input => {
            progress.stage = Stage::Cancelled
        }
        (Stage::Cancelled, "FIXTURE LOGOUT_REPEATED") => progress.stage = Stage::Repeated,
        (Stage::Repeated, "FIXTURE LOGOUT_EXPIRED") => {
            verify_token(token_path, progress.saved_token.as_deref())?;
            progress.stage = Stage::Expired;
        }
        (Stage::Expired, "FIXTURE LOGOUT_LOGIN_QUIET") => {
            let remote = progress.remote.ok_or("Post-logout remote player missing")?;
            app.world_mut()
                .get_mut::<Position>(remote)
                .ok_or("Post-logout remote player has no position")?
                .x += 4.0;
            progress.stage = Stage::LoginQuiet;
        }
        (Stage::LoginQuiet, "FIXTURE LOGOUT_LIVE_CONNECTION") => {
            progress.stage = Stage::LiveConnection;
        }
        (Stage::LiveConnection, "FIXTURE LOGOUT_RELOGIN") => {
            if let Some(player) = progress.selected.take() {
                app.world_mut().despawn(player);
            }
            if let Some(remote) = progress.remote.take() {
                app.world_mut().despawn(remote);
            }
            progress.stage = Stage::Relogin;
        }
        (Stage::Relogin, "FIXTURE LOGOUT_RELOADING") if progress.saw_token_login => {
            send::<_, TerrainChannel>(
                app,
                LoadTerrain {
                    map_name: "azeroth".into(),
                    initial_tile_y: 32,
                    initial_tile_x: 48,
                },
            );
            progress.stage = Stage::Reloading;
        }
        (Stage::Reloading, "FIXTURE LOGOUT_REST_TRUE_FINAL") => {
            send_rest(
                app,
                Some(RestSnapshot {
                    in_rest_area: true,
                    rest_area_kind: None,
                    rested_xp: 42,
                    rested_xp_max: 100,
                }),
            );
            progress.stage = Stage::RestFinal;
        }
        (Stage::RestFinal, "FIXTURE LOGOUT_DONE") => progress.stage = Stage::Done,
        (_, line) if line.starts_with("FIXTURE LOGOUT_") => {
            return Err(format!(
                "Out-of-order logout marker {:?}: {line}",
                progress.stage
            ));
        }
        _ => {}
    }
    Ok(())
}

fn check_inputs(app: &mut App, progress: &mut Progress) -> Result<(), String> {
    let inputs = take_inputs(app);
    if progress.stage == Stage::Countdown {
        progress.saw_cancel_input |= assert_forward_input(inputs)?;
    } else if !inputs.is_empty() {
        return Err(format!(
            "Unexpected decoded PlayerInput during {:?}: {inputs:?}",
            progress.stage
        ));
    }
    Ok(())
}
