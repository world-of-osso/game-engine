use game_engine_core::ui_sound_kits::ui_sound_kit;

// Cached Retail SoundKitConstants.lua and installed12.1.5.70391 SoundKit/Entry.
#[test]
fn trading_post_menu_open_kit_matches_70391_files_and_gain() {
    let kit = ui_sound_kit(218932).expect("Trading Post menu open kit218932");
    assert_eq!(kit.volume, 0.562_341_33);
    assert_eq!(kit.files, [4956048, 4956050, 4956052, 4956054, 4956056]);
    for (index, fdid) in kit.files.iter().enumerate() {
        assert_eq!(kit.file(index as u32), *fdid);
    }
    assert_eq!(kit.file(5), 4956048);
}

#[test]
fn trading_post_menu_close_kit_matches_70391_files_and_gain() {
    let kit = ui_sound_kit(218933).expect("Trading Post menu close kit218933");
    assert_eq!(kit.volume, 0.01);
    assert_eq!(
        kit.files,
        [
            4555731, 4555733, 4555735, 4555737, 4555739, 4555741, 4555743, 4555745, 4555747,
            4555749
        ]
    );
    for (index, fdid) in kit.files.iter().enumerate() {
        assert_eq!(kit.file(index as u32), *fdid);
    }
    assert_eq!(kit.file(10), 4555731);
}
