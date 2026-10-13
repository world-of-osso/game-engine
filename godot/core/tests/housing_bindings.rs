use game_engine_core::input_bindings_data::{
    BindingKey, InputAction, InputBinding, InputBindingsData, actions_for_section,
};

#[test]
fn housing_actions_are_unbound_assignable_and_persist_retail_names() {
    let mut bindings = InputBindingsData::default();
    for (name, label) in [
        ("HOUSING_TOGGLEBASICDECORMODE", "Decorate Mode"),
        ("HOUSING_TOGGLEEXPERTDECORMODE", "Advanced Mode"),
        ("HOUSING_TOGGLECLEANUPMODE", "Cleanup Mode"),
        ("HOUSING_BASICDECOR_ROTATELEFT", "Rotate Decor Left"),
        ("HOUSING_BASICDECOR_ROTATERIGHT", "Rotate Decor Right"),
        ("HOUSING_REMOVEDECOR", "Remove Decor"),
    ] {
        let action = InputAction::from_key(name).expect("Retail housing action must be assignable");
        assert_eq!(action.key(), name);
        assert_eq!(action.label(), label);
        assert_eq!(action.section().title(), "Housing");
        assert!(actions_for_section(action.section()).contains(&action));
        assert_eq!(action.default_binding(), None);
        assert_eq!(bindings.binding(action), None);
        bindings.assign(action, InputBinding::Keyboard(BindingKey::F6));
        let json = serde_json::to_string(&bindings).unwrap();
        let saved: InputBindingsData = serde_json::from_str(&json).unwrap();
        assert_eq!(
            saved.binding(action),
            Some(InputBinding::Keyboard(BindingKey::F6))
        );
        bindings.clear(action);
    }
}
