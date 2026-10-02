//! Isolated real-input shore crossing; shares the parent fixture's UDP server and launcher.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shore {
    DryForward,
    WetForward,
    WetLeft,
    WetRight,
    WetBackward,
    DryBackward,
}

impl Shore {
    fn from_input(input: &PlayerInput) -> Result<Self, String> {
        if !input.position.iter().all(|axis| axis.is_finite())
            || !input.facing_yaw.is_finite()
            || !input.running
            || input.jumping
        {
            return Err(format!("invalid swimming fixture PlayerInput: {input:?}"));
        }
        let forward = Directional::Walk.expected_vector(input.facing_yaw);
        let backward = Directional::Backward.expected_vector(input.facing_yaw);
        let left = Directional::Left.expected_vector(input.facing_yaw);
        let right = Directional::Right.expected_vector(input.facing_yaw);
        let matches = |expected: [f32; 3]| {
            input
                .direction
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.is_finite() && (actual - expected).abs() < 0.15)
        };
        match (
            input.swimming,
            matches(forward),
            matches(backward),
            matches(left),
            matches(right),
        ) {
            (false, true, false, false, false) => Ok(Self::DryForward),
            (true, true, false, false, false) => Ok(Self::WetForward),
            (true, false, false, true, false) => Ok(Self::WetLeft),
            (true, false, false, false, true) => Ok(Self::WetRight),
            (true, false, true, false, false) => Ok(Self::WetBackward),
            (false, false, true, false, false) => Ok(Self::DryBackward),
            _ => Err(format!("unexpected shore direction/swim flags: {input:?}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Loading,
    World,
    Forward,
    Idle,
    SpaceIdle,
    SpaceForward,
    Left,
    LeftIdle,
    Right,
    RightIdle,
    Backward,
    Dry,
    Done,
}

struct FixtureProgress {
    selected: Option<Entity>,
    remote: Option<Entity>,
    stage: Stage,
    shore: Option<Shore>,
    saw_space_forward: bool,
    saw_left: bool,
    saw_right: bool,
    released_at: Option<Instant>,
}

impl FixtureProgress {
    fn new() -> Self {
        Self {
            selected: None,
            remote: None,
            stage: Stage::Loading,
            shore: None,
            saw_space_forward: false,
            saw_left: false,
            saw_right: false,
            released_at: None,
        }
    }
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: ClientLines,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut progress = FixtureProgress::new();
    let mut readers = Some(readers);
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::Swimming)?;
        respond_to_selection(
            app,
            StartupScreen::Swimming,
            &mut progress.selected,
            &mut progress.remote,
        )?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        join_readers_on_exit(status.as_ref(), &mut readers)?;
        consume_fixture_lines(app, &lines, &mut progress)?;
        check_decoded_inputs(app, &mut progress)?;
        if finish_on_exit(app, status, &progress)? {
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "timed out waiting for swimming fixture at {:?}; decoded shore={:?}",
        progress.stage, progress.shore
    ))
}

fn join_readers_on_exit(
    status: Option<&std::process::ExitStatus>,
    readers: &mut Option<Vec<thread::JoinHandle<()>>>,
) -> Result<(), String> {
    if status.is_some() {
        for reader in readers.take().expect("join output once") {
            reader.join().map_err(|_| "Godot output reader panicked")?;
        }
    }
    Ok(())
}

fn consume_fixture_lines(
    app: &mut App,
    lines: &ClientLines,
    progress: &mut FixtureProgress,
) -> Result<(), String> {
    for line in lines.after_selection(progress.selected.is_some()) {
        let line = line.trim();
        if line.starts_with("GODOT_STDERR: ERROR:")
            || line.starts_with("GODOT_STDERR: SCRIPT ERROR:")
        {
            return Err(format!("Godot swimming runtime error: {line}"));
        }
        advance_fixture_marker(app, line, progress)?;
    }
    Ok(())
}

fn advance_fixture_marker(
    app: &mut App,
    line: &str,
    progress: &mut FixtureProgress,
) -> Result<(), String> {
    match (progress.stage, line) {
        (Stage::Loading, "FIXTURE LOADING_OBSERVED") => {
            if !take_inputs(app).is_empty() {
                return Err("Loading sent input".into());
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
        (Stage::World, "FIXTURE SWIM_WORLD_READY") => progress.stage = Stage::Forward,
        (Stage::Forward, "FIXTURE SWIM_RELEASED") => {
            progress.released_at = Some(Instant::now());
            progress.stage = Stage::Idle;
        }
        (Stage::Idle, "FIXTURE SWIM_IDLE_DONE") => {
            ensure_quiet(app, progress.released_at, "swim idle")?;
            progress.stage = Stage::SpaceIdle;
        }
        (Stage::SpaceIdle, "FIXTURE SWIM_SPACE_IDLE_DONE") => {
            progress.stage = Stage::SpaceForward;
        }
        (Stage::SpaceForward, "FIXTURE SWIM_SPACE_FORWARD_DONE") => {
            if !progress.saw_space_forward {
                return Err("no decoded wet W+Space packets".into());
            }
            progress.stage = Stage::Left;
        }
        (Stage::Left, "FIXTURE SWIM_LEFT_START") => {}
        (Stage::Left, "FIXTURE SWIM_LEFT_END") => {
            progress.released_at = Some(Instant::now());
            progress.stage = Stage::LeftIdle;
        }
        (Stage::LeftIdle, "FIXTURE SWIM_LEFT_IDLE_DONE") => {
            ensure_quiet(app, progress.released_at, "swim left release")?;
            if !progress.saw_left {
                return Err("no decoded wet A/SwimLeft packets".into());
            }
            progress.stage = Stage::Right;
        }
        (Stage::Right, "FIXTURE SWIM_RIGHT_START") => {}
        (Stage::Right, "FIXTURE SWIM_RIGHT_END") => {
            progress.released_at = Some(Instant::now());
            progress.stage = Stage::RightIdle;
        }
        (Stage::RightIdle, "FIXTURE SWIM_RIGHT_IDLE_DONE") => {
            ensure_quiet(app, progress.released_at, "swim right release")?;
            if !progress.saw_right {
                return Err("no decoded wet D/SwimRight packets".into());
            }
        }
        (Stage::RightIdle, "FIXTURE SWIM_BACKWARD_START") => progress.stage = Stage::Backward,
        (Stage::Backward, "FIXTURE SWIM_BACKWARD_RELEASED") => {
            progress.released_at = Some(Instant::now());
            progress.stage = Stage::Dry;
        }
        (Stage::Dry, "FIXTURE SWIM_DONE") => {
            ensure_quiet(app, progress.released_at, "dry stand")?;
            progress.stage = Stage::Done;
        }
        (_, line) if line.starts_with("FIXTURE ") => {
            return Err(format!(
                "out-of-order swim stage {:?}: {line}",
                progress.stage
            ));
        }
        _ => {}
    }
    Ok(())
}

fn check_decoded_inputs(app: &mut App, progress: &mut FixtureProgress) -> Result<(), String> {
    for input in take_inputs(app) {
        check_decoded_input(&input, progress)?;
    }
    Ok(())
}

fn check_decoded_input(input: &PlayerInput, progress: &mut FixtureProgress) -> Result<(), String> {
    let stage = progress.stage;
    if matches!(stage, Stage::Loading | Stage::World) {
        return Err(format!("input before native world readiness: {input:?}"));
    }
    if matches!(
        stage,
        Stage::Idle
            | Stage::SpaceIdle
            | Stage::LeftIdle
            | Stage::RightIdle
            | Stage::Dry
            | Stage::Done
    ) {
        if progress
            .released_at
            .is_some_and(|at| at.elapsed() >= RELEASE_DRAIN)
        {
            return Err(format!(
                "input after key release during {stage:?}: {input:?}"
            ));
        }
        // In-flight packets from the preceding held key still count toward the
        // ordered shore crossing; they are never discarded.
    }
    let current = Shore::from_input(input)?;
    if (current == Shore::WetLeft && !matches!(stage, Stage::Left | Stage::LeftIdle | Stage::Right))
        || (current == Shore::WetRight
            && !matches!(stage, Stage::Right | Stage::RightIdle | Stage::Backward))
    {
        return Err(format!(
            "lateral input outside held/release stage {stage:?}: {input:?}"
        ));
    }
    let step = match current {
        Shore::DryForward => 0,
        Shore::WetForward => 1,
        Shore::WetLeft => 2,
        Shore::WetRight => 3,
        Shore::WetBackward => 4,
        Shore::DryBackward => 5,
    };
    let shore = progress.shore;
    let previous = shore.map(|state| match state {
        Shore::DryForward => 0,
        Shore::WetForward => 1,
        Shore::WetLeft => 2,
        Shore::WetRight => 3,
        Shore::WetBackward => 4,
        Shore::DryBackward => 5,
    });
    if step > previous.map_or(0, |index| index + 1) || previous.is_some_and(|index| step < index) {
        return Err(format!(
            "out-of-order decoded shore flags: {shore:?} -> {current:?}; {input:?}"
        ));
    }
    if stage == Stage::SpaceForward && current == Shore::WetForward {
        progress.saw_space_forward = true;
    }
    progress.saw_left |= current == Shore::WetLeft;
    progress.saw_right |= current == Shore::WetRight;
    progress.shore = Some(current);
    Ok(())
}

fn finish_on_exit(
    app: &App,
    status: Option<std::process::ExitStatus>,
    progress: &FixtureProgress,
) -> Result<bool, String> {
    let Some(status) = status else {
        return Ok(false);
    };
    if !status.success()
        || progress.stage != Stage::Done
        || progress.shore != Some(Shore::DryBackward)
    {
        return Err(format!(
            "Godot exited {status} at {:?}; decoded shore={:?}",
            progress.stage, progress.shore
        ));
    }
    ensure_quiet(app, progress.released_at, "final dry stand")?;
    println!(
        "PASS: real shore W dry->wet, wet W+Space, A SwimLeft43, D SwimRight44, reverse S wet->dry; no jumping; released UDP quiet"
    );
    Ok(true)
}

fn ensure_quiet(app: &App, released_at: Option<Instant>, stage: &str) -> Result<(), String> {
    if !released_at.is_some_and(|at| at.elapsed() >= RELEASE_DRAIN + RELEASE_QUIET) {
        return Err(format!("{stage} ended before bounded UDP quiet interval"));
    }
    ensure_release_reported(app, stage)
}
