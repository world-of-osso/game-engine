//! Opt-in native offline-preview inventory, not part of normal launcher tests.
#[test]
#[ignore = "requires staged native extension and GODOT_BIN; offline font/layout audit"]
fn buttonfit_all_native_previews() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN");
    let status = std::process::Command::new(godot)
        .args(["--headless", "--path"])
        .arg(root.join("godot"))
        .args([
            "--resolution",
            "1920x1080",
            "--script",
            "res://tests/buttonfit_audit.gd",
        ])
        .env("GODOT_BUTTONFIT_AUDIT", "1")
        .env("GODOT_AUCTION_VIEW", "browse")
        .status()
        .expect("launch offline audit");
    assert!(status.success(), "native buttonfit audit: {status}");
}
