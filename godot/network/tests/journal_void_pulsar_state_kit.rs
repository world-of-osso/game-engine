//! Genuine Journal selection exercises detached CDI effects, not a fake Replica unit.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 fb realm and owned profile/display"]
fn journal_void_pulsar_detached_state_kit_children_and_lifetime() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_void_pulsar_state_kit.gd"])
        .output()
        .expect("run genuine detached Journal state-kit fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let completed = stdout
        .lines()
        .filter(|line| *line == "ALL_CASES_DONE 1/1")
        .count();
    assert!(
        output.status.success() && completed == 1,
        "Journal state-kit fixture failed:\n{stdout}\n{stderr}"
    );
}
