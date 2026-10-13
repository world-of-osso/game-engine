//! Native controller boundary: Retail scrollbar is a sibling of its clipped ScrollBox.
use super::super::{RegistryModel, ScreenPostsetup, ui_parent::UiParent};
use super::{drag_thumbs, press_thumb, release_thumbs, tests as scroll_lists_tests};
use game_engine_ui_model::raid_info::{
    RaidInfoState, SCROLL_BOX, SavedInstanceRow, screen as raid_info_screen,
};
use shared::protocol::InstanceLockInfo;
use ui_toolkit::{
    screen::{Screen, SharedContext},
    widgets::scroll_list::{thumb_name, track_name},
};

#[test]
fn raid_sibling_scrollbar_thumb_drag_reaches_eighth_saved_instance() {
    game_engine_ui_model::paths::set_data_root(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
    let rows = [
        (409, 9, "Molten Core"),
        (469, 9, "Blackwing Lair"),
        (533, 3, "Naxxramas"),
        (249, 3, "Onyxia's Lair"),
        (532, 3, "Karazhan"),
        (565, 4, "Gruul's Lair"),
        (544, 4, "Magtheridon's Lair"),
        (580, 4, "Sunwell Plateau"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (map_id, difficulty_id, name))| SavedInstanceRow {
        lock: InstanceLockInfo {
            map_id,
            difficulty_id,
            instance_id: i as u32 + 1,
            time_remaining_secs: 4000,
            completed_mask: 0,
            locked: true,
            extended: false,
        },
        name: name.into(),
        difficulty: "Normal".into(),
        extend_disabled: false,
    })
    .collect();
    let mut shared = SharedContext::new();
    shared.insert(RaidInfoState {
        visible: true,
        rows,
        ..Default::default()
    });
    let mut model = RegistryModel {
        screen: Screen::new(raid_info_screen),
        shared,
        registry: UiParent::for_viewport(1920.0, 1080.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    scroll_lists_tests::rebuild(&mut model);
    let thumb = model.registry.get_by_name(&thumb_name(SCROLL_BOX)).unwrap();
    let thumb_rect = model
        .registry
        .get(thumb)
        .unwrap()
        .layout_rect
        .clone()
        .unwrap();
    let track = scroll_lists_tests::rect(&model, &track_name(SCROLL_BOX)).unwrap();
    assert!(
        press_thumb(
            &mut model.registry,
            thumb,
            thumb_rect.y + thumb_rect.height / 2.0
        ),
        "Sibling scrollbar must control its associated ScrollBox"
    );
    assert!(drag_thumbs(&mut model.registry, track.y + track.height));
    assert!(release_thumbs(&mut model.registry));
    scroll_lists_tests::rebuild(&mut model);
    assert_eq!(
        model
            .registry
            .scroll_lists
            .get(SCROLL_BOX)
            .unwrap()
            .first_row,
        163
    );
    let last = scroll_lists_tests::rect(&model, "RaidInfoRow7").unwrap();
    let clip = scroll_lists_tests::rect(&model, SCROLL_BOX).unwrap();
    assert!(last.y >= clip.y && last.y + last.height <= clip.y + clip.height);
}
