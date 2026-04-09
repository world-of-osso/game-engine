use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use game_engine::status::{MapStatusSnapshot, Waypoint};

use crate::collision::{
    clamp_movement_against_doodad_colliders, clamp_movement_against_wmo_meshes,
    validate_movement_slope,
};
use crate::terrain_heightmap::TerrainHeightmap;

const PATH_GRID_STEP: f32 = 1.5;
const PATH_GOAL_REACHED_RADIUS: f32 = 0.8;
const PATH_NODE_REACHED_RADIUS: f32 = 0.9;
const PATH_EDGE_SAMPLE_STEP: f32 = 1.0;
const PATH_REBUILD_MARGIN_STEPS: i32 = 8;
const PATH_MAX_EXPANSIONS: usize = 4096;
const PATH_POSITION_TOLERANCE: f32 = 0.05;

#[derive(Resource, Default)]
pub struct PathingState {
    active: Option<ActivePath>,
}

#[derive(Clone, Debug, PartialEq)]
struct ActivePath {
    waypoint: Waypoint,
    nodes: Vec<Vec2>,
    next_node: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathingDirective {
    pub facing_yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenCell {
    cell: IVec2,
    estimated_total_cost: f32,
}

#[derive(Clone, Copy)]
struct SearchBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl Eq for OpenCell {}

impl Ord for OpenCell {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total_cost
            .total_cmp(&self.estimated_total_cost)
            .then_with(|| self.cell.x.cmp(&other.cell.x))
            .then_with(|| self.cell.y.cmp(&other.cell.y))
    }
}

impl PartialOrd for OpenCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn update_waypoint_pathing(
    state: &mut PathingState,
    map_status: &mut MapStatusSnapshot,
    player_position: Vec3,
    terrain: Option<&TerrainHeightmap>,
    ray_cast: &mut MeshRayCast,
    collision_meshes: &HashSet<Entity>,
    doodad_colliders: &[(Vec3, Vec3)],
    manual_override: bool,
) -> Option<PathingDirective> {
    let Some(terrain) = terrain else {
        state.active = None;
        return None;
    };
    let current_xz = Vec2::new(player_position.x, player_position.z);
    sync_path_state(
        state,
        &mut map_status.waypoint,
        current_xz,
        manual_override,
        |start, goal| {
            build_terrain_path(
                start,
                goal,
                terrain,
                ray_cast,
                collision_meshes,
                doodad_colliders,
            )
        },
    )
}

fn sync_path_state<F>(
    state: &mut PathingState,
    waypoint: &mut Option<Waypoint>,
    current_xz: Vec2,
    manual_override: bool,
    rebuild_path: F,
) -> Option<PathingDirective>
where
    F: FnOnce(Vec2, Vec2) -> Option<Vec<Vec2>>,
{
    if manual_override {
        clear_active_waypoint(state, waypoint);
        return None;
    }

    let Some(target_waypoint) = *waypoint else {
        state.active = None;
        return None;
    };
    let goal = waypoint_to_vec2(target_waypoint);
    if current_xz.distance(goal) <= PATH_GOAL_REACHED_RADIUS {
        clear_active_waypoint(state, waypoint);
        return None;
    }

    prepare_active_path(
        state,
        waypoint,
        current_xz,
        target_waypoint,
        goal,
        rebuild_path,
    )?;

    let active = state
        .active
        .as_mut()
        .expect("active path should exist after rebuild");
    advance_path_progress(active, current_xz);
    active_path_directive(active, current_xz).or_else(|| {
        clear_active_waypoint(state, waypoint);
        None
    })
}

fn clear_active_waypoint(state: &mut PathingState, waypoint: &mut Option<Waypoint>) {
    state.active = None;
    *waypoint = None;
}

fn prepare_active_path<F>(
    state: &mut PathingState,
    waypoint: &mut Option<Waypoint>,
    current_xz: Vec2,
    target_waypoint: Waypoint,
    goal: Vec2,
    rebuild_path: F,
) -> Option<()>
where
    F: FnOnce(Vec2, Vec2) -> Option<Vec<Vec2>>,
{
    if state
        .active
        .as_ref()
        .is_some_and(|active| active.waypoint == target_waypoint)
    {
        return Some(());
    }

    let nodes = rebuild_path(current_xz, goal).or_else(|| {
        clear_active_waypoint(state, waypoint);
        None
    })?;
    state.active = Some(ActivePath {
        waypoint: target_waypoint,
        nodes,
        next_node: 0,
    });
    Some(())
}

fn active_path_directive(path: &ActivePath, current_xz: Vec2) -> Option<PathingDirective> {
    let target_node = path.nodes.get(path.next_node).copied()?;
    direction_to_node(current_xz, target_node)
}

fn direction_to_node(current_xz: Vec2, target_node: Vec2) -> Option<PathingDirective> {
    let to_target = target_node - current_xz;
    if to_target.length_squared() <= f32::EPSILON {
        return None;
    }
    Some(PathingDirective {
        facing_yaw: to_target.x.atan2(to_target.y),
    })
}

fn advance_path_progress(path: &mut ActivePath, current_xz: Vec2) {
    while let Some(node) = path.nodes.get(path.next_node) {
        if current_xz.distance(*node) > PATH_NODE_REACHED_RADIUS {
            break;
        }
        path.next_node += 1;
    }
}

fn waypoint_to_vec2(waypoint: Waypoint) -> Vec2 {
    Vec2::new(waypoint.x, waypoint.y)
}

fn build_terrain_path(
    start: Vec2,
    goal: Vec2,
    terrain: &TerrainHeightmap,
    ray_cast: &mut MeshRayCast,
    collision_meshes: &HashSet<Entity>,
    doodad_colliders: &[(Vec3, Vec3)],
) -> Option<Vec<Vec2>> {
    find_grid_path(start, goal, PATH_GRID_STEP, |edge_start, edge_end| {
        terrain_segment_is_walkable(
            edge_start,
            edge_end,
            terrain,
            ray_cast,
            collision_meshes,
            doodad_colliders,
        )
    })
}

fn terrain_segment_is_walkable(
    start: Vec2,
    end: Vec2,
    terrain: &TerrainHeightmap,
    ray_cast: &mut MeshRayCast,
    collision_meshes: &HashSet<Entity>,
    doodad_colliders: &[(Vec3, Vec3)],
) -> bool {
    let distance = start.distance(end);
    let steps = (distance / PATH_EDGE_SAMPLE_STEP).ceil().max(1.0) as usize;
    let mut previous = start;
    for index in 1..=steps {
        let t = index as f32 / steps as f32;
        let sample = start.lerp(end, t);
        if !terrain_step_is_walkable(
            previous,
            sample,
            terrain,
            ray_cast,
            collision_meshes,
            doodad_colliders,
        ) {
            return false;
        }
        previous = sample;
    }
    true
}

fn terrain_step_is_walkable(
    start: Vec2,
    end: Vec2,
    terrain: &TerrainHeightmap,
    ray_cast: &mut MeshRayCast,
    collision_meshes: &HashSet<Entity>,
    doodad_colliders: &[(Vec3, Vec3)],
) -> bool {
    let Some(start_y) = terrain.height_at(start.x, start.y) else {
        return false;
    };
    let Some(end_y) = terrain.height_at(end.x, end.y) else {
        return false;
    };
    let current = Vec3::new(start.x, start_y, start.y);
    let proposed = Vec3::new(end.x, end_y, end.y);

    let slope_checked = validate_movement_slope(current, proposed, terrain, true);
    if position_error(slope_checked, proposed) > PATH_POSITION_TOLERANCE {
        return false;
    }

    let after_wmo =
        clamp_movement_against_wmo_meshes(current, proposed, ray_cast, collision_meshes);
    if position_error(after_wmo, proposed) > PATH_POSITION_TOLERANCE {
        return false;
    }

    let after_doodad = clamp_movement_against_doodad_colliders(current, proposed, doodad_colliders);
    if position_error(after_doodad, proposed) > PATH_POSITION_TOLERANCE {
        return false;
    }

    true
}

fn position_error(actual: Vec3, expected: Vec3) -> f32 {
    Vec2::new(actual.x - expected.x, actual.z - expected.z).length()
}

fn find_grid_path<F>(
    start: Vec2,
    goal: Vec2,
    step: f32,
    mut edge_is_walkable: F,
) -> Option<Vec<Vec2>>
where
    F: FnMut(Vec2, Vec2) -> bool,
{
    if edge_is_walkable(start, goal) {
        return Some(vec![goal]);
    }

    let goal_cell = goal_cell(start, goal, step);
    let bounds = search_bounds(goal_cell);
    let start_cell = IVec2::ZERO;

    let mut open = BinaryHeap::new();
    let mut came_from = HashMap::new();
    let mut cost_so_far = HashMap::new();
    cost_so_far.insert(start_cell, 0.0);
    open.push(OpenCell {
        cell: start_cell,
        estimated_total_cost: start.distance(goal),
    });

    let mut expansions = 0usize;
    while let Some(OpenCell { cell, .. }) = open.pop() {
        if search_exhausted(&mut expansions) {
            return None;
        }

        let cell_world = cell_to_world(start, step, cell);
        if can_reach_goal(cell, goal_cell, cell_world, goal, &mut edge_is_walkable) {
            return reconstruct_path(start, goal, step, cell, &came_from, &mut edge_is_walkable);
        }

        for neighbor in neighbors(cell) {
            update_neighbor(
                neighbor,
                cell,
                cell_world,
                goal,
                start,
                step,
                bounds,
                &mut cost_so_far,
                &mut came_from,
                &mut open,
                &mut edge_is_walkable,
            );
        }
    }

    None
}

fn goal_cell(start: Vec2, goal: Vec2, step: f32) -> IVec2 {
    let goal_delta = goal - start;
    IVec2::new(
        (goal_delta.x / step).round() as i32,
        (goal_delta.y / step).round() as i32,
    )
}

fn search_bounds(goal_cell: IVec2) -> SearchBounds {
    SearchBounds {
        min_x: goal_cell.x.min(0) - PATH_REBUILD_MARGIN_STEPS,
        max_x: goal_cell.x.max(0) + PATH_REBUILD_MARGIN_STEPS,
        min_y: goal_cell.y.min(0) - PATH_REBUILD_MARGIN_STEPS,
        max_y: goal_cell.y.max(0) + PATH_REBUILD_MARGIN_STEPS,
    }
}

fn search_exhausted(expansions: &mut usize) -> bool {
    *expansions += 1;
    *expansions > PATH_MAX_EXPANSIONS
}

fn can_reach_goal<F>(
    cell: IVec2,
    goal_cell: IVec2,
    cell_world: Vec2,
    goal: Vec2,
    edge_is_walkable: &mut F,
) -> bool
where
    F: FnMut(Vec2, Vec2) -> bool,
{
    cell == goal_cell && edge_is_walkable(cell_world, goal)
}

fn update_neighbor<F>(
    neighbor: IVec2,
    cell: IVec2,
    cell_world: Vec2,
    goal: Vec2,
    start: Vec2,
    step: f32,
    bounds: SearchBounds,
    cost_so_far: &mut HashMap<IVec2, f32>,
    came_from: &mut HashMap<IVec2, IVec2>,
    open: &mut BinaryHeap<OpenCell>,
    edge_is_walkable: &mut F,
) where
    F: FnMut(Vec2, Vec2) -> bool,
{
    if !bounds.contains(neighbor) {
        return;
    }

    let neighbor_world = cell_to_world(start, step, neighbor);
    if !edge_is_walkable(cell_world, neighbor_world) {
        return;
    }

    let new_cost = cost_so_far.get(&cell).copied().unwrap_or(f32::INFINITY)
        + cell_world.distance(neighbor_world);
    if cost_so_far
        .get(&neighbor)
        .is_some_and(|cost| *cost <= new_cost)
    {
        return;
    }

    cost_so_far.insert(neighbor, new_cost);
    came_from.insert(neighbor, cell);
    open.push(OpenCell {
        cell: neighbor,
        estimated_total_cost: new_cost + neighbor_world.distance(goal),
    });
}

impl SearchBounds {
    fn contains(self, cell: IVec2) -> bool {
        cell.x >= self.min_x && cell.x <= self.max_x && cell.y >= self.min_y && cell.y <= self.max_y
    }
}

fn reconstruct_path<F>(
    start: Vec2,
    goal: Vec2,
    step: f32,
    goal_cell: IVec2,
    came_from: &HashMap<IVec2, IVec2>,
    edge_is_walkable: &mut F,
) -> Option<Vec<Vec2>>
where
    F: FnMut(Vec2, Vec2) -> bool,
{
    let mut cells = vec![goal_cell];
    let mut current = goal_cell;
    while let Some(previous) = came_from.get(&current).copied() {
        current = previous;
        if current == IVec2::ZERO {
            break;
        }
        cells.push(current);
    }
    cells.reverse();

    let mut raw = cells
        .into_iter()
        .map(|cell| cell_to_world(start, step, cell))
        .collect::<Vec<_>>();
    if raw.last().copied() != Some(goal) {
        raw.push(goal);
    }

    smooth_path(start, raw, edge_is_walkable)
}

fn smooth_path<F>(start: Vec2, raw: Vec<Vec2>, edge_is_walkable: &mut F) -> Option<Vec<Vec2>>
where
    F: FnMut(Vec2, Vec2) -> bool,
{
    let mut smoothed = Vec::new();
    let mut anchor = start;
    let mut index = 0usize;
    while index < raw.len() {
        let mut furthest = index;
        while furthest + 1 < raw.len() && edge_is_walkable(anchor, raw[furthest + 1]) {
            furthest += 1;
        }
        let node = raw[furthest];
        if !edge_is_walkable(anchor, node) {
            return None;
        }
        smoothed.push(node);
        anchor = node;
        index = furthest + 1;
    }
    Some(smoothed)
}

fn cell_to_world(start: Vec2, step: f32, cell: IVec2) -> Vec2 {
    start + Vec2::new(cell.x as f32 * step, cell.y as f32 * step)
}

fn neighbors(cell: IVec2) -> [IVec2; 8] {
    [
        cell + IVec2::new(-1, -1),
        cell + IVec2::new(0, -1),
        cell + IVec2::new(1, -1),
        cell + IVec2::new(-1, 0),
        cell + IVec2::new(1, 0),
        cell + IVec2::new(-1, 1),
        cell + IVec2::new(0, 1),
        cell + IVec2::new(1, 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn direct_path(start: Vec2, goal: Vec2) -> Vec<Vec2> {
        vec![start.lerp(goal, 0.5), goal]
    }

    #[test]
    fn sync_path_state_clears_waypoint_on_manual_override() {
        let mut state = PathingState::default();
        let mut waypoint = Some(Waypoint { x: 6.0, y: 0.0 });

        let directive = sync_path_state(
            &mut state,
            &mut waypoint,
            Vec2::ZERO,
            true,
            |_start, _goal| Some(Vec::new()),
        );

        assert!(directive.is_none());
        assert!(state.active.is_none());
        assert!(waypoint.is_none());
    }

    #[test]
    fn sync_path_state_rebuilds_and_faces_next_node() {
        let mut state = PathingState::default();
        let mut waypoint = Some(Waypoint { x: 6.0, y: 0.0 });

        let directive = sync_path_state(
            &mut state,
            &mut waypoint,
            Vec2::ZERO,
            false,
            |_start, goal| Some(direct_path(Vec2::ZERO, goal)),
        )
        .expect("expected directive");

        assert!((directive.facing_yaw - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
        assert_eq!(state.active.as_ref().map(|path| path.nodes.len()), Some(2));
    }

    #[test]
    fn sync_path_state_clears_waypoint_when_goal_is_reached() {
        let mut state = PathingState::default();
        let mut waypoint = Some(Waypoint { x: 1.0, y: 1.0 });

        let directive = sync_path_state(
            &mut state,
            &mut waypoint,
            Vec2::new(1.0, 1.0),
            false,
            |_start, _goal| Some(Vec::new()),
        );

        assert!(directive.is_none());
        assert!(waypoint.is_none());
    }

    #[test]
    fn find_grid_path_routes_around_blocked_segment() {
        let path = find_grid_path(Vec2::ZERO, Vec2::new(6.0, 0.0), 1.0, |start, end| {
            let blocked = (start.y.abs() < 0.1 && end.y.abs() < 0.1)
                && start.x.min(end.x) < 3.5
                && start.x.max(end.x) > 2.5;
            !blocked
        })
        .expect("expected a routed path");

        assert_eq!(path.last().copied(), Some(Vec2::new(6.0, 0.0)));
        assert!(path.iter().any(|point| point.y.abs() > 0.1));
    }
}
