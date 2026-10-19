//! Run one completion-contract GDScript; GODOT_BIN and output directory are explicit.
use game_engine_network::fixture_completion::{Completion, validate_output};
use std::{
    env, fs,
    path::PathBuf,
    process::{Command, ExitCode, Output},
};

const FIXTURES: &[(&str, usize)] = &[
    ("campsite_selected_hover", 2),
    ("communities_fidelity", 2),
    ("dropdown_caption_centering", 4),
    ("guild_control_fidelity", 2),
    ("guild_member_notes_live", 1),
    ("guild_tabard_fidelity", 2),
    ("house_access_fidelity", 2),
    ("hud_edit_controls_fidelity", 2),
    ("loot_item_hitarea", 2),
    ("minimap_zoom_fidelity", 6),
    ("pvp_ready_placement", 8),
    ("pvp_random_bonus", 4),
    ("ready_check_difficulty", 22),
    ("ready_check_portrait", 2),
    ("ready_check_title_bounds", 2),
    ("registration_flow", 2),
    ("achievement_search_selection_focus", 12),
];

struct Invocation {
    root: PathBuf,
    fixture: String,
    output: PathBuf,
    mode: String,
    expected: usize,
}

fn read_invocation() -> Result<Invocation, String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [root, fixture, output, mode] = args.as_slice() else {
        return Err(
            "usage: gdscript_completion_fixture ROOT FIXTURE OUTPUT --headless|--window|--check"
                .into(),
        );
    };
    let expected = FIXTURES
        .iter()
        .find(|(name, _)| name == fixture)
        .map(|(_, expected)| *expected)
        .ok_or_else(|| format!("unknown fixture {fixture}"))?;
    if !["--headless", "--window", "--check"].contains(&mode.as_str()) {
        return Err(format!("unknown mode {mode}"));
    }
    Ok(Invocation {
        root: root.into(),
        fixture: fixture.clone(),
        output: output.into(),
        mode: mode.clone(),
        expected,
    })
}

fn spawn_godot(invocation: &Invocation) -> Result<Output, String> {
    let godot = env::var_os("GODOT_BIN").ok_or("GODOT_BIN required")?;
    // Absolute duration bound, even when an awaited coroutine never resumes.
    let mut command = Command::new("timeout");
    command.args(["--kill-after=5s", "910s"]).arg(godot);
    if invocation.mode != "--window" {
        command.arg("--headless");
    }
    command
        .arg("--path")
        .arg(invocation.root.join("godot"))
        .args([
            "--script",
            &format!("res://tests/{}.gd", invocation.fixture),
        ]);
    if invocation.mode == "--check" {
        command.arg("--check-only");
    }
    command
        .output()
        .map_err(|error| format!("spawn Godot for {}: {error}", invocation.fixture))
}

fn write_streams(invocation: &Invocation, result: &Output) -> Result<(), String> {
    fs::create_dir_all(&invocation.output).map_err(|error| error.to_string())?;
    for (stream, bytes) in [("stdout", &result.stdout), ("stderr", &result.stderr)] {
        let path = invocation
            .output
            .join(format!("{}.{stream}.log", invocation.fixture));
        fs::write(&path, bytes).map_err(|error| format!("write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn validate_parse(result: &Output) -> Result<(), String> {
    let mut diagnostics = Completion::new(0);
    for bytes in [&result.stdout, &result.stderr] {
        for line in String::from_utf8_lossy(bytes).lines() {
            diagnostics.observe(line)?;
        }
    }
    if !result.status.success() {
        return Err(format!("GDScript parse exited {}", result.status));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let invocation = read_invocation()?;
    let result = spawn_godot(&invocation)?;
    write_streams(&invocation, &result)?;
    if invocation.mode == "--check" {
        validate_parse(&result)?;
        println!("PARSE_ONLY {}; not behavior acceptance", invocation.fixture);
    } else {
        validate_output(&result, invocation.expected)?;
        println!(
            "ACCEPTED {} {1}/{1}",
            invocation.fixture, invocation.expected
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL {error}");
            ExitCode::FAILURE
        }
    }
}
