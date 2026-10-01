use game_engine_ui_model::container_layout_data::container_positions;
use game_engine_ui_model::window_manager::{WindowClass, WindowId, WindowManager};

#[test]
fn single_bag_clears_right_edge_and_bottom_bar() {
    assert_eq!(
        container_positions(&[(0, [200.0, 300.0])], [1280.0, 720.0]),
        vec![(0, [1064.0, 324.0])]
    );
}

#[test]
fn sorted_bags_stack_upward_with_eight_unit_gap() {
    assert_eq!(
        container_positions(&[(0, [200.0, 180.0]), (3, [160.0, 220.0])], [1280.0, 720.0]),
        vec![(0, [1064.0, 444.0]), (3, [1104.0, 216.0])]
    );
}

#[test]
fn crossing_top_threshold_starts_column_left_of_widest_bag() {
    assert_eq!(
        container_positions(
            &[
                (0, [180.0, 220.0]),
                (1, [240.0, 220.0]),
                (4, [160.0, 100.0])
            ],
            [1280.0, 720.0],
        ),
        vec![
            (0, [1084.0, 404.0]),
            (1, [1024.0, 176.0]),
            (4, [856.0, 524.0])
        ]
    );
}

#[test]
fn bag_exactly_at_top_threshold_remains_in_column() {
    assert_eq!(
        container_positions(&[(0, [200.0, 220.0]), (1, [160.0, 292.0])], [1280.0, 720.0]),
        vec![(0, [1064.0, 404.0]), (1, [1104.0, 104.0])]
    );
}

#[test]
fn oversized_bags_and_offscreen_columns_clamp_to_viewport() {
    assert_eq!(
        container_positions(&[(0, [500.0, 600.0]), (1, [100.0, 200.0])], [320.0, 240.0]),
        vec![(0, [0.0, 0.0]), (1, [0.0, 0.0])]
    );
    assert!(container_positions(&[], [1280.0, 720.0]).is_empty());
}

#[test]
fn bag_toggle_uses_original_manager_open_state_and_raise_rank() {
    let mut manager = WindowManager::default();
    assert_eq!(WindowId::Bag(2).class(), WindowClass::Container);
    assert!(manager.toggle(WindowId::Bag(2)));
    let initial_rank = manager.raise_rank(WindowId::Bag(2)).unwrap();
    manager.open(WindowId::Bag(2));
    assert_eq!(manager.open_windows(), &[WindowId::Bag(2)]);
    assert!(manager.raise_rank(WindowId::Bag(2)).unwrap() > initial_rank);
    assert!(!manager.toggle(WindowId::Bag(2)));
    assert!(!manager.any_open());
    assert_eq!(manager.raise_rank(WindowId::Bag(2)), None);
}

#[test]
fn close_all_clears_bags_panels_and_stacking() {
    let mut manager = WindowManager::default();
    manager.open(WindowId::Bag(0));
    manager.open(WindowId::Merchant);
    manager.open(WindowId::Bag(4));
    assert!(manager.close_all());
    assert!(manager.open_windows().is_empty());
    assert_eq!(manager.panel_in_slot(0), None);
    assert_eq!(manager.raise_rank(WindowId::Merchant), None);
    assert_eq!(manager.raise_rank(WindowId::Bag(0)), None);
    assert_eq!(manager.raise_rank(WindowId::Bag(4)), None);
    assert!(!manager.close_all());
}

#[test]
fn containers_coexist_with_panel_and_wide_window_transitions() {
    let mut manager = WindowManager::default();
    manager.open(WindowId::Bag(0));
    manager.open(WindowId::Character);
    manager.open(WindowId::Merchant);
    assert_eq!(manager.panel_in_slot(0), Some(WindowId::Merchant));
    assert_eq!(manager.panel_in_slot(1), Some(WindowId::Character));
    manager.open(WindowId::WorldMap);
    assert_eq!(
        manager.open_windows(),
        &[WindowId::Bag(0), WindowId::WorldMap]
    );
    manager.open(WindowId::Bag(3));
    manager.open(WindowId::Mail);
    assert_eq!(
        manager.open_windows(),
        &[WindowId::Bag(0), WindowId::Bag(3), WindowId::Mail]
    );
    assert_eq!(manager.panel_in_slot(0), Some(WindowId::Mail));
}
