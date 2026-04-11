use super::*;

#[test]
fn character_card_frames_set_expected_onclick_actions() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });

    let card0 = reg
        .get(reg.get_by_name("CharCard_0").expect("CharCard_0"))
        .expect("CharCard_0 frame");
    let card1 = reg
        .get(reg.get_by_name("CharCard_1").expect("CharCard_1"))
        .expect("CharCard_1 frame");

    assert_eq!(card0.onclick.as_deref(), Some("select_char:0"));
    assert_eq!(card1.onclick.as_deref(), Some("select_char:1"));
}

#[test]
fn find_clicked_action_returns_character_card_action_from_each_card_center() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Brom".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });

    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    for index in 0..3 {
        let center = frame_center(&ui.registry, &format!("CharCard_{index}"));
        let expected = format!("select_char:{index}");

        assert_eq!(
            crate::scenes::char_select::input::find_clicked_action(&ui, center.x, center.y)
                .as_deref(),
            Some(expected.as_str())
        );
    }
}

#[test]
fn find_clicked_action_returns_enter_world_from_button_center() {
    let reg = build_screen_with_real_layout(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Theron".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });
    let center = frame_center(&reg, ENTER_WORLD_BUTTON.0);
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, center.x, center.y).as_deref(),
        Some("enter_world")
    );
}

#[test]
fn find_clicked_action_returns_back_from_button_center() {
    let reg = build_screen_with_real_layout(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Theron".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });
    let center = frame_center(&reg, BACK_BUTTON.0);
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, center.x, center.y).as_deref(),
        Some("back")
    );
}

#[test]
fn find_clicked_action_returns_none_from_empty_area() {
    let reg = build_screen_with_real_layout(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Theron".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, 0.0, 0.0),
        None
    );
}

#[test]
fn find_clicked_action_walks_up_from_character_card_text_to_parent_onclick() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Elara".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Elara".to_string(),
        ..Default::default()
    });
    let label_center = frame_center(&reg, "CharCard_0Name");
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, label_center.x, label_center.y)
            .as_deref(),
        Some("select_char:0")
    );
}

#[test]
fn walk_up_for_onclick_reaches_character_card_action_from_texture_child() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Elara".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Elara".to_string(),
        ..Default::default()
    });
    let backdrop_id = reg
        .get_by_name("CharCard_0Backdrop")
        .expect("CharCard_0Backdrop");

    assert_eq!(
        crate::ui_input::walk_up_for_onclick(&reg, backdrop_id).as_deref(),
        Some("select_char:0")
    );
}

#[test]
fn parse_click_action_event_logs_known_select_action() {
    let (parsed_action, parsed_action_label) =
        crate::scenes::char_select::input::parse_click_action_event("select_char:1");

    assert_eq!(parsed_action, Some(CharSelectAction::SelectChar(1)));
    assert_eq!(parsed_action_label, "select_char:1");
}

#[test]
fn parse_click_action_event_marks_unknown_action_unparsed() {
    let (parsed_action, parsed_action_label) =
        crate::scenes::char_select::input::parse_click_action_event("not_an_action");

    assert_eq!(parsed_action, None);
    assert_eq!(parsed_action_label, "unparsed");
}
