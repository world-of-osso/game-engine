//! The pick's box rejection never changes which triangle a ray hits.
use super::{Ray, SurfaceData, ray_triangle, surface_hit};
use godot::builtin::{Basis, Transform3D, Vector3};

/// A 10x10 quad grid in the XY plane at z = 0, 1 yd per quad: the left half on bone 0,
/// the right half on bone 1, the middle column blended half and half.
fn grid(weight_scale: f32) -> SurfaceData {
    let mut vertices = Vec::new();
    let (mut bones, mut weights) = (Vec::new(), Vec::new());
    for y in 0..=10 {
        for x in 0..=10 {
            vertices.push(Vector3::new(x as f32, y as f32, 0.0));
            let right = match x {
                0..5 => 0.0,
                5 => 0.5,
                _ => 1.0,
            };
            bones.extend([0, 1]);
            weights.extend([(1.0 - right) * weight_scale, right * weight_scale]);
        }
    }
    let mut indices = Vec::new();
    for y in 0..10 {
        for x in 0..10 {
            let corner = y * 11 + x;
            indices.extend([
                corner,
                corner + 1,
                corner + 11,
                corner + 1,
                corner + 12,
                corner + 11,
            ]);
        }
    }
    SurfaceData::new(vertices, bones, weights, indices)
}

/// Skin every vertex and test every triangle: the pick without rejection.
fn reference_hit(
    data: &SurfaceData,
    to_world: Transform3D,
    palette: &[Transform3D],
    ray: &Ray,
) -> Option<f32> {
    let world: Vec<Vector3> = data
        .vertices
        .iter()
        .enumerate()
        .map(|(index, &vertex)| {
            let skinned = (0..2).fold(Vector3::ZERO, |sum, slot| {
                let bone = palette[data.bones[index * 2 + slot] as usize];
                sum + (bone * vertex) * data.weights[index * 2 + slot]
            });
            to_world * skinned
        })
        .collect();
    data.indices
        .chunks_exact(3)
        .filter_map(|t| {
            ray_triangle(
                ray,
                world[t[0] as usize],
                world[t[1] as usize],
                world[t[2] as usize],
            )
        })
        .min_by(f32::total_cmp)
}

/// Bone 1 swings 40 degrees about Y and lifts 2 yd; the model stands at (100, 0, 50).
fn pose() -> (Transform3D, [Transform3D; 2]) {
    let swing = Transform3D::new(
        Basis::from_axis_angle(Vector3::UP, 40f32.to_radians()),
        Vector3::new(0.0, 2.0, 0.0),
    );
    let to_world = Transform3D::new(Basis::IDENTITY, Vector3::new(100.0, 0.0, 50.0));
    (to_world, [Transform3D::IDENTITY, swing])
}

/// Rays from 20 yd in front of the model toward a 0.25 yd grid of targets that
/// covers the posed surface and the space around it.
fn rays() -> impl Iterator<Item = Ray> {
    (-20..=60).flat_map(|i| {
        (-20..=60).map(move |j| {
            let target = Vector3::new(100.0 + i as f32 * 0.25, j as f32 * 0.25, 50.0);
            let origin = Vector3::new(103.0, 4.0, 70.0);
            Ray {
                origin,
                direction: (target - origin).normalized(),
            }
        })
    })
}

fn assert_same_hits(data: &SurfaceData) {
    let (to_world, palette) = pose();
    let (mut hits, mut misses) = (0, 0);
    for ray in rays() {
        let expected = reference_hit(data, to_world, &palette, &ray);
        assert_eq!(surface_hit(data, to_world, Some(&palette), &ray), expected);
        if expected.is_some() {
            hits += 1
        } else {
            misses += 1
        }
    }
    assert!(hits > 500 && misses > 500, "{hits} hits, {misses} misses");
}

#[test]
fn skinned_pick_matches_full_skinning() {
    assert_same_hits(&grid(1.0));
}

/// Weights summing to 0.6 pull vertices toward the skeleton origin, outside their
/// bones' rest boxes; the pick must still find them.
#[test]
fn unnormalized_weights_pick_matches_full_skinning() {
    assert_same_hits(&grid(0.6));
}

#[test]
fn unskinned_pick_hits_the_placed_surface_only() {
    let data = grid(1.0);
    let (to_world, _) = pose();
    let at = |x: f32, y: f32| {
        let origin = Vector3::new(100.0 + x, y, 60.0);
        surface_hit(
            &data,
            to_world,
            None,
            &Ray {
                origin,
                direction: Vector3::new(0.0, 0.0, -1.0),
            },
        )
    };
    assert_eq!(at(3.3, 7.6), Some(10.0));
    assert_eq!(at(10.5, 7.6), None);
    assert_eq!(at(3.3, -0.5), None);
}
