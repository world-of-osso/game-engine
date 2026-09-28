use game_engine_core::movement_animation_data::direction_to_anim_id;
use game_engine_core::movement_input_data::MoveDirection;

#[test]
fn land_direction_animation_respects_running_only_when_forward() {
    for (direction, walking, running) in [
        (MoveDirection::None, 0, 0),
        (MoveDirection::Forward, 4, 5),
        (MoveDirection::Backward, 13, 13),
        (MoveDirection::Left, 11, 11),
        (MoveDirection::Right, 12, 12),
    ] {
        assert_eq!(direction_to_anim_id(direction, false, false), walking);
        assert_eq!(direction_to_anim_id(direction, true, false), running);
    }
}

#[test]
fn swimming_direction_animation_ignores_running() {
    for (direction, expected) in [
        (MoveDirection::None, 41),
        (MoveDirection::Forward, 42),
        (MoveDirection::Backward, 45),
        (MoveDirection::Left, 43),
        (MoveDirection::Right, 44),
    ] {
        for running in [false, true] {
            assert_eq!(direction_to_anim_id(direction, running, true), expected);
        }
    }
}
