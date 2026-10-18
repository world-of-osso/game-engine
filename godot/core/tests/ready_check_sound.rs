use game_engine_core::ui_sound_kits::ui_sound_kit;

#[test]
fn ready_check_sound_uses_active_70448_clip_and_gain() {
    let kit = ui_sound_kit(8960).expect("ReadyCheck OnShow SoundKit8960");
    assert_eq!(kit.volume, 1.0);
    assert_eq!(kit.files, [567478]);
    assert_eq!(kit.file(0), 567478);
    assert_eq!(kit.file(u32::MAX), 567478);
}
