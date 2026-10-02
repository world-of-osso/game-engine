use super::*;

fn direct_path(start: Vec2, goal: Vec2) -> Vec<Vec2> {
    vec![start.lerp(goal, 0.5), goal]
}

#[test]
fn manual_movement_clears_the_waypoint() {
    let mut path = WaypointPath::default();
    let mut waypoint = Some((6.0, 0.0));

    let facing = path.sync(&mut waypoint, Vec2::ZERO, true, |_, _| Some(Vec::new()));

    assert_eq!(facing, None);
    assert!(path.active.is_none());
    assert_eq!(waypoint, None);
}

#[test]
fn a_new_waypoint_builds_a_path_and_faces_its_next_node() {
    let mut path = WaypointPath::default();
    let mut waypoint = Some((6.0, 0.0));

    let facing = path
        .sync(&mut waypoint, Vec2::ZERO, false, |_, goal| {
            Some(direct_path(Vec2::ZERO, goal))
        })
        .expect("a facing toward the first node");

    // +X is a quarter turn from +Z, the original's yaw convention.
    assert!((facing - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    assert_eq!(path.active.as_ref().map(|path| path.nodes.len()), Some(2));
    assert_eq!(waypoint, Some((6.0, 0.0)));
}

#[test]
fn reaching_the_goal_clears_the_waypoint() {
    let mut path = WaypointPath::default();
    let mut waypoint = Some((1.0, 1.0));

    let facing = path.sync(&mut waypoint, Vec2::new(1.0, 1.0), false, |_, _| {
        Some(Vec::new())
    });

    assert_eq!(facing, None);
    assert_eq!(waypoint, None);
}

#[test]
fn an_unreachable_goal_clears_the_waypoint() {
    let mut path = WaypointPath::default();
    let mut waypoint = Some((6.0, 0.0));

    let facing = path.sync(&mut waypoint, Vec2::ZERO, false, |_, _| None);

    assert_eq!(facing, None);
    assert_eq!(waypoint, None);
}

#[test]
fn passing_a_node_faces_the_next_one() {
    let mut path = WaypointPath::default();
    let mut waypoint = Some((6.0, 6.0));
    let nodes = vec![Vec2::new(6.0, 0.0), Vec2::new(6.0, 6.0)];
    path.sync(&mut waypoint, Vec2::ZERO, false, |_, _| Some(nodes.clone()));

    // Within the node radius of (6, 0): the next node is straight +Z.
    let facing = path
        .sync(&mut waypoint, Vec2::new(5.5, 0.0), false, |_, _| None)
        .expect("a facing toward the second node");

    assert!((facing - (0.5_f32).atan2(6.0)).abs() < 1e-4);
    assert_eq!(path.active.as_ref().map(|path| path.next_node), Some(1));
}

#[test]
fn the_grid_search_routes_around_a_blocked_segment() {
    let path = find_grid_path(Vec2::ZERO, Vec2::new(6.0, 0.0), 1.0, |start, end| {
        let blocked = (start.y.abs() < 0.1 && end.y.abs() < 0.1)
            && start.x.min(end.x) < 3.5
            && start.x.max(end.x) > 2.5;
        !blocked
    })
    .expect("a routed path");

    assert_eq!(path.last().copied(), Some(Vec2::new(6.0, 0.0)));
    assert!(path.iter().any(|point| point.y.abs() > 0.1));
}

#[test]
fn a_walkable_straight_line_is_one_node() {
    let path = find_grid_path(Vec2::ZERO, Vec2::new(6.0, 2.0), 1.5, |_, _| true);

    assert_eq!(path, Some(vec![Vec2::new(6.0, 2.0)]));
}
