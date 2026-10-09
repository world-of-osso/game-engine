use game_engine_core::client_options_data::{
    ClientOptionsFile, ExtraActionBars, load_options_file_from_path, save_options_file_to_path,
};

#[test]
fn extra_action_bars_defaults_and_settings_store_round_trip() {
    let path = std::env::temp_dir().join(format!("extrabars-options-{}.ron", std::process::id()));
    let mut file = ClientOptionsFile::default();
    assert_eq!(file.hud.extra_action_bars, ExtraActionBars::default());
    assert!(!file.hud.extra_action_bars.action_bar_4);
    assert!(!file.hud.extra_action_bars.action_bar_5);
    file.hud.extra_action_bars = ExtraActionBars {
        action_bar_2: Some(false),
        action_bar_3: Some(true),
        action_bar_4: true,
        action_bar_5: true,
    };
    file.hud.auto_loot = true;
    save_options_file_to_path(&path, &file).unwrap();
    let restored = load_options_file_from_path(&path);
    assert_eq!(restored.hud.extra_action_bars, file.hud.extra_action_bars);
    assert!(restored.hud.auto_loot);
    let old_hud = r#"(show_minimap:true,show_action_bars:true,show_nameplates:true,show_health_bars:true,show_target_marker:true,show_fps_overlay:false)"#;
    let restored: game_engine_core::client_options_data::HudOptionsFile =
        ron::from_str(old_hud).unwrap();
    assert_eq!(restored.extra_action_bars, ExtraActionBars::default());
    std::fs::remove_file(path).unwrap();
}
