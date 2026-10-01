use game_engine_core::input_bindings_data::{
    BindingKey, BindingMouseButton, BindingSection, InputAction, InputBinding, InputBindingsData,
    InputState, actions_for_section, parse_binding_token,
};

#[derive(Default)]
struct State {
    held: Vec<BindingKey>,
    edge: Vec<BindingKey>,
    mouse_held: Vec<BindingMouseButton>,
    mouse_edge: Vec<BindingMouseButton>,
    shift: bool,
    ctrl: bool,
}
impl InputState for State {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.held.contains(&key)
    }
    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.edge.contains(&key)
    }
    fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
        self.mouse_held.contains(&button)
    }
    fn mouse_just_pressed(&self, button: BindingMouseButton) -> bool {
        self.mouse_edge.contains(&button)
    }
    fn shift_held(&self) -> bool {
        self.shift
    }
    fn ctrl_held(&self) -> bool {
        self.ctrl
    }
}

#[test]
fn complete_token_grammar_round_trips_and_rejects_unsupported() {
    let keys: Vec<_> = ('A'..='Z')
        .map(|c| format!("Key{c}"))
        .chain((0..=9).map(|n| format!("Digit{n}")))
        .chain((1..=12).map(|n| format!("F{n}")))
        .chain(
            [
                "Space",
                "Tab",
                "Escape",
                "Minus",
                "Equal",
                "BracketLeft",
                "BracketRight",
                "ArrowLeft",
                "ArrowRight",
                "ArrowUp",
                "ArrowDown",
                "PageUp",
                "PageDown",
                "NumLock",
                "Home",
                "End",
                "Insert",
                "Delete",
                "Backspace",
                "Enter",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .collect();
    assert_eq!(keys.len(), 68);
    for key in keys {
        for prefix in ["key:", "shift+key:", "ctrl+key:"] {
            let token = format!("{prefix}{key}");
            let binding = parse_binding_token(&token).unwrap();
            assert_eq!(
                serde_json::to_string(&binding).unwrap(),
                format!("\"{token}\"")
            );
        }
    }
    for button in [
        "Left",
        "Right",
        "Middle",
        "Back",
        "Forward",
        "Other(0)",
        "Other(65535)",
    ] {
        let token = format!("mouse:{button}");
        let binding = parse_binding_token(&token).unwrap();
        assert_eq!(
            serde_json::to_string(&binding).unwrap(),
            format!("\"{token}\"")
        );
    }
    for bad in [
        "key:Numpad1",
        "mouse:Other(65536)",
        "key:keyA",
        "mouse:Other(-1)",
    ] {
        assert!(parse_binding_token(bad).is_err(), "{bad}");
    }
}

#[test]
fn inventory_defaults_and_sections_are_exact() {
    let bindings = InputBindingsData::default();
    assert_eq!(InputAction::ALL.len(), 40);
    assert_eq!(
        BindingSection::ALL.map(|s| actions_for_section(s).len()),
        [8, 6, 2, 12, 1, 11]
    );
    let mut seen = std::collections::BTreeSet::new();
    for action in InputAction::ALL {
        assert!(seen.insert(action));
        assert_eq!(InputAction::from_key(action.key()), Some(action));
        assert_eq!(bindings.binding(action), action.default_binding());
    }
    assert_eq!(
        bindings.binding(InputAction::ToggleMute),
        Some(InputBinding::CtrlKeyboard(BindingKey::KeyS))
    );
    assert_eq!(bindings.binding(InputAction::ToggleLootRules), None);
    // Retail `TOGGLEFPS` "Toggle Framerate Display": Ctrl+R.
    assert_eq!(
        bindings.binding(InputAction::ToggleFramerate),
        Some(InputBinding::CtrlKeyboard(BindingKey::KeyR))
    );
    assert_eq!(
        InputAction::ToggleFramerate.label(),
        "Toggle Framerate Display"
    );
    // Retail `SITORSTAND` (`BINDING_NAME_SITORSTAND` "Sit/Move Down"): X, descends while swimming.
    assert_eq!(
        bindings.binding(InputAction::SitOrStand),
        Some(InputBinding::Keyboard(BindingKey::KeyX))
    );
    assert_eq!(InputAction::SitOrStand.label(), "Sit/Move Down");
    assert_eq!(InputAction::SitOrStand.section(), BindingSection::Movement);
    let ron = ron::to_string(&bindings).unwrap();
    assert!(
        ron.contains("ToggleMute") && ron.contains("ctrl+key:KeyS"),
        "{ron}"
    );
    let restored: InputBindingsData = ron::from_str(&ron).unwrap();
    assert_eq!(restored, bindings);
}

#[test]
fn migration_null_missing_and_explicit_owner_survive_serde() {
    let saved = r#"{"bindings":{"ToggleMute":"key:KeyM","Jump":"key:KeyN","MoveForward":null,"TargetSelf":"ctrl+key:KeyS"}}"#;
    let bindings: InputBindingsData = serde_json::from_str(saved).unwrap();
    assert_eq!(
        bindings.binding(InputAction::ToggleMute),
        None,
        "explicit TargetSelf owns Ctrl+S"
    );
    assert_eq!(bindings.binding(InputAction::MoveForward), None);
    assert_eq!(bindings.binding(InputAction::ToggleTalents), None);
    assert_eq!(
        bindings.binding(InputAction::ToggleWorldMap),
        Some(InputBinding::Keyboard(BindingKey::KeyM))
    );
    assert_eq!(
        bindings.binding(InputAction::TargetSelf),
        Some(InputBinding::CtrlKeyboard(BindingKey::KeyS))
    );
    let mut owned = bindings.clone();
    owned.assign(InputAction::Jump, InputBinding::Keyboard(BindingKey::KeyM));
    assert_eq!(owned.binding(InputAction::ToggleWorldMap), None);
    assert!(
        serde_json::from_str::<InputBindingsData>(r#"{"bindings":{"Jump":"key:Numpad1"}}"#)
            .is_err()
    );
}

#[test]
fn modifiers_shadow_plain_and_edges_are_distinct() {
    let mut bindings = InputBindingsData::default();
    bindings.assign(
        InputAction::ToggleLootRules,
        InputBinding::ShiftKeyboard(BindingKey::KeyM),
    );
    let state = State {
        held: vec![BindingKey::KeyM],
        edge: vec![BindingKey::KeyM],
        shift: true,
        ..Default::default()
    };
    assert!(bindings.is_pressed(InputAction::ToggleLootRules, &state));
    assert!(bindings.is_just_pressed(InputAction::ToggleLootRules, &state));
    assert!(!bindings.is_pressed(InputAction::ToggleWorldMap, &state));
    let state = State {
        held: vec![BindingKey::KeyM],
        shift: true,
        ..Default::default()
    };
    assert!(!bindings.is_just_pressed(InputAction::ToggleLootRules, &state));
    let state = State {
        held: vec![BindingKey::KeyS],
        edge: vec![BindingKey::KeyS],
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    assert!(bindings.is_just_pressed(InputAction::ToggleMute, &state));
    assert!(!bindings.is_pressed(InputAction::MoveBackward, &state));
    let state = State {
        mouse_held: vec![BindingMouseButton::Other(65535)],
        mouse_edge: vec![],
        ..Default::default()
    };
    bindings.assign(
        InputAction::Jump,
        InputBinding::Mouse(BindingMouseButton::Other(65535)),
    );
    assert!(bindings.is_pressed(InputAction::Jump, &state));
    assert!(!bindings.is_just_pressed(InputAction::Jump, &state));
}
