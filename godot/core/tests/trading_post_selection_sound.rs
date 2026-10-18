use game_engine_core::ui_sound_kits::ui_sound_kit;

#[test]
fn trading_post_selection_sound_kit_matches_active_70448_file_and_gain() {
    let kit = ui_sound_kit(218230).expect("Retail product-selection kit218230");
    assert_eq!(kit.volume, 0.790_000_021_f32);
    assert_eq!(kit.files, [4906955]);
    assert_eq!(kit.file(0), 4906955);
    assert_eq!(kit.file(u32::MAX), 4906955);
}
