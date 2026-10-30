//! Exact low-wall interval check for a grounded step; no full-body collision.
use glam::{Vec2, Vec3};

use crate::player_physics_data::WALL_RAY_HEIGHT;

// The authored support and live Godot shapes use separate floating-point transforms.
const SUPPORT_FACE_TOLERANCE: f32 = 0.001;

/// Walkable faces supporting the destination are floors, not wall obstructions.
/// Reconstruct their support plane for the adjoining vertical edges: ground-ray
/// interpolation can round feet below the authored plane. No epsilon is used.
/// Identical endpoints test the vertical headroom above the landing instead.
pub fn step_wall_blocked(
    triangles: &[[Vec3; 3]],
    from: Vec3,
    to: Vec3,
    support_top: f32,
    min_walkable_normal_y: f32,
) -> bool {
    let support = if from.x == to.x && from.z == to.z {
        to.y
    } else {
        step_sweep_support(triangles, to, support_top, min_walkable_normal_y)
    };
    triangles.iter().any(|&triangle| {
        triangle_blocks_step(
            triangle,
            from,
            to,
            support_top,
            min_walkable_normal_y,
            support,
        )
    })
}

/// Indices and landing-column heights for the faces that block the same live query.
pub fn step_wall_blocking_triangles(
    triangles: &[[Vec3; 3]],
    from: Vec3,
    to: Vec3,
    support_top: f32,
    min_walkable_normal_y: f32,
) -> Vec<(usize, Option<f32>)> {
    let support = if from.x == to.x && from.z == to.z {
        to.y
    } else {
        step_sweep_support(triangles, to, support_top, min_walkable_normal_y)
    };
    triangles
        .iter()
        .enumerate()
        .filter_map(|(index, &triangle)| {
            triangle_blocks_step(
                triangle,
                from,
                to,
                support_top,
                min_walkable_normal_y,
                support,
            )
            .then(|| {
                (
                    index,
                    destination_support_height(triangle, to, f32::INFINITY, f32::MIN_POSITIVE),
                )
            })
        })
        .collect()
}

fn step_sweep_support(
    triangles: &[[Vec3; 3]],
    to: Vec3,
    support_top: f32,
    min_normal_y: f32,
) -> f32 {
    triangles
        .iter()
        .filter_map(|&triangle| destination_support_height(triangle, to, support_top, min_normal_y))
        .fold(to.y, f32::max)
}

fn triangle_blocks_step(
    triangle: [Vec3; 3],
    from: Vec3,
    to: Vec3,
    support_top: f32,
    min_normal_y: f32,
    support: f32,
) -> bool {
    if from.x == to.x && from.z == to.z {
        return destination_support_height(
            triangle,
            from,
            from.y + WALL_RAY_HEIGHT,
            f32::MIN_POSITIVE,
        )
        .is_some_and(|height| {
            let is_landing_face = (height - from.y).abs() <= SUPPORT_FACE_TOLERANCE;
            !is_landing_face && height > from.y && height < from.y + WALL_RAY_HEIGHT
        });
    }
    let supports_landing =
        destination_support_height(triangle, to, support_top, min_normal_y).is_some();
    !supports_landing && blocks_step_sweep(triangle, from.with_y(support), to.with_y(support))
}

/// Find the top of the solid faces in a horizontal step corridor.
/// A face extending above `max_top` cannot be stepped over.
pub fn step_obstacle_top(
    triangles: impl IntoIterator<Item = [Vec3; 3]>,
    from: Vec3,
    to: Vec3,
    max_top: f32,
) -> Option<f32> {
    let movement = (to - from).with_y(0.0);
    let distance = movement.length();
    if distance == 0.0 {
        return None;
    }
    let direction = movement / distance;
    let mut top = from.y;
    let min = from.min(to);
    let max = from.max(to);
    for triangle in triangles {
        if triangle.iter().all(|point| point.x < min.x)
            || triangle.iter().all(|point| point.x > max.x)
            || triangle.iter().all(|point| point.z < min.z)
            || triangle.iter().all(|point| point.z > max.z)
        {
            continue;
        }
        let section = intersect_wall_plane(triangle, from, direction);
        let section = clip_section(&section, 0, 0.0, 1.0);
        let section = clip_section(&section, 0, distance, -1.0);
        if section.is_empty() {
            continue;
        }
        let bottom = section
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        let height = section
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max);
        if bottom > max_top - from.y || height <= 0.0 {
            continue;
        }
        top = top.max(from.y + height);
        if top > max_top {
            return None;
        }
    }
    Some(top)
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
    let above_feet = section.iter().any(|point| point.y > SUPPORT_FACE_TOLERANCE);
    let below_ray = section.iter().any(|point| point.y < WALL_RAY_HEIGHT);
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
    if height > top {
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

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn halls_godot_transformed_landing_and_lip_do_not_block_step() {
        // World-space faces captured from live WmoCollision33746373/Group23.
        let landing = [
            Vec3::new(29.497047, -32.414986, 155.17384),
            Vec3::new(29.497051, -32.414986, 154.43205),
            Vec3::new(24.662296, -35.560482, 154.43205),
        ];
        let lip = [
            Vec3::new(25.961683, -35.760395, 154.11337),
            Vec3::new(24.662296, -35.560482, 154.43205),
            Vec3::new(29.497051, -32.414986, 154.43205),
        ];
        let triangles = [landing, lip];
        let landing_feet = Vec3::new(27.20584, -33.905655, 154.50899);
        let raised = Vec3::new(27.40882, -33.836037, 154.36305);
        let destination = Vec3::new(27.20584, -33.836037, 154.50899);
        assert!(!step_wall_blocked(
            &triangles,
            landing_feet,
            landing_feet,
            landing_feet.y,
            0.7
        ));
        assert!(!step_wall_blocked(
            &triangles,
            raised,
            destination,
            -32.399925,
            0.7
        ));
    }

    #[test]
    fn raised_wall_above_tolerance_still_blocks() {
        let wall = [[
            Vec3::new(0.5, 0.002, -1.0),
            Vec3::new(0.5, 0.002, 1.0),
            Vec3::new(0.5, 1.0, 0.0),
        ]];
        assert!(step_wall_blocked(&wall, Vec3::ZERO, Vec3::X, 1.6, 0.7));
    }

    #[test]
    fn reports_the_overhead_face_and_its_height() {
        let floor = [
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let ceiling = floor.map(|point| point + Vec3::Y * 0.4);
        let triangles = [floor, ceiling];
        let feet = Vec3::ZERO;
        assert!(step_wall_blocked(&triangles, feet, feet, feet.y, 0.7));
        assert_eq!(
            step_wall_blocking_triangles(&triangles, feet, feet, feet.y, 0.7),
            vec![(1, Some(0.4))]
        );
    }
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
