use super::*;

#[test]
fn automation_click_create_char_transitions_to_char_create() {
    let mut app = build_test_app();
    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::ClickFrame("CreateChar".to_string()),
    ])));

    app.update();
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::CharCreate
    );
    assert!(
        app.world().resource::<UiAutomationQueue>().is_empty(),
        "expected CreateChar click to be consumed by CharSelect automation"
    );
}

#[test]
fn automation_click_delete_char_opens_confirmation_dialog() {
    let mut app = build_test_app();
    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::ClickFrame("DeleteChar".to_string()),
    ])));

    app.update();
    app.update();

    let delete_confirm = app.world().resource::<DeleteCharacterConfirmationState>();
    let target = delete_confirm
        .target
        .as_ref()
        .expect("delete confirm target");
    assert_eq!(target.name, "Elara");
}

#[test]
fn dispatch_char_select_action_select_event_updates_selected_index() {
    let mut app = build_test_app();
    app.insert_resource(CharacterList(vec![
        CharacterListEntry {
            character_id: 1,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
        CharacterListEntry {
            character_id: 2,
            name: "Theron".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
    ]));
    app.update();
    app.world_mut().resource_mut::<SelectedCharIndex>().0 = Some(0);
    app.world_mut()
        .resource_mut::<Messages<crate::scenes::char_select::input::CharSelectClickEvent>>()
        .write(crate::scenes::char_select::input::CharSelectClickEvent(
            "select_char:1".to_string(),
        ));

    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::dispatch_char_select_action)
        .expect("dispatch_char_select_action should run");

    assert_eq!(
        app.world().resource::<SelectedCharIndex>().0,
        Some(1),
        "select_char:1 event should update SelectedCharIndex"
    );
}

#[test]
fn dispatch_char_select_action_delete_event_opens_confirmation_for_selected_character() {
    let mut app = build_test_app();
    app.insert_resource(CharacterList(vec![
        CharacterListEntry {
            character_id: 1,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
        CharacterListEntry {
            character_id: 2,
            name: "Theron".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
    ]));
    app.update();
    app.world_mut().resource_mut::<SelectedCharIndex>().0 = Some(1);
    app.world_mut()
        .resource_mut::<Messages<crate::scenes::char_select::input::CharSelectClickEvent>>()
        .write(crate::scenes::char_select::input::CharSelectClickEvent(
            "delete_char".to_string(),
        ));

    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::dispatch_char_select_action)
        .expect("dispatch_char_select_action should run");

    let delete_confirm = app.world().resource::<DeleteCharacterConfirmationState>();
    let target = delete_confirm
        .target
        .as_ref()
        .expect("delete confirm target");
    assert_eq!(target.character_id, 2);
    assert_eq!(target.name, "Theron");
    assert_eq!(
        delete_confirm.typed_text, "",
        "delete confirmation should start with an empty typed phrase"
    );
    assert_eq!(
        delete_confirm.elapsed_secs, 0.0,
        "delete confirmation countdown should reset when opened"
    );

    app.update();

    let ui = app.world().resource::<UiState>();
    let delete_confirm_input = ui
        .registry
        .get_by_name("DeleteCharacterConfirmInput")
        .expect("DeleteCharacterConfirmInput");
    assert_eq!(
        app.world().resource::<CharSelectFocus>().0,
        Some(delete_confirm_input),
        "delete confirmation should focus the confirmation input when opened"
    );
}

#[test]
fn char_select_update_visuals_rebuilds_ui_state_after_selection_change() {
    let mut app = build_test_app();
    app.insert_resource(CharacterList(vec![
        CharacterListEntry {
            character_id: 1,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
        CharacterListEntry {
            character_id: 2,
            name: "Theron".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
    ]));
    app.update();

    app.world_mut().resource_mut::<SelectedCharIndex>().0 = Some(1);
    app.world_mut()
        .run_system_once(super::char_select_update_visuals)
        .expect("char_select_update_visuals should run");

    let ui = app.world().resource::<UiState>();
    let selected_name_id = ui
        .registry
        .get_by_name("CharSelectCharacterName")
        .expect("CharSelectCharacterName");
    let Some(WidgetData::FontString(selected_name)) = ui
        .registry
        .get(selected_name_id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("CharSelectCharacterName should be a fontstring");
    };
    assert_eq!(
        selected_name.text, "Theron",
        "visual update should rebuild selected name text from the new SelectedCharIndex"
    );

    let card0_selected = ui
        .registry
        .get(
            ui.registry
                .get_by_name("CharCard_0Selected")
                .expect("CharCard_0Selected"),
        )
        .expect("CharCard_0Selected frame");
    let card1_selected = ui
        .registry
        .get(
            ui.registry
                .get_by_name("CharCard_1Selected")
                .expect("CharCard_1Selected"),
        )
        .expect("CharCard_1Selected frame");
    assert!(
        card0_selected.hidden,
        "first card highlight should hide after selecting the second character"
    );
    assert!(
        !card1_selected.hidden,
        "second card highlight should show after selecting the second character"
    );
}

#[test]
fn char_select_keyboard_arrow_down_advances_selected_index() {
    let mut app = build_test_app();
    app.insert_resource(CharacterList(vec![
        CharacterListEntry {
            character_id: 1,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
        CharacterListEntry {
            character_id: 2,
            name: "Theron".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: shared::components::EquipmentAppearance::default(),
        },
    ]));
    app.update();
    app.world_mut().resource_mut::<SelectedCharIndex>().0 = Some(0);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: KeyCode::ArrowDown,
            logical_key: bevy::input::keyboard::Key::ArrowDown,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });

    app.world_mut()
        .run_system_once(crate::scenes::char_select::input::char_select_keyboard_input)
        .expect("char_select_keyboard_input should run");

    assert_eq!(
        app.world().resource::<SelectedCharIndex>().0,
        Some(1),
        "ArrowDown should move char-select selection from index 0 to index 1"
    );
}

#[test]
fn automation_type_uppercases_delete_confirmation_input() {
    let mut app = build_test_app();
    app.update();
    app.world_mut()
        .resource_mut::<DeleteCharacterConfirmationState>()
        .target = Some(DeleteCharacterTarget {
        character_id: 1,
        name: "Elara".to_string(),
    });
    app.update();

    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::TypeText("delete".to_string()),
    ])));

    app.update();

    let delete_confirm = app.world().resource::<DeleteCharacterConfirmationState>();
    assert_eq!(delete_confirm.typed_text, "DELETE");

    let ui = app.world().resource::<UiState>();
    let input_id = ui
        .registry
        .get_by_name("DeleteCharacterConfirmInput")
        .expect("delete confirmation input");
    let Some(WidgetData::EditBox(editbox)) = ui
        .registry
        .get(input_id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("DeleteCharacterConfirmInput should be an editbox");
    };
    assert_eq!(editbox.text, "DELETE");
}
