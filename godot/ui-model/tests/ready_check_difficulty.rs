use game_engine_ui_model::{
    ready_check_difficulty::ReadyCheckDifficulties,
    ready_check_frame_component::{ReadyCheckFrameState, ready_check_frame},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
};

fn catalog() -> ReadyCheckDifficulties {
    ReadyCheckDifficulties::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap()
}

#[test]
fn ready_check_difficulty_uses_current_name_not_toggle_target_or_selection() {
    let catalog = catalog();
    for (id, name) in [
        (3, "10 Player"),
        (4, "25 Player"),
        (5, "10 Player (Heroic)"),
        (6, "25 Player (Heroic)"),
    ] {
        assert_eq!(
            catalog
                .listener_name(Some(id), Some(14))
                .unwrap()
                .as_deref(),
            Some(name)
        );
        assert_eq!(
            catalog.listener_name(Some(0), Some(id)).unwrap().as_deref(),
            Some(name)
        );
    }
    // Active modern raid overrides a toggle-capable saved selection too.
    assert_eq!(catalog.listener_name(Some(14), Some(3)).unwrap(), None);
}

#[test]
fn ready_check_difficulty_omits_suffix_for_modern_raid_dungeon_and_no_selection() {
    let catalog = catalog();
    for id in [1, 2, 8, 14, 15, 16, 23] {
        assert_eq!(catalog.listener_name(Some(0), Some(id)).unwrap(), None);
    }
    assert_eq!(catalog.listener_name(None, None).unwrap(), None);
    assert!(
        catalog
            .listener_name(Some(u32::MAX), Some(3))
            .unwrap_err()
            .contains("missing difficulty")
    );
}

fn screen(shared: &SharedContext) -> Element {
    ready_check_frame(shared.get::<ReadyCheckFrameState>().unwrap())
}

#[test]
fn ready_check_difficulty_projects_two_lines_without_changing_body_bounds_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        let mut state = ReadyCheckFrameState {
            visible: true,
            initiator: "Ann".into(),
            difficulty_name: None,
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(screen);
        for name in [None, Some("10 Player"), Some("25 Player (Heroic)"), None] {
            state.difficulty_name = name.map(str::to_owned);
            shared.insert(state.clone());
            screen.sync(&shared, &mut registry);
            let body = registry
                .get(registry.get_by_name("ReadyCheckFrameText").unwrap())
                .unwrap();
            let expected = match name {
                Some(name) => format!("Ann has initiated a ready check.\nRaid Difficulty: {name}"),
                None => "Ann has initiated a ready check.".to_owned(),
            };
            assert!(
                matches!(&body.widget_data, Some(WidgetData::FontString(text)) if text.text == expected)
            );
            assert_eq!((body.width.value(), body.height.value()), (240.0, 28.0));
        }
    }
}
