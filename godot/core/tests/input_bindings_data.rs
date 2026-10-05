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
fn launcher_binding_is_listed_and_matches_ctrl_space_only() {
    let action = InputAction::from_key("toggle_launcher").expect("Toggle Launcher binding");
    assert_eq!(action.label(), "Toggle Launcher");
    assert!(actions_for_section(BindingSection::Interface).contains(&action));
    let bindings = InputBindingsData::default();
    assert_eq!(
        bindings.binding(action),
        Some(InputBinding::CtrlKeyboard(BindingKey::Space))
    );
    let mut state = State {
        edge: vec![BindingKey::Space],
        ..State::default()
    };
    assert!(!bindings.is_just_pressed(action, &state));
    state.ctrl = true;
    assert!(bindings.is_just_pressed(action, &state));
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
    assert_eq!(InputAction::ALL.len(), 96);
    assert_eq!(
        BindingSection::ALL.map(|s| actions_for_section(s).len()),
        [8, 6, 14, 22, 12, 12, 1, 15, 6]
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
fn player_spells_bindings_are_listed_under_interface_without_invented_spec_default() {
    let bindings = InputBindingsData::default();
    for (action, label, binding) in [
        (
            InputAction::ToggleSpellbook,
            "Spellbook",
            Some(InputBinding::Keyboard(BindingKey::KeyP)),
        ),
        (
            InputAction::ToggleTalents,
            "Talents",
            Some(InputBinding::Keyboard(BindingKey::KeyN)),
        ),
        (InputAction::ToggleSpecialization, "Specialization", None),
    ] {
        assert_eq!(action.label(), label);
        assert_eq!(action.section(), BindingSection::Interface);
        assert_eq!(bindings.binding(action), binding);
        assert!(
            game_engine_core::input_bindings_data::actions_for_section(BindingSection::Interface)
                .contains(&action)
        );
    }
    let restored: InputBindingsData = ron::from_str(&ron::to_string(&bindings).unwrap()).unwrap();
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

/// Forever labels a binding with FlareUI's `ShortenKey` over the raw `GetBindingKey` string
/// (`Modules/ActionBars.lua:201-222`): "BUTTONn" -> "Bn", then `KEY_SHORT` in order; Modern
/// keeps `GetBindingText(key, 1)`.
#[test]
fn forever_hotkey_text_follows_flareui_shorten_key() {
    use BindingKey::*;
    use BindingMouseButton::*;
    use InputBinding::{CtrlKeyboard, Keyboard, Mouse, ShiftKeyboard};
    let table = [
        (CtrlKeyboard(Digit1), "CTRL-1", "C1", "c-1"),
        (ShiftKeyboard(Digit3), "SHIFT-3", "S3", "s-3"),
        (ShiftKeyboard(BracketRight), "SHIFT-]", "S]", "s-]"),
        (Keyboard(KeyQ), "Q", "Q", "Q"),
        (Keyboard(Space), "SPACE", "Sp", "Spacebar"),
        (CtrlKeyboard(Space), "CTRL-SPACE", "CSp", "c-Spacebar"),
        (Keyboard(Backspace), "BACKSPACE", "BS", "Backspace"),
        (Keyboard(Tab), "TAB", "Tb", "Tab"),
        (Keyboard(NumpadAdd), "NUMPADPLUS", "N+", "Num Pad +"),
        (Keyboard(NumpadSubtract), "NUMPADMINUS", "N-", "Num Pad -"),
        (Keyboard(NumLock), "NUMLOCK", "NL", "Num Lock"),
        (Keyboard(PageDown), "PAGEDOWN", "PD", "Page Down"),
        (Keyboard(Delete), "DELETE", "Del", "Delete"),
        (Keyboard(Insert), "INSERT", "Ins", "Insert"),
        (Keyboard(Home), "HOME", "Hm", "Home"),
        (Keyboard(End), "END", "En", "End"),
        (Keyboard(F5), "F5", "F5", "F5"),
        (Keyboard(Minus), "-", "-", "-"),
        (Mouse(Middle), "BUTTON3", "B3", "Middle Mouse"),
        (Mouse(Back), "BUTTON4", "B4", "Mouse Button 4"),
        (Mouse(Other(10)), "BUTTON10", "B10", "Mouse Button 10"),
    ];
    for (binding, key, forever, modern) in table {
        assert_eq!(binding.binding_key_name(), key, "{binding:?}");
        assert_eq!(binding.flare_hotkey_text(), forever, "{binding:?}");
        assert_eq!(binding.hotkey_text(), modern, "{binding:?}");
    }
}

/// Retail `MULTIACTIONBAR1BUTTONn` / `MULTIACTIONBAR2BUTTONn`: "Action Bar 2 Button n" /
/// "Action Bar 3 Button n" under the Action Bar 2 / 3 headers, shipped unbound.
#[test]
fn extra_action_bar_buttons_are_listed_unbound_under_their_bar() {
    let bindings = InputBindingsData::default();
    for (bar, actions, section) in [
        (
            2,
            InputAction::MULTI_ACTION_BAR_1,
            BindingSection::ActionBar2,
        ),
        (
            3,
            InputAction::MULTI_ACTION_BAR_2,
            BindingSection::ActionBar3,
        ),
    ] {
        assert_eq!(actions_for_section(section), actions);
        assert_eq!(section.title(), format!("Action Bar {bar}"));
        for (index, action) in actions.into_iter().enumerate() {
            assert_eq!(bindings.binding(action), None, "{action:?}");
            assert_eq!(
                action.label(),
                format!("Action Bar {bar} Button {}", index + 1)
            );
            assert_eq!(action.section(), section);
            assert_eq!(InputAction::from_key(action.key()), Some(action));
        }
    }
}

/// A key bound to `MULTIACTIONBAR1BUTTON3` survives the options file's save and load; a
/// file saved before the extra bars existed loads them unbound.
#[test]
fn extra_action_bar_binding_persists_in_the_options_file() {
    use game_engine_core::client_options_data::{
        ClientOptionsFile, load_options_file_from_path, save_options_file_to_path,
    };
    let dir = std::env::temp_dir().join(format!("binds-options-{}", std::process::id()));
    let path = dir.join("options.ron");
    let mut file = ClientOptionsFile::default();
    file.bindings.assign(
        InputAction::MultiActionBar1Button3,
        InputBinding::Keyboard(BindingKey::KeyQ),
    );
    save_options_file_to_path(&path, &file).unwrap();
    let loaded = load_options_file_from_path(&path);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        loaded.bindings.binding(InputAction::MultiActionBar1Button3),
        Some(InputBinding::Keyboard(BindingKey::KeyQ))
    );
    assert_eq!(
        loaded.bindings.binding(InputAction::MultiActionBar1Button4),
        None
    );

    let old: InputBindingsData =
        serde_json::from_str(r#"{"bindings":{"ActionSlot1":"key:Digit1"}}"#).unwrap();
    assert_eq!(old.binding(InputAction::MultiActionBar2Button12), None);
}

/// `GetBindingText(key, 1)`: `s-`/`c-` modifiers, the key's own `KEY_` text
/// (`KEY_SPACE` "Spacebar", `KEY_BACKSPACE` "Backspace", `KEY_BUTTON3` "Middle Mouse",
/// `KEY_BUTTON4` "Mouse Button 4").
#[test]
fn hotkey_text_uses_retail_key_names() {
    let cases = [
        (InputBinding::ShiftKeyboard(BindingKey::Digit1), "s-1"),
        (InputBinding::ShiftKeyboard(BindingKey::BracketLeft), "s-["),
        (
            InputBinding::Mouse(BindingMouseButton::Middle),
            "Middle Mouse",
        ),
        (
            InputBinding::Mouse(BindingMouseButton::Back),
            "Mouse Button 4",
        ),
        (InputBinding::Keyboard(BindingKey::Space), "Spacebar"),
        (InputBinding::Keyboard(BindingKey::Backspace), "Backspace"),
        (InputBinding::Keyboard(BindingKey::KeyQ), "Q"),
    ];
    for (binding, text) in cases {
        assert_eq!(binding.hotkey_text(), text, "{binding:?}");
    }
}
