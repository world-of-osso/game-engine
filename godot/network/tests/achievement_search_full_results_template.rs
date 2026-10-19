//! Both-skin native RED is published before full-search presentation implementation.
#[test]
#[ignore = "Requires a built debug GDExtension and explicit pinned GODOT_BIN"]
fn achievement_search_full_results_template_native() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name the pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new("timeout")
        .args(["--kill-after=5s", "180s"])
        .arg(godot)
        .arg("--headless")
        .arg("--path")
        .arg(project)
        .args(["-s", "res://tests/achievement_search_full_results.gd"])
        .output()
        .expect("start native achievement full-search fixture");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    game_engine_network::fixture_completion::validate_output(&output, 8)
        .expect("both-skin native full-search completion contract");
}
