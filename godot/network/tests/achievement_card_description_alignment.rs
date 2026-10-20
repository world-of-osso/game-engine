//! Ignored both-skin native RED precedes the ordinary Description alignment correction.
#[test]
#[ignore = "Requires a built debug GDExtension and explicit pinned GODOT_BIN"]
fn achievement_card_description_alignment_native() {
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN must name the pinned Godot");
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = std::process::Command::new("timeout")
        .args(["--kill-after=5s", "180s"])
        .arg(godot)
        .arg("--headless")
        .arg("--path")
        .arg(project)
        .args([
            "-s",
            "res://tests/achievement_card_description_alignment.gd",
        ])
        .output()
        .expect("start native achievement Description alignment fixture");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    game_engine_network::fixture_completion::validate_output(&output, 6)
        .expect("both-skin native Description alignment completion contract");
}
