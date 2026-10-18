//! Run one completion-contract GDScript; GODOT_BIN and output directory are explicit.
use game_engine_network::fixture_completion::validate_output;
use std::{
    env, fs,
    path::PathBuf,
    process::{Command, ExitCode},
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
];

fn run() -> Result<(), String> {
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
    let output = PathBuf::from(output);
    fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    let project = PathBuf::from(root).join("godot");
    let godot = env::var_os("GODOT_BIN").ok_or("GODOT_BIN required")?;
    let mut command = Command::new(godot);
    if mode != "--window" {
        command.arg("--headless");
    }
    command
        .arg("--path")
        .arg(&project)
        .args(["--script", &format!("res://tests/{fixture}.gd")]);
    if mode == "--check" {
        command.arg("--check-only");
    }
    // Absolute duration bound, even when an awaited coroutine never resumes.
    let mut bounded = Command::new("timeout");
    bounded
        .arg("--kill-after=5s")
        .arg("310s")
        .arg(command.get_program())
        .args(command.get_args());
    let result = bounded.output().map_err(|error| error.to_string())?;
    fs::write(output.join(format!("{fixture}.stdout.log")), &result.stdout)
        .map_err(|error| error.to_string())?;
    fs::write(output.join(format!("{fixture}.stderr.log")), &result.stderr)
        .map_err(|error| error.to_string())?;
    if mode == "--check" {
        if !result.status.success()
            || [&result.stdout, &result.stderr].iter().any(|bytes| {
                let text = String::from_utf8_lossy(bytes);
                text.contains("SCRIPT ERROR") || text.contains("ERROR:")
            })
        {
            return Err(format!(
                "GDScript parse check failed: {fixture} ({})",
                result.status
            ));
        }
        println!("PARSE_ONLY {fixture}; not behavior acceptance");
        return Ok(());
    }
    validate_output(&result, expected)?;
    println!("ACCEPTED {fixture} {expected}/{expected}");
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
