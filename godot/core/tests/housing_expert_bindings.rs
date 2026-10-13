use game_engine_core::input_bindings_data::{
    BindingKey, BindingMouseButton, InputAction, InputBinding, InputBindingsData, InputState,
    actions_for_section,
};

#[derive(Default)]
struct HeldInput {
    pressed: bool,
    ctrl: bool,
}
impl InputState for HeldInput {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.pressed && key == BindingKey::F6
    }
    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.key_pressed(key)
    }
    fn mouse_pressed(&self, _: BindingMouseButton) -> bool {
        false
    }
    fn mouse_just_pressed(&self, _: BindingMouseButton) -> bool {
        false
    }
    fn shift_held(&self) -> bool {
        false
    }
    fn ctrl_held(&self) -> bool {
        self.ctrl
    }
}

#[test]
fn expert_held_increments_follow_assignment_release_modifiers_and_unbind() {
    let mut bindings = InputBindingsData::default();
    let mut input = HeldInput {
        pressed: true,
        ctrl: false,
    };
    assert_eq!(bindings.expert_increment_state(&input), [false; 10]);
    bindings.assign(
        InputAction::HousingExpertRight,
        InputBinding::Keyboard(BindingKey::F6),
    );
    let mut expected = [false; 10];
    expected[1] = true;
    assert_eq!(bindings.expert_increment_state(&input), expected);
    input.pressed = false;
    assert_eq!(bindings.expert_increment_state(&input), [false; 10]);
    bindings.assign(
        InputAction::HousingExpertScaleUp,
        InputBinding::CtrlKeyboard(BindingKey::F6),
    );
    input.pressed = true;
    input.ctrl = true;
    expected = [false; 10];
    expected[8] = true;
    assert_eq!(bindings.expert_increment_state(&input), expected);
    input.ctrl = false;
    expected = [false; 10];
    expected[1] = true;
    assert_eq!(bindings.expert_increment_state(&input), expected);
    bindings.clear(InputAction::HousingExpertRight);
    assert_eq!(bindings.expert_increment_state(&input), [false; 10]);
}

#[test]
fn expert_increment_actions_have_distinct_hold_slots_and_submodes_do_not_hold() {
    for (index, name) in [
        "HOUSING_EXPERTDECORINCREMENT_LEFT",
        "HOUSING_EXPERTDECORINCREMENT_RIGHT",
        "HOUSING_EXPERTDECORINCREMENT_FORWARD",
        "HOUSING_EXPERTDECORINCREMENT_BACK",
        "HOUSING_EXPERTDECORINCREMENT_UP",
        "HOUSING_EXPERTDECORINCREMENT_DOWN",
        "HOUSING_EXPERTDECORINCREMENT_ROTATELEFT",
        "HOUSING_EXPERTDECORINCREMENT_ROTATERIGHT",
        "HOUSING_EXPERTDECORINCREMENT_SCALEUP",
        "HOUSING_EXPERTDECORINCREMENT_SCALEDOWN",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            InputAction::from_key(name)
                .unwrap()
                .expert_increment_index(),
            Some(index)
        );
    }
    assert_eq!(
        InputAction::HousingExpertTranslate.expert_increment_index(),
        None
    );
    assert_eq!(
        InputAction::HousingExpertNextAxis.expert_increment_index(),
        None
    );
    assert_eq!(
        InputAction::HousingRotateLeft.expert_increment_index(),
        None
    );
}

#[test]
fn retail_expert_actions_are_assignable_without_invented_default_keys() {
    let bindings = InputBindingsData::default();
    for name in [
        "HOUSING_EXPERTDECORTRANSLATESUBMODE",
        "HOUSING_EXPERTDECORROTATESUBMODE",
        "HOUSING_EXPERTDECORSCALESUBMODE",
        "HOUSING_EXPERTDECORINCREMENT_LEFT",
        "HOUSING_EXPERTDECORINCREMENT_RIGHT",
        "HOUSING_EXPERTDECORINCREMENT_FORWARD",
        "HOUSING_EXPERTDECORINCREMENT_BACK",
        "HOUSING_EXPERTDECORINCREMENT_UP",
        "HOUSING_EXPERTDECORINCREMENT_DOWN",
        "HOUSING_EXPERTDECORINCREMENT_SCALEUP",
        "HOUSING_EXPERTDECORINCREMENT_SCALEDOWN",
        "HOUSING_EXPERTDECORINCREMENT_ROTATELEFT",
        "HOUSING_EXPERTDECORINCREMENT_ROTATERIGHT",
        "HOUSING_EXPERTDECORROTATION_NEXTAXIS",
    ] {
        let action = InputAction::from_key(name).expect("Retail Expert action must be assignable");
        assert_eq!(action.key(), name);
        assert_eq!(action.section().title(), "Housing");
        assert!(actions_for_section(action.section()).contains(&action));
        assert_eq!(action.default_binding(), None);
        assert_eq!(bindings.binding(action), None);
    }
}
