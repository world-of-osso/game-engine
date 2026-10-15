//! Continuous, zero-radius body clearance for a grounded step.
use glam::{Vec2, Vec3};

/// TrinityCore Object.h DEFAULT_COLLISION_HEIGHT; matches server head clearance.
pub const BODY_HEIGHT: f32 = 2.03128;

/// Whether an authored triangle crosses the raised body's horizontal sweep.
/// Destination support faces are not obstacles, including when a sampled ray's
/// interpolated height rounds slightly below the supporting triangle's plane.
pub fn blocks_step_sweep(
    triangle: [Vec3; 3],
    from: Vec3,
    to: Vec3,
    support_top: f32,
    min_walkable_normal_y: f32,
) -> bool {
    if supports_destination(triangle, to, support_top, min_walkable_normal_y) {
        return false;
    }
    let movement = (to - from).with_y(0.0);
    let distance = movement.length();
    if distance == 0.0 {
        return false;
    }
    let direction = movement / distance;
    let mut section = intersect_body_plane(triangle, from, direction);
    for (axis, boundary, sign) in [
        (0, 0.0, 1.0),
        (0, distance, -1.0),
        (1, 0.0, 1.0),
        (1, BODY_HEIGHT, -1.0),
    ] {
        section = clip_section(&section, axis, boundary, sign);
    }
    let above_feet = section.iter().any(|point| point.y > 0.0);
    let below_head = section.iter().any(|point| point.y < BODY_HEIGHT);
    above_feet && below_head
}

fn supports_destination(triangle: [Vec3; 3], feet: Vec3, top: f32, min_normal_y: f32) -> bool {
    let [a, b, c] = triangle;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.y.abs() < min_normal_y {
        return false;
    }
    let height = a.y - (normal.x * (feet.x - a.x) + normal.z * (feet.z - a.z)) / normal.y;
    if height < feet.y || height > top {
        return false;
    }
    let point = feet.with_y(height);
    let edge_sides =
        [(a, b), (b, c), (c, a)].map(|(start, end)| (end - start).cross(point - start).dot(normal));
    edge_sides.iter().all(|side| *side >= 0.0) || edge_sides.iter().all(|side| *side <= 0.0)
}

fn intersect_body_plane(triangle: [Vec3; 3], from: Vec3, direction: Vec3) -> Vec<Vec2> {
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
            section.push(project(triangle[index].lerp(triangle[next], fraction)));
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
            clipped.push(from.lerp(to, start / (start - end)));
        }
    }
    clipped
}
