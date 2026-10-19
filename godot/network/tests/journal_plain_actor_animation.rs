//! Opt-in native source AnimationID25/current-ID and progressing phase boundary.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 realm and owned native display/profile"]
fn journal_plain_actor_animation_selects_source_clip_without_sync_resets() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_plain_actor_animation.gd"])
        .output()
        .expect("run Journal plain authored animation fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 2/2"), "{stdout}\n{stderr}");
}
