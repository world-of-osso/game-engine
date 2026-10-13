use game_engine_core::input_bindings_data::{InputAction, InputBindingsData, actions_for_section};

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
