use game_engine_core::camera_control_data::CameraState;
use game_engine_core::camera_input_data::{CameraInput, apply_camera_input};
use game_engine_core::input_bindings_data::{
    BindingKey, BindingMouseButton, InputAction, InputBinding, InputBindingsData, InputState,
};

#[derive(Default)]
struct Held {
    keys: Vec<BindingKey>,
    mouse: Vec<BindingMouseButton>,
    shift: bool,
}

impl InputState for Held {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.keys.contains(&key)
    }
    fn key_just_pressed(&self, _: BindingKey) -> bool {
        false
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

fn input() -> CameraInput {
    CameraInput {
        delta_x: 0.0,
        delta_y: 0.0,
        scroll_y: 0.0,
        dt: 1.0,
        look_sensitivity: 0.5,
        invert_y: false,
    }
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn right_mouse_turns_camera_and_faces_character_but_left_only_orbits() {
    let bindings = InputBindingsData::default();
    let mut camera = CameraState::default();
    let mut motion = input();
    motion.delta_x = 2.0;
    motion.delta_y = 1.0;
    let right = Held {
        mouse: vec![BindingMouseButton::Right],
        ..Default::default()
    };
    let facing = apply_camera_input(&mut camera, Some(0.7), &bindings, &right, motion);
    near(camera.yaw, -1.0);
    near(camera.pitch, -0.8);
    near(facing.unwrap(), camera.yaw + std::f32::consts::PI);

    let left = Held {
        mouse: vec![BindingMouseButton::Left],
        ..Default::default()
    };
    let facing = apply_camera_input(&mut camera, Some(0.7), &bindings, &left, motion);
    near(camera.yaw, -2.0);
    near(camera.pitch, -1.3);
    near(facing.unwrap(), 0.7);
}

#[test]
fn both_mouse_buttons_take_right_button_facing_path() {
    let mut camera = CameraState::default();
    let held = Held {
        mouse: vec![BindingMouseButton::Left, BindingMouseButton::Right],
        ..Default::default()
    };
    let mut motion = input();
    motion.delta_x = 2.0;
    near(
        apply_camera_input(
            &mut camera,
            Some(1.0),
            &InputBindingsData::default(),
            &held,
            motion,
        )
        .unwrap(),
        std::f32::consts::PI - 1.0,
    );
    near(camera.yaw, -1.0);
}

#[test]
fn inverted_mouse_pitch_and_keyboard_pitch_clamp_at_88_degrees() {
    let mut camera = CameraState::default();
    camera.pitch = 87.0_f32.to_radians();
    let held = Held {
        mouse: vec![BindingMouseButton::Left],
        keys: vec![BindingKey::ArrowUp],
        ..Default::default()
    };
    let mut motion = input();
    motion.delta_y = 20.0;
    motion.invert_y = true;
    apply_camera_input(
        &mut camera,
        None,
        &InputBindingsData::default(),
        &held,
        motion,
    );
    near(camera.pitch, 88.0_f32.to_radians());

    let mut camera = CameraState::default();
    camera.pitch = -87.0_f32.to_radians();
    motion.invert_y = false;
    apply_camera_input(
        &mut camera,
        None,
        &InputBindingsData::default(),
        &held,
        motion,
    );
    near(camera.pitch, (-88.0_f32).to_radians() + 2.5);
}

#[test]
fn opposing_keyboard_actions_choose_positive_and_obey_binding_modifiers() {
    let mut bindings = InputBindingsData::default();
    bindings.assign(
        InputAction::TurnLeft,
        InputBinding::ShiftKeyboard(BindingKey::KeyQ),
    );
    let held = Held {
        keys: vec![
            BindingKey::KeyQ,
            BindingKey::ArrowRight,
            BindingKey::ArrowUp,
            BindingKey::ArrowDown,
            BindingKey::PageUp,
            BindingKey::PageDown,
        ],
        shift: true,
        ..Default::default()
    };
    let mut camera = CameraState::default();
    let facing = apply_camera_input(&mut camera, Some(0.25), &bindings, &held, input());
    near(camera.yaw, 2.5);
    near(facing.unwrap(), 2.75);
    near(camera.pitch, 88.0_f32.to_radians());
    near(camera.target_distance, 30.0);
}

#[test]
fn mouse_facing_precedes_keyboard_turn_and_wheel_follows_keyboard_zoom() {
    let held = Held {
        mouse: vec![BindingMouseButton::Right],
        keys: vec![BindingKey::ArrowLeft, BindingKey::PageDown],
        ..Default::default()
    };
    let mut camera = CameraState::default();
    camera.target_distance = 39.0;
    let mut motion = input();
    motion.delta_x = 2.0;
    motion.scroll_y = 3.0;
    let facing = apply_camera_input(
        &mut camera,
        Some(0.0),
        &InputBindingsData::default(),
        &held,
        motion,
    );
    near(camera.yaw, 1.5);
    near(facing.unwrap(), std::f32::consts::PI + 1.5);
    near(camera.target_distance, 34.0); // key clamps 39+15 to 40, then wheel subtracts 6
}

#[test]
fn absent_character_does_not_prevent_camera_input() {
    let held = Held {
        keys: vec![BindingKey::ArrowLeft],
        ..Default::default()
    };
    let mut camera = CameraState::default();
    assert_eq!(
        apply_camera_input(
            &mut camera,
            None,
            &InputBindingsData::default(),
            &held,
            input()
        ),
        None
    );
    near(camera.yaw, 2.5);
}
