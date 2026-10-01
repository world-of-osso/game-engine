//! IPC `map waypoint add` auto-walk: the original's terrain path search and following
//! (`src/pathing.rs`). A path is a grid search around blocked segments, smoothed to the
//! furthest walkable node; each frame the player faces the next node and walks forward
//! until within the goal radius, which clears the waypoint. Manual movement input
//! clears it too.
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use glam::{IVec2, Vec2, Vec3};

use crate::ground::TerrainGround;
use crate::terrain::streaming::StreamedTerrain;

const PATH_GRID_STEP: f32 = 1.5;
const PATH_GOAL_REACHED_RADIUS: f32 = 0.8;
const PATH_NODE_REACHED_RADIUS: f32 = 0.9;
const PATH_EDGE_SAMPLE_STEP: f32 = 1.0;
const PATH_REBUILD_MARGIN_STEPS: i32 = 8;
const PATH_MAX_EXPANSIONS: usize = 4096;
const PATH_POSITION_TOLERANCE: f32 = 0.05;

/// The path being followed to the current waypoint (world x, z).
#[derive(Default)]
pub(crate) struct WaypointPath {
    active: Option<ActivePath>,
}

#[derive(Clone, Debug, PartialEq)]
struct ActivePath {
    waypoint: (f32, f32),
    nodes: Vec<Vec2>,
    next_node: usize,
}

impl WaypointPath {
    pub fn clear(&mut self) {
        self.active = None;
    }

    /// This frame's facing toward the next node, or `None` when no waypoint is being
    /// walked to; reaching the goal, an unreachable goal or `manual_override` clears
    /// `waypoint`.
    pub fn follow(
        &mut self,
        waypoint: &mut Option<(f32, f32)>,
        position: Vec3,
        manual_override: bool,
        terrain: &StreamedTerrain,
        ground: &TerrainGround,
    ) -> Option<f32> {
        self.sync(
            waypoint,
            Vec2::new(position.x, position.z),
            manual_override,
            |start, goal| {
                find_grid_path(start, goal, PATH_GRID_STEP, |from, to| {
                    segment_is_walkable(from, to, terrain, ground)
                })
            },
        )
    }

    fn sync(
        &mut self,
        waypoint: &mut Option<(f32, f32)>,
        current: Vec2,
        manual_override: bool,
        rebuild_path: impl FnOnce(Vec2, Vec2) -> Option<Vec<Vec2>>,
    ) -> Option<f32> {
        if manual_override {
            self.active = None;
            *waypoint = None;
            return None;
        }
        let Some(target) = *waypoint else {
            self.active = None;
            return None;
        };
        let goal = Vec2::new(target.0, target.1);
        if current.distance(goal) <= PATH_GOAL_REACHED_RADIUS {
            self.active = None;
            *waypoint = None;
            return None;
        }
        if self.active.as_ref().is_none_or(|path| path.waypoint != target) {
            let Some(nodes) = rebuild_path(current, goal) else {
                self.active = None;
                *waypoint = None;
                return None;
            };
            self.active = Some(ActivePath {
                waypoint: target,
                nodes,
                next_node: 0,
            });
        }
        let path = self.active.as_mut().expect("path built above");
        while path
            .nodes
            .get(path.next_node)
            .is_some_and(|node| current.distance(*node) <= PATH_NODE_REACHED_RADIUS)
        {
            path.next_node += 1;
        }
        let facing = path
            .nodes
            .get(path.next_node)
            .map(|node| *node - current)
            .filter(|to_node| to_node.length_squared() > f32::EPSILON)
            .map(|to_node| to_node.x.atan2(to_node.y));
        if facing.is_none() {
            self.active = None;
            *waypoint = None;
        }
        facing
    }
}

/// Terrain steps of at most `PATH_EDGE_SAMPLE_STEP` that the movement rules (walls,
/// slope, step height) let through unchanged.
fn segment_is_walkable(
    start: Vec2,
    end: Vec2,
    terrain: &StreamedTerrain,
    ground: &TerrainGround,
) -> bool {
    let steps = (start.distance(end) / PATH_EDGE_SAMPLE_STEP).ceil().max(1.0) as usize;
    let mut previous = start;
    (1..=steps).all(|index| {
        let sample = start.lerp(end, index as f32 / steps as f32);
        let walkable = step_is_walkable(previous, sample, terrain, ground);
        previous = sample;
        walkable
    })
}

fn step_is_walkable(
    start: Vec2,
    end: Vec2,
    terrain: &StreamedTerrain,
    ground: &TerrainGround,
) -> bool {
    let (Some(start_y), Some(end_y)) = (
        terrain.height_at(start.x, start.y),
        terrain.height_at(end.x, end.y),
    ) else {
        return false;
    };
    let current = Vec3::new(start.x, start_y, start.y);
    let proposed = Vec3::new(end.x, end_y, end.y);
    let allowed = ground.validate_move(current, proposed, true);
    Vec2::new(allowed.x - proposed.x, allowed.z - proposed.z).length() <= PATH_POSITION_TOLERANCE
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenCell {
    cell: IVec2,
    estimated_total_cost: f32,
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

/// A* over a `step` grid anchored at `start`, within a margin around the start-goal
/// box; a direct walkable segment is the whole path.
fn find_grid_path(
    start: Vec2,
    goal: Vec2,
    step: f32,
    mut walkable: impl FnMut(Vec2, Vec2) -> bool,
) -> Option<Vec<Vec2>> {
    if walkable(start, goal) {
        return Some(vec![goal]);
    }
    let delta = goal - start;
    let goal_cell = IVec2::new(
        (delta.x / step).round() as i32,
        (delta.y / step).round() as i32,
    );
    let min = goal_cell.min(IVec2::ZERO) - IVec2::splat(PATH_REBUILD_MARGIN_STEPS);
    let max = goal_cell.max(IVec2::ZERO) + IVec2::splat(PATH_REBUILD_MARGIN_STEPS);
    let to_world = |cell: IVec2| start + cell.as_vec2() * step;
    let mut open = BinaryHeap::from([OpenCell {
        cell: IVec2::ZERO,
        estimated_total_cost: start.distance(goal),
    }]);
    let mut came_from = HashMap::new();
    let mut cost_so_far = HashMap::from([(IVec2::ZERO, 0.0_f32)]);
    let mut expansions = 0;
    while let Some(OpenCell { cell, .. }) = open.pop() {
        expansions += 1;
        if expansions > PATH_MAX_EXPANSIONS {
            return None;
        }
        let cell_world = to_world(cell);
        if cell == goal_cell && walkable(cell_world, goal) {
            return reconstruct_path(start, goal, step, cell, &came_from, &mut walkable);
        }
        for offset in NEIGHBORS {
            let neighbor = cell + offset;
            if neighbor.cmplt(min).any() || neighbor.cmpgt(max).any() {
                continue;
            }
            let neighbor_world = to_world(neighbor);
            if !walkable(cell_world, neighbor_world) {
                continue;
            }
            let cost = cost_so_far[&cell] + cell_world.distance(neighbor_world);
            if cost_so_far.get(&neighbor).is_some_and(|known| *known <= cost) {
                continue;
            }
            cost_so_far.insert(neighbor, cost);
            came_from.insert(neighbor, cell);
            open.push(OpenCell {
                cell: neighbor,
                estimated_total_cost: cost + neighbor_world.distance(goal),
            });
        }
    }
    None
}

const NEIGHBORS: [IVec2; 8] = [
    IVec2::new(-1, -1),
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(-1, 0),
    IVec2::new(1, 0),
    IVec2::new(-1, 1),
    IVec2::new(0, 1),
    IVec2::new(1, 1),
];

fn reconstruct_path(
    start: Vec2,
    goal: Vec2,
    step: f32,
    goal_cell: IVec2,
    came_from: &HashMap<IVec2, IVec2>,
    walkable: &mut impl FnMut(Vec2, Vec2) -> bool,
) -> Option<Vec<Vec2>> {
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
    let mut raw: Vec<Vec2> = cells
        .into_iter()
        .map(|cell| start + cell.as_vec2() * step)
        .collect();
    if raw.last().copied() != Some(goal) {
        raw.push(goal);
    }
    smooth_path(start, &raw, walkable)
}

/// Each node is the furthest raw node walkable from the previous one.
fn smooth_path(
    start: Vec2,
    raw: &[Vec2],
    walkable: &mut impl FnMut(Vec2, Vec2) -> bool,
) -> Option<Vec<Vec2>> {
    let mut smoothed = Vec::new();
    let mut anchor = start;
    let mut index = 0;
    while index < raw.len() {
        let mut furthest = index;
        while furthest + 1 < raw.len() && walkable(anchor, raw[furthest + 1]) {
            furthest += 1;
        }
        let node = raw[furthest];
        if !walkable(anchor, node) {
            return None;
        }
        smoothed.push(node);
        anchor = node;
        index = furthest + 1;
    }
    Some(smoothed)
}

#[cfg(test)]
#[path = "waypoint_path_tests.rs"]
mod tests;
