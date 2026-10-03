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
                "NumpadAdd",
                "NumpadSubtract",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .collect();
    assert_eq!(keys.len(), 70);
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
    assert_eq!(InputAction::ALL.len(), 70);
    assert_eq!(
        BindingSection::ALL.map(|s| actions_for_section(s).len()),
        [8, 6, 14, 22, 1, 13, 6]
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
    // Retail OPENALLBAGS B, TOGGLEBACKPACK Shift-B, TOGGLEBAG1-4 F8-F11 -> ToggleBag(4..1).
    let bag_defaults = [
        (
            InputAction::OpenAllBags,
            InputBinding::Keyboard(BindingKey::KeyB),
            None,
        ),
        (
            InputAction::ToggleBackpack,
            InputBinding::ShiftKeyboard(BindingKey::KeyB),
            None,
        ),
        (
            InputAction::ToggleBag1,
            InputBinding::Keyboard(BindingKey::F8),
            Some(4),
        ),
        (
            InputAction::ToggleBag2,
            InputBinding::Keyboard(BindingKey::F9),
            Some(3),
        ),
        (
            InputAction::ToggleBag3,
            InputBinding::Keyboard(BindingKey::F10),
            Some(2),
        ),
        (
            InputAction::ToggleBag4,
            InputBinding::Keyboard(BindingKey::F11),
            Some(1),
        ),
    ];
    for (action, binding, bag) in bag_defaults {
        assert_eq!(bindings.binding(action), Some(binding), "{action:?}");
        assert_eq!(action.toggled_bag(), bag, "{action:?}");
        assert_eq!(action.section(), BindingSection::Bags);
    }
    // Retail MINIMAPZOOMIN Num Pad +, MINIMAPZOOMOUT Num Pad -.
    assert_eq!(
        bindings.binding(InputAction::MinimapZoomIn),
        Some(InputBinding::Keyboard(BindingKey::NumpadAdd))
    );
    assert_eq!(
        bindings.binding(InputAction::MinimapZoomOut),
        Some(InputBinding::Keyboard(BindingKey::NumpadSubtract))
    );
    assert_eq!(
        InputBinding::Keyboard(BindingKey::NumpadAdd).display(),
        "Num Pad +"
    );
    // Retail ASSISTTARGET F, TARGETPREVIOUSENEMY Shift-Tab.
    assert_eq!(
        bindings.binding(InputAction::AssistTarget),
        Some(InputBinding::Keyboard(BindingKey::KeyF))
    );
    assert_eq!(
        bindings.binding(InputAction::TargetPreviousEnemy),
        Some(InputBinding::ShiftKeyboard(BindingKey::Tab))
    );
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

#[test]
fn shift_b_toggles_backpack_and_plain_b_opens_all_bags() {
    let bindings = InputBindingsData::default();
    let shift_b = State {
        held: vec![BindingKey::KeyB],
        edge: vec![BindingKey::KeyB],
        shift: true,
        ..Default::default()
    };
    assert!(bindings.is_just_pressed(InputAction::ToggleBackpack, &shift_b));
    assert!(!bindings.is_just_pressed(InputAction::OpenAllBags, &shift_b));
    let b = State {
        held: vec![BindingKey::KeyB],
        edge: vec![BindingKey::KeyB],
        ..Default::default()
    };
    assert!(bindings.is_just_pressed(InputAction::OpenAllBags, &b));
    assert!(!bindings.is_just_pressed(InputAction::ToggleBackpack, &b));
    let shift_tab = State {
        held: vec![BindingKey::Tab],
        edge: vec![BindingKey::Tab],
        shift: true,
        ..Default::default()
    };
    assert!(bindings.is_just_pressed(InputAction::TargetPreviousEnemy, &shift_tab));
    assert!(!bindings.is_just_pressed(InputAction::TargetNearest, &shift_tab));
}

/// Retail `BONUSACTIONBUTTON1..10` (Bindings_Standard.xml:272-361, `BINDING_HEADER_ACTIONBAR`)
/// on Ctrl-1..Ctrl-9, Ctrl-0: pressing Ctrl+N is pet button N and never main bar button N.
#[test]
fn bonus_action_buttons_default_to_ctrl_digits_and_shadow_the_main_bar() {
    let bindings = InputBindingsData::default();
    let digits = [
        BindingKey::Digit1,
        BindingKey::Digit2,
        BindingKey::Digit3,
        BindingKey::Digit4,
        BindingKey::Digit5,
        BindingKey::Digit6,
        BindingKey::Digit7,
        BindingKey::Digit8,
        BindingKey::Digit9,
        BindingKey::Digit0,
    ];
    let main_bar = [
        InputAction::ActionSlot1,
        InputAction::ActionSlot2,
        InputAction::ActionSlot3,
        InputAction::ActionSlot4,
        InputAction::ActionSlot5,
        InputAction::ActionSlot6,
        InputAction::ActionSlot7,
        InputAction::ActionSlot8,
        InputAction::ActionSlot9,
        InputAction::ActionSlot10,
    ];
    for (index, action) in InputAction::PET_ACTION_SLOTS.into_iter().enumerate() {
        let key = digits[index];
        assert_eq!(
            bindings.binding(action),
            Some(InputBinding::CtrlKeyboard(key)),
            "{action:?}"
        );
        assert_eq!(action.section(), BindingSection::ActionBar);
        assert_eq!(action.label(), format!("Pet Action Button {}", index + 1));
        assert_eq!(action.pet_action_slot(), Some(index));
        assert_eq!(InputAction::from_key(action.key()), Some(action));
        let ctrl = State {
            held: vec![key],
            edge: vec![key],
            ctrl: true,
            ..Default::default()
        };
        assert!(bindings.is_just_pressed(action, &ctrl), "{action:?}");
        assert!(
            !bindings.is_just_pressed(main_bar[index], &ctrl),
            "Ctrl+{key:?} must not press {:?}",
            main_bar[index]
        );
        let plain = State {
            held: vec![key],
            edge: vec![key],
            ..Default::default()
        };
        assert!(bindings.is_just_pressed(main_bar[index], &plain));
        assert!(!bindings.is_just_pressed(action, &plain));
    }
    assert_eq!(InputAction::ActionSlot1.pet_action_slot(), None);
}

/// Action button hotkeys show `GetBindingText(key, true)`: modifiers abbreviated to
/// `CTRL_KEY_TEXT_ABBR` "c" / `SHIFT_KEY_TEXT_ABBR` "s" (GlobalStrings).
#[test]
fn hotkey_text_abbreviates_modifiers() {
    assert_eq!(
        InputBinding::CtrlKeyboard(BindingKey::Digit1).hotkey_text(),
        "c-1"
    );
    assert_eq!(
        InputBinding::CtrlKeyboard(BindingKey::Digit0).hotkey_text(),
        "c-0"
    );
    assert_eq!(
        InputBinding::ShiftKeyboard(BindingKey::KeyB).hotkey_text(),
        "s-B"
    );
    assert_eq!(InputBinding::Keyboard(BindingKey::Minus).hotkey_text(), "-");
}
