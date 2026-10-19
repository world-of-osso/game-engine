//! Read-only native diagnostic; a passing diagnostic does not fix the painted gap.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 realm and owned native display/profile"]
fn journal_raszageth_readonly_frustum_keeps_the_authored_paint_gap() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/adventure_raszageth_frustum.gd"])
        .output()
        .expect("run read-only Raszageth projection diagnostic");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("ALL_CASES_DONE 1/1"), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("JOURNAL_RASZAGETH_TOP_REJECTION_CONFIRMED product_gap_not_fixed"),
        "{stdout}\n{stderr}"
    );
}
