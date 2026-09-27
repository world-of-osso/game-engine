use game_engine_core::input_bindings_data::{
    BindingKey, BindingMouseButton, InputAction, InputBinding, InputBindingsData, InputState,
};
use game_engine_core::movement_input_data::{
    MoveDirection, compute_movement_input, has_manual_movement_override, movement_speed_multiplier,
    sync_movement_toggles,
};

#[derive(Default)]
struct Held {
    keys: Vec<BindingKey>,
    edges: Vec<BindingKey>,
    mouse: Vec<BindingMouseButton>,
    shift: bool,
}

impl InputState for Held {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.keys.contains(&key)
    }
    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.edges.contains(&key)
    }
    fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
        self.mouse.contains(&button)
    }
    fn mouse_just_pressed(&self, _: BindingMouseButton) -> bool {
        false
    }
    fn shift_held(&self) -> bool {
        self.shift
    }
    fn ctrl_held(&self) -> bool {
        false
    }
}

#[test]
fn forward_and_both_mouse_contribute_twice_before_normalization() {
    let held = Held {
        keys: vec![BindingKey::KeyW, BindingKey::KeyD],
        mouse: vec![BindingMouseButton::Left, BindingMouseButton::Right],
        ..Default::default()
    };
    let (vector, animation) =
        compute_movement_input(&InputBindingsData::default(), &held, false, false, 0.0);
    assert_eq!(vector, [-1.0, 0.0, 2.0]);
    assert_eq!(animation, MoveDirection::Forward);
}

#[test]
fn opposing_actions_cancel_vector_but_forward_wins_animation() {
    let held = Held {
        keys: vec![
            BindingKey::KeyW,
            BindingKey::KeyS,
            BindingKey::KeyA,
            BindingKey::KeyD,
        ],
        ..Default::default()
    };
    let (vector, animation) = compute_movement_input(
        &InputBindingsData::default(),
        &held,
        false,
        false,
        std::f32::consts::FRAC_PI_2,
    );
    assert!(vector.iter().all(|axis| axis.abs() < 1e-6));
    assert_eq!(animation, MoveDirection::Forward);
}

#[test]
fn modified_bindings_and_scripted_forward_preserve_priority() {
    let mut bindings = InputBindingsData::default();
    bindings.assign(
        InputAction::StrafeLeft,
        InputBinding::ShiftKeyboard(BindingKey::KeyQ),
    );
    let held = Held {
        keys: vec![BindingKey::KeyQ, BindingKey::KeyS],
        shift: true,
        ..Default::default()
    };
    let (vector, animation) = compute_movement_input(&bindings, &held, false, false, 0.0);
    assert_eq!(vector, [1.0, 0.0, -1.0]);
    assert_eq!(animation, MoveDirection::Backward);
    assert!(has_manual_movement_override(&bindings, &held));

    let empty = Held::default();
    assert_eq!(
        compute_movement_input(&bindings, &empty, false, true, 0.0),
        ([0.0, 0.0, 1.0], MoveDirection::Forward),
    );
    assert!(!has_manual_movement_override(&bindings, &empty));
}

#[test]
fn toggles_use_press_edges_and_backward_cancels_autorun_after_toggle() {
    let bindings = InputBindingsData::default();
    let held = Held {
        keys: vec![BindingKey::KeyS],
        edges: vec![BindingKey::NumLock, BindingKey::KeyZ],
        ..Default::default()
    };
    assert_eq!(
        sync_movement_toggles(&bindings, &held, false, true),
        (false, false)
    );
    let held = Held {
        keys: vec![BindingKey::NumLock, BindingKey::KeyZ],
        ..Default::default()
    };
    assert_eq!(
        sync_movement_toggles(&bindings, &held, false, true),
        (false, true)
    );
    assert_eq!(
        sync_movement_toggles(&bindings, &Held::default(), true, false),
        (true, false),
    );
}

#[test]
fn jump_edge_autorun_edge_and_both_mouse_override_but_scripted_alone_does_not() {
    let bindings = InputBindingsData::default();
    for key in [BindingKey::Space, BindingKey::NumLock] {
        let held = Held {
            edges: vec![key],
            ..Default::default()
        };
        assert!(has_manual_movement_override(&bindings, &held));
    }
    let held = Held {
        mouse: vec![BindingMouseButton::Left, BindingMouseButton::Right],
        ..Default::default()
    };
    assert!(has_manual_movement_override(&bindings, &held));
    assert!(!has_manual_movement_override(&bindings, &Held::default()));
}

#[test]
fn animation_direction_selects_original_speed_multiplier() {
    assert_eq!(movement_speed_multiplier(MoveDirection::None), 1.0);
    assert_eq!(movement_speed_multiplier(MoveDirection::Forward), 1.0);
    assert_eq!(movement_speed_multiplier(MoveDirection::Backward), 0.6);
    assert_eq!(movement_speed_multiplier(MoveDirection::Left), 0.8);
    assert_eq!(movement_speed_multiplier(MoveDirection::Right), 0.8);
}
