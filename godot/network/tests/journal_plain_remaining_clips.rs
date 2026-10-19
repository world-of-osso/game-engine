//! Each requested source case runs in its own process; no failure hides later cases.
#[test]
#[ignore = "Requires pinned GODOT_BIN, private5771 realm and owned native display/profile"]
fn journal_remaining_plain_clips_select_source_without_sync_resets() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut failures = Vec::new();
    for creature in ["4311", "5204", "5932"] {
        let output = std::process::Command::new(&godot)
            .arg("--path")
            .arg(&project)
            .args(["-s", "res://tests/adventure_plain_remaining_clips.gd"])
            .env("JOURNAL_PLAIN_CLIP_CREATURE", creature)
            .output()
            .expect("run isolated Journal source clip process");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() || !stdout.contains("ALL_CASES_DONE 1/1") {
            failures.push(format!("creature{creature}: {stdout}\n{stderr}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
