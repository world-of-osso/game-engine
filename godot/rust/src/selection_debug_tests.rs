use game_engine_ui_model::selection_debug_component::SelectionDebugAction;
use godot::global::Key;

use super::{SelectionModel, key_action};

#[test]
fn cycling_wraps_both_directions() {
    let mut model = SelectionModel::default();
    assert!(model.apply(&SelectionDebugAction::Prev));
    assert_eq!(model.state.selected_index, 4);
    assert_eq!(model.state.last_action, "Focused World Object");
    assert!(model.apply(&SelectionDebugAction::Next));
    assert_eq!(model.state.selected_index, 0);
    assert_eq!(model.state.last_action, "Focused Local Player");
}

#[test]
fn row_selection_and_pinning_name_the_candidate() {
    let mut model = SelectionModel::default();
    model.apply(&SelectionDebugAction::SelectEntry(2));
    assert_eq!(model.state.selected_index, 2);
    assert_eq!(model.state.last_action, "Selected Enemy Creature");
    model.apply(&SelectionDebugAction::SelectEntry(9));
    assert_eq!(model.state.selected_index, 2);
    model.apply(&SelectionDebugAction::TogglePinned);
    assert!(model.state.pinned);
    assert_eq!(model.state.last_action, "Pinned Enemy Creature");
    model.apply(&SelectionDebugAction::TogglePinned);
    assert_eq!(model.state.last_action, "Unpinned Enemy Creature");
}

#[test]
fn back_leaves_and_keys_follow_the_original_bindings() {
    let mut model = SelectionModel::default();
    assert!(!model.apply(&SelectionDebugAction::Back));
    assert_eq!(key_action(Key::UP), Some(SelectionDebugAction::Prev));
    assert_eq!(key_action(Key::LEFT), Some(SelectionDebugAction::Prev));
    assert_eq!(key_action(Key::DOWN), Some(SelectionDebugAction::Next));
    assert_eq!(key_action(Key::RIGHT), Some(SelectionDebugAction::Next));
    assert_eq!(key_action(Key::ENTER), Some(SelectionDebugAction::TogglePinned));
    assert_eq!(key_action(Key::SPACE), Some(SelectionDebugAction::TogglePinned));
    assert_eq!(key_action(Key::ESCAPE), Some(SelectionDebugAction::Back));
    assert_eq!(key_action(Key::A), None);
}
