//! Opt-in native production-primary title fit and physical hover boundary.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 realm and owned native display/profile"]
fn journal_primary_title_hover_uses_authentic_truncated_and_fitting_source_rows() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_primary_title_hover.gd"])
        .output()
        .expect("run native primary-title fit and hover fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 4/4"), "{stdout}\n{stderr}");
}
