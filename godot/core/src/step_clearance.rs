//! Exact low-wall interval check for a grounded step; no full-body collision.
use glam::{Vec2, Vec3};

use crate::player_physics_data::WALL_RAY_HEIGHT;

/// Reconstruct the admitted destination support plane before sweeping. Ground ray
/// interpolation can round feet below that plane; floor contact must not turn its
/// adjoining vertical edge into an obstruction. This uses authored support, not an epsilon.
pub fn step_wall_blocked(
    triangles: &[[Vec3; 3]],
    from: Vec3,
    to: Vec3,
    support_top: f32,
    min_walkable_normal_y: f32,
) -> bool {
    let support = triangles
        .iter()
        .filter_map(|&triangle| {
            destination_support_height(triangle, to, support_top, min_walkable_normal_y)
        })
        .fold(to.y, f32::max);
    triangles
        .iter()
        .any(|&triangle| blocks_step_sweep(triangle, from.with_y(support), to.with_y(support)))
}

/// Whether an authored triangle enters the interval below the existing low wall ray.
fn blocks_step_sweep(triangle: [Vec3; 3], from: Vec3, to: Vec3) -> bool {
    let movement = (to - from).with_y(0.0);
    let distance = movement.length();
    if distance == 0.0 {
        return false;
    }
    let direction = movement / distance;
    let mut section = intersect_wall_plane(triangle, from, direction);
    for (axis, boundary, sign) in [
        (0, 0.0, 1.0),
        (0, distance, -1.0),
        (1, 0.0, 1.0),
        (1, WALL_RAY_HEIGHT, -1.0),
    ] {
        section = clip_section(&section, axis, boundary, sign);
    }
    let above_feet = section.iter().any(|point| point.y > 0.0);
    let below_ray = section.iter().any(|point| point.y < WALL_RAY_HEIGHT);
    if above_feet && below_ray {
        eprintln!(
            "STEP_WALL_DIAGNOSTIC triangle={triangle:?} from={from:?} to={to:?} section={section:?}"
        );
    }
    above_feet && below_ray
}

fn destination_support_height(
    triangle: [Vec3; 3],
    feet: Vec3,
    top: f32,
    min_normal_y: f32,
) -> Option<f32> {
    let [a, b, c] = triangle;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.y.abs() < min_normal_y {
        return None;
    }
    let height = a.y - (normal.x * (feet.x - a.x) + normal.z * (feet.z - a.z)) / normal.y;
    if height < feet.y || height > top {
        return None;
    }
    let point = feet.with_y(height);
    let edge_sides =
        [(a, b), (b, c), (c, a)].map(|(start, end)| (end - start).cross(point - start).dot(normal));
    let inside =
        edge_sides.iter().all(|side| *side >= 0.0) || edge_sides.iter().all(|side| *side <= 0.0);
    inside.then_some(height)
}

fn intersect_wall_plane(triangle: [Vec3; 3], from: Vec3, direction: Vec3) -> Vec<Vec2> {
    let normal = direction.cross(Vec3::Y);
    let sides = triangle.map(|point| (point - from).dot(normal));
    let project = |point: Vec3| Vec2::new((point - from).dot(direction), point.y - from.y);
    let mut section = Vec::with_capacity(3);
    for index in 0..3 {
        let next = (index + 1) % 3;
        if sides[index] == 0.0 {
            section.push(project(triangle[index]));
        }
        if sides[index] * sides[next] < 0.0 {
            let fraction = sides[index] / (sides[index] - sides[next]);
            // Difference form preserves coordinates shared by both edge vertices
            // exactly (notably a flat support plane), unlike weighted-sum lerp.
            let point = triangle[index] + (triangle[next] - triangle[index]) * fraction;
            section.push(project(point));
        }
    }
    section
}

fn clip_section(points: &[Vec2], axis: usize, boundary: f32, sign: f32) -> Vec<Vec2> {
    let mut clipped = Vec::with_capacity(points.len() + 1);
    for index in 0..points.len() {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        let start = (from[axis] - boundary) * sign;
        let end = (to[axis] - boundary) * sign;
        if start >= 0.0 {
            clipped.push(from);
        }
        if start * end < 0.0 {
            let fraction = start / (start - end);
            let mut intersection = from + (to - from) * fraction;
            intersection[axis] = boundary;
            clipped.push(intersection);
        }
    }
    clipped
}
