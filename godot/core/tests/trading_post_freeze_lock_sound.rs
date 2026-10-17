use game_engine_core::ui_sound_kits::ui_sound_kit;

// Retail70391 SoundKit/Entry; installed audio provenance is recorded separately.
#[test]
fn trading_post_freeze_lock_kit_matches_70391_variants_and_gain() {
    let kit = ui_sound_kit(218229).expect("Trading Post accepted freeze-lock kit218229");
    assert_eq!(kit.volume, 0.707_945_76);
    assert_eq!(kit.files, [4908453, 4908455, 4908457, 4908459]);
    for (roll, file) in [4908453, 4908455, 4908457, 4908459].into_iter().enumerate() {
        assert_eq!(kit.file(roll as u32), file);
    }
    assert_eq!(kit.file(4), 4908453);
    assert_eq!(kit.file(u32::MAX), 4908459);
}
