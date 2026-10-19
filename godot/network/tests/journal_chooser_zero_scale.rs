//! Real Skeleton3D boundary: CPU-only engine, authentic authored model, no realm.
#[test]
#[ignore = "Requires pinned GODOT_BIN, native extension and authenticated 3049899 assets"]
fn authored_zero_scale_preserves_rotation_without_basis_errors() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--headless")
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/journal_chooser_zero_scale.gd"])
        .output()
        .expect("run CPU-only authored zero-scale bone fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let completed = stdout
        .lines()
        .filter(|line| *line == "ALL_CASES_DONE 1/1")
        .count();
    assert!(
        output.status.success() && completed == 1,
        "authored zero-scale bone fixture failed:\n{stdout}\n{stderr}"
    );
}
