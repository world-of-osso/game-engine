use game_engine_core::ui_sound_kits::ui_sound_kit;

// Retail SoundKitConstants.lua and SoundKit/Entry70391; audio bytes have separate provenance.
#[test]
fn trading_post_product_hover_kit_matches_70391_file_and_gain() {
    let kit = ui_sound_kit(218934).expect("Trading Post product hover kit218934");
    assert_eq!(kit.volume, 0.891_250_9);
    assert_eq!(kit.files, [4906953]);
    assert_eq!(kit.file(0), 4906953);
    assert_eq!(kit.file(u32::MAX), 4906953);
}
