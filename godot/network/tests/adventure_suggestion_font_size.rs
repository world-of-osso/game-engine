//! Process proof against the real Godot Label and production truncation detector.
#[test]
#[ignore = "Requires a built debug GDExtension and explicit GODOT_BIN"]
fn adventure_suggestion_painted_font_size_truncation() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name the pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--headless")
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_suggestion_font_size.gd"])
        .output()
        .expect("start native FontString boundary fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 1/1"), "{stdout}\n{stderr}");
}
