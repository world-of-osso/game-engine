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
