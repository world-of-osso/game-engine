//! Opt-in real Godot/private-realm proof; ordinary CPU gates must not start a client.
#[test]
#[ignore = "Requires pinned GODOT_BIN, a private5771 realm and owned native display/profile"]
fn journal_model_scenes_use_authored_creature_camera_and_normalization() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_model_scenes.gd"])
        .output()
        .expect("run Journal authored-scene native fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 2/2"), "{stdout}\n{stderr}");
}
