//! Opt-in source-refusal/painted defaults proof against a real private realm.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 realm and owned native display/profile"]
fn journal_zero_actor_displays_use_source_defaults_and_paint_real_models() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_zero_actor_display.gd"])
        .output()
        .expect("run Journal zero-display native fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 3/3"), "{stdout}\n{stderr}");
}
