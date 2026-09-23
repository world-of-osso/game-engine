use super::*;

#[test]
fn assign_swaps_existing_owner() {
    let mut bindings = InputBindings::default();
    bindings.assign(InputAction::Jump, InputBinding::Keyboard(KeyCode::KeyW));

    assert_eq!(
        bindings.binding(InputAction::Jump),
        Some(InputBinding::Keyboard(KeyCode::KeyW))
    );
    assert_eq!(bindings.binding(InputAction::MoveForward), None);
}

#[test]
fn reset_section_only_resets_selected_section() {
    let mut bindings = InputBindings::default();
    bindings.clear(InputAction::MoveForward);
    bindings.clear(InputAction::ToggleMute);

    bindings.reset_section(BindingSection::Movement);

    assert_eq!(
        bindings.binding(InputAction::MoveForward),
        Some(InputBinding::Keyboard(KeyCode::KeyW))
    );
    assert_eq!(bindings.binding(InputAction::ToggleMute), None);
}

#[test]
fn autorun_has_default_num_lock_binding() {
    assert_eq!(
        InputAction::AutoRun.default_binding(),
        Some(InputBinding::Keyboard(KeyCode::NumLock))
    );
    assert_eq!(
        InputBindings::default().binding(InputAction::AutoRun),
        Some(InputBinding::Keyboard(KeyCode::NumLock))
    );
}

#[test]
fn target_nearest_has_default_tab_binding() {
    assert_eq!(
        InputAction::TargetNearest.default_binding(),
        Some(InputBinding::Keyboard(KeyCode::Tab))
    );
    assert_eq!(
        InputBindings::default().binding(InputAction::TargetNearest),
        Some(InputBinding::Keyboard(KeyCode::Tab))
    );
    assert_eq!(
        InputAction::TargetNearest.section(),
        BindingSection::Targeting
    );
}

#[test]
fn target_nearest_tab_binding_token_round_trips() {
    let token = binding_token(InputBinding::Keyboard(KeyCode::Tab));
    let parsed = parse_binding_token(&token).expect("tab token should parse");
    assert_eq!(parsed, InputBinding::Keyboard(KeyCode::Tab));
    assert_eq!(key_display(KeyCode::Tab), "Tab");
}

#[test]
fn num_lock_binding_token_round_trips() {
    let token = binding_token(InputBinding::Keyboard(KeyCode::NumLock));
    let parsed = parse_binding_token(&token).expect("num lock token should parse");
    assert_eq!(parsed, InputBinding::Keyboard(KeyCode::NumLock));
    assert_eq!(key_display(KeyCode::NumLock), "Num Lock");
}

#[test]
fn panel_toggles_default_to_classic_keys() {
    let bindings = InputBindings::default();
    let expected = [
        (
            InputAction::ToggleCharacter,
            InputBinding::Keyboard(KeyCode::KeyC),
        ),
        (
            InputAction::ToggleProfessions,
            InputBinding::Keyboard(KeyCode::KeyK),
        ),
        (
            InputAction::ToggleAchievements,
            InputBinding::Keyboard(KeyCode::KeyY),
        ),
        (
            InputAction::ToggleTalents,
            InputBinding::Keyboard(KeyCode::KeyN),
        ),
        (
            InputAction::ToggleEncounterJournal,
            InputBinding::Keyboard(KeyCode::KeyJ),
        ),
        (
            InputAction::ToggleSocial,
            InputBinding::Keyboard(KeyCode::KeyO),
        ),
        (
            InputAction::ToggleMail,
            InputBinding::Keyboard(KeyCode::KeyM),
        ),
        (
            InputAction::ToggleLootRules,
            InputBinding::Keyboard(KeyCode::KeyL),
        ),
        (
            InputAction::ToggleWorldMap,
            InputBinding::ShiftKeyboard(KeyCode::KeyM),
        ),
    ];
    for (action, binding) in expected {
        assert_eq!(bindings.binding(action), Some(binding), "{action:?}");
        assert_eq!(action.section(), BindingSection::Interface);
        assert_eq!(InputAction::from_key(action.key()), Some(action));
    }
    assert_eq!(
        actions_for_section(BindingSection::Interface).len(),
        expected.len()
    );
}

#[test]
fn shift_binding_shadows_plain_binding_on_same_key() {
    let bindings = InputBindings::default();
    let mouse = ButtonInput::<MouseButton>::default();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ShiftLeft);
    keys.press(KeyCode::KeyM);

    assert!(bindings.is_just_pressed(InputAction::ToggleWorldMap, &keys, &mouse));
    assert!(!bindings.is_just_pressed(InputAction::ToggleMail, &keys, &mouse));
    assert!(!bindings.is_just_pressed(InputAction::ToggleMute, &keys, &mouse));

    let mut plain = ButtonInput::<KeyCode>::default();
    plain.press(KeyCode::KeyM);
    assert!(bindings.is_just_pressed(InputAction::ToggleMail, &plain, &mouse));
    assert!(!bindings.is_just_pressed(InputAction::ToggleWorldMap, &plain, &mouse));
}

#[test]
fn shift_binding_token_round_trips_and_displays() {
    let binding = InputBinding::ShiftKeyboard(KeyCode::KeyM);
    let token = binding_token(binding);
    assert_eq!(token, "shift+key:KeyM");
    assert_eq!(parse_binding_token(&token), Ok(binding));
    assert_eq!(binding.display(), "Shift-M");
}

#[test]
fn captured_binding_uses_shift_as_modifier() {
    let mut keys = ButtonInput::<KeyCode>::default();
    assert_eq!(
        captured_keyboard_binding(KeyCode::KeyP, &keys),
        Some(InputBinding::Keyboard(KeyCode::KeyP))
    );
    keys.press(KeyCode::ShiftRight);
    assert_eq!(captured_keyboard_binding(KeyCode::ShiftRight, &keys), None);
    assert_eq!(
        captured_keyboard_binding(KeyCode::KeyP, &keys),
        Some(InputBinding::ShiftKeyboard(KeyCode::KeyP))
    );
}

#[test]
fn saved_file_without_panel_toggles_gets_non_conflicting_defaults() {
    let saved = r#"{"bindings":{"Jump":"key:KeyC","ToggleMute":"key:KeyM","MoveForward":null}}"#;

    let bindings: InputBindings = serde_json::from_str(saved).unwrap();

    assert_eq!(
        bindings.binding(InputAction::Jump),
        Some(InputBinding::Keyboard(KeyCode::KeyC))
    );
    assert_eq!(
        bindings.binding(InputAction::MoveForward),
        None,
        "saved clear is kept"
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleCharacter),
        None,
        "default C is already owned by saved Jump"
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleMail),
        None,
        "M owned by saved mute"
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleTalents),
        Some(InputBinding::Keyboard(KeyCode::KeyN))
    );
    assert_eq!(
        bindings.binding(InputAction::MoveBackward),
        Some(InputBinding::Keyboard(KeyCode::KeyS)),
        "actions missing from the file fall back to defaults"
    );
}
