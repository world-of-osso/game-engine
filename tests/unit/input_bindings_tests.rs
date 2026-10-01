use super::*;

#[test]
fn assign_swaps_existing_owner() {
    let mut bindings = InputBindings::default();
    bindings.assign(InputAction::Jump, InputBinding::Keyboard(BindingKey::KeyW));

    assert_eq!(
        bindings.binding(InputAction::Jump),
        Some(InputBinding::Keyboard(BindingKey::KeyW))
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
        Some(InputBinding::Keyboard(BindingKey::KeyW))
    );
    assert_eq!(bindings.binding(InputAction::ToggleMute), None);
}

#[test]
fn autorun_has_default_num_lock_binding() {
    assert_eq!(
        InputAction::AutoRun.default_binding(),
        Some(InputBinding::Keyboard(BindingKey::NumLock))
    );
    assert_eq!(
        InputBindings::default().binding(InputAction::AutoRun),
        Some(InputBinding::Keyboard(BindingKey::NumLock))
    );
}

#[test]
fn target_nearest_has_default_tab_binding() {
    assert_eq!(
        InputAction::TargetNearest.default_binding(),
        Some(InputBinding::Keyboard(BindingKey::Tab))
    );
    assert_eq!(
        InputBindings::default().binding(InputAction::TargetNearest),
        Some(InputBinding::Keyboard(BindingKey::Tab))
    );
    assert_eq!(
        InputAction::TargetNearest.section(),
        BindingSection::Targeting
    );
}

#[test]
fn target_nearest_tab_binding_token_round_trips() {
    let token = binding_token(InputBinding::Keyboard(BindingKey::Tab));
    let parsed = parse_binding_token(&token).expect("tab token should parse");
    assert_eq!(parsed, InputBinding::Keyboard(BindingKey::Tab));
    assert_eq!(key_display(BindingKey::Tab), "Tab");
}

#[test]
fn num_lock_binding_token_round_trips() {
    let token = binding_token(InputBinding::Keyboard(BindingKey::NumLock));
    let parsed = parse_binding_token(&token).expect("num lock token should parse");
    assert_eq!(parsed, InputBinding::Keyboard(BindingKey::NumLock));
    assert_eq!(key_display(BindingKey::NumLock), "Num Lock");
}

#[test]
fn panel_toggles_use_retail_default_keys() {
    let bindings = InputBindings::default();
    let expected = [
        (
            InputAction::ToggleCharacter,
            Some(InputBinding::Keyboard(BindingKey::KeyC)),
        ),
        (
            InputAction::ToggleSpellbook,
            Some(InputBinding::Keyboard(BindingKey::KeyP)),
        ),
        (
            InputAction::ToggleProfessions,
            Some(InputBinding::Keyboard(BindingKey::KeyK)),
        ),
        (
            InputAction::ToggleAchievements,
            Some(InputBinding::Keyboard(BindingKey::KeyY)),
        ),
        (
            InputAction::ToggleTalents,
            Some(InputBinding::Keyboard(BindingKey::KeyN)),
        ),
        (
            InputAction::ToggleEncounterJournal,
            Some(InputBinding::Keyboard(BindingKey::KeyJ)),
        ),
        (
            InputAction::ToggleSocial,
            Some(InputBinding::Keyboard(BindingKey::KeyO)),
        ),
        (InputAction::ToggleLootRules, None),
        (
            InputAction::ToggleQuestLog,
            Some(InputBinding::Keyboard(BindingKey::KeyL)),
        ),
        (
            InputAction::ToggleWorldMap,
            Some(InputBinding::Keyboard(BindingKey::KeyM)),
        ),
        (
            InputAction::ToggleFramerate,
            Some(InputBinding::CtrlKeyboard(BindingKey::KeyR)),
        ),
    ];
    for (action, binding) in expected {
        assert_eq!(bindings.binding(action), binding, "{action:?}");
        assert_eq!(action.section(), BindingSection::Interface);
        assert_eq!(InputAction::from_key(action.key()), Some(action));
    }
    assert_eq!(
        actions_for_section(BindingSection::Interface).len(),
        expected.len()
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleMute),
        Some(InputBinding::CtrlKeyboard(BindingKey::KeyS))
    );
}

#[test]
fn default_bindings_have_no_duplicates() {
    let bindings = InputBindings::default();
    let bound: Vec<_> = InputAction::ALL
        .into_iter()
        .filter_map(|action| bindings.binding(action))
        .collect();
    for (index, binding) in bound.iter().enumerate() {
        assert!(
            !bound[index + 1..].contains(binding),
            "{binding:?} bound twice"
        );
    }
}

#[test]
fn ctrl_s_toggles_mute_without_moving_backward() {
    let bindings = InputBindings::default();
    let mouse = ButtonInput::<MouseButton>::default();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ControlLeft);
    keys.press(KeyCode::KeyS);

    assert!(bindings.is_just_pressed(InputAction::ToggleMute, &keys, &mouse));
    assert!(!bindings.is_pressed(InputAction::MoveBackward, &keys, &mouse));

    let mut plain = ButtonInput::<KeyCode>::default();
    plain.press(KeyCode::KeyS);
    assert!(bindings.is_pressed(InputAction::MoveBackward, &plain, &mouse));
    assert!(!bindings.is_just_pressed(InputAction::ToggleMute, &plain, &mouse));
}

#[test]
fn shift_binding_shadows_plain_binding_on_same_key() {
    let mut bindings = InputBindings::default();
    bindings.assign(
        InputAction::ToggleLootRules,
        InputBinding::ShiftKeyboard(BindingKey::KeyM),
    );
    let mouse = ButtonInput::<MouseButton>::default();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ShiftLeft);
    keys.press(KeyCode::KeyM);

    assert!(bindings.is_just_pressed(InputAction::ToggleLootRules, &keys, &mouse));
    assert!(!bindings.is_just_pressed(InputAction::ToggleWorldMap, &keys, &mouse));
}

#[test]
fn modifier_binding_tokens_round_trip_and_display() {
    for (binding, token, display) in [
        (
            InputBinding::ShiftKeyboard(BindingKey::KeyM),
            "shift+key:KeyM",
            "Shift-M",
        ),
        (
            InputBinding::CtrlKeyboard(BindingKey::KeyS),
            "ctrl+key:KeyS",
            "Ctrl-S",
        ),
    ] {
        assert_eq!(binding_token(binding), token);
        assert_eq!(parse_binding_token(token), Ok(binding));
        assert_eq!(binding.display(), display);
    }
}

#[test]
fn captured_binding_uses_shift_and_ctrl_as_modifiers() {
    let mut keys = ButtonInput::<KeyCode>::default();
    assert_eq!(
        captured_keyboard_binding(KeyCode::KeyP, &keys),
        Ok(Some(InputBinding::Keyboard(BindingKey::KeyP)))
    );
    keys.press(KeyCode::ShiftRight);
    assert_eq!(
        captured_keyboard_binding(KeyCode::ShiftRight, &keys),
        Ok(None)
    );
    assert_eq!(
        captured_keyboard_binding(KeyCode::KeyP, &keys),
        Ok(Some(InputBinding::ShiftKeyboard(BindingKey::KeyP)))
    );
    keys.press(KeyCode::ControlLeft);
    assert_eq!(
        captured_keyboard_binding(KeyCode::ControlLeft, &keys),
        Ok(None)
    );
    assert_eq!(
        captured_keyboard_binding(KeyCode::KeyP, &keys),
        Ok(Some(InputBinding::CtrlKeyboard(BindingKey::KeyP)))
    );
}

#[test]
fn unsupported_captured_key_reports_error_without_rebinding() {
    let keys = ButtonInput::<KeyCode>::default();
    assert_eq!(
        captured_keyboard_binding(KeyCode::Numpad1, &keys),
        Err(KeyCode::Numpad1)
    );
    assert_eq!(
        captured_keyboard_binding(KeyCode::ShiftLeft, &keys),
        Ok(None)
    );
}

#[test]
fn saved_file_on_old_mute_default_migrates_to_retail_defaults() {
    let saved = r#"{"bindings":{"ToggleMute":"key:KeyM","MoveBackward":"key:KeyS"}}"#;

    let bindings: InputBindings = serde_json::from_str(saved).unwrap();

    assert_eq!(
        bindings.binding(InputAction::ToggleMute),
        Some(InputBinding::CtrlKeyboard(BindingKey::KeyS))
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleWorldMap),
        Some(InputBinding::Keyboard(BindingKey::KeyM))
    );
    assert_eq!(bindings.binding(InputAction::ToggleLootRules), None);
}

#[test]
fn saved_explicit_bindings_win_over_new_defaults() {
    let saved = r#"{"bindings":{"Jump":"key:KeyM","ToggleMute":"key:KeyU","MoveForward":null,"TargetSelf":"ctrl+key:KeyS"}}"#;

    let bindings: InputBindings = serde_json::from_str(saved).unwrap();

    assert_eq!(
        bindings.binding(InputAction::Jump),
        Some(InputBinding::Keyboard(BindingKey::KeyM))
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleMute),
        Some(InputBinding::Keyboard(BindingKey::KeyU))
    );
    assert_eq!(
        bindings.binding(InputAction::MoveForward),
        None,
        "saved clear is kept"
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleWorldMap),
        None,
        "M owned by saved Jump"
    );
    assert_eq!(
        bindings.binding(InputAction::ToggleTalents),
        Some(InputBinding::Keyboard(BindingKey::KeyN)),
        "missing actions get their defaults"
    );
}
