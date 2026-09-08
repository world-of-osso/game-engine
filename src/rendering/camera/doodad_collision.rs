use std::sync::Arc;

use bevy::{
    math::{
        Affine3A,
        bounding::{Aabb3d, RayCast3d},
    },
    picking::mesh_picking::ray_cast::{Backfaces, ray_mesh_intersection},
    prelude::*,
};

use crate::asset::m2::{M2CollisionMesh, wow_to_bevy};

/// Visual bounds used by interaction volumes, never as solid collision geometry.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct DoodadVisualBounds {
    pub world_min: Vec3,
    pub world_max: Vec3,
}

/// Authored collision geometry shared between placements, with a world-space broadphase.
#[derive(Component, Clone, Debug)]
pub struct DoodadCollider {
    pub world_min: Vec3,
    pub world_max: Vec3,
    geometry: Arc<M2CollisionMesh>,
    world_from_model: Affine3A,
}

impl DoodadCollider {
    pub fn new(geometry: Arc<M2CollisionMesh>, transform: &Transform) -> Self {
        let (world_min, world_max) =
            compute_world_aabb(geometry.bounds_min, geometry.bounds_max, transform);
        Self {
            world_min,
            world_max,
            geometry,
            world_from_model: model_to_world(transform),
        }
    }

    /// Return a real double-sided surface hit, not a broadphase entry distance.
    pub fn ray_hit(&self, ray: Ray3d, max_distance: f32) -> Option<f32> {
        RayCast3d::new(ray.origin, ray.direction, max_distance)
            .aabb_intersection_at(&Aabb3d::from_min_max(self.world_min, self.world_max))?;
        ray_mesh_intersection(
            ray,
            &self.world_from_model,
            &self.geometry.vertices,
            None,
            Some(self.geometry.indices.as_slice()),
            None,
            Backfaces::Include,
        )
        .map(|hit| hit.distance)
        .filter(|distance| *distance <= max_distance)
    }
}

fn model_to_world(transform: &Transform) -> Affine3A {
    let convert = |axis: Vec3| Vec3::from(wow_to_bevy(axis.x, axis.y, axis.z));
    let basis = Mat3::from_cols(convert(Vec3::X), convert(Vec3::Y), convert(Vec3::Z));
    transform.compute_affine() * Affine3A::from_mat3(basis)
}

/// Transform model-local WoW bounds without changing placement rotation or scale.
pub fn compute_world_aabb(
    local_min: [f32; 3],
    local_max: [f32; 3],
    transform: &Transform,
) -> (Vec3, Vec3) {
    let lo = Vec3::from(local_min);
    let hi = Vec3::from(local_max);
    let corners = [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ];
    let affine = model_to_world(transform);
    corners
        .into_iter()
        .map(|corner| affine.transform_point3(corner))
        .fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(min, max), corner| (min.min(corner), max.max(corner)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(transform: Transform) -> DoodadCollider {
        DoodadCollider::new(
            Arc::new(M2CollisionMesh {
                bounds_min: [2.0, -2.0, -2.0],
                bounds_max: [2.0, 2.0, 2.0],
                vertices: vec![
                    [2.0, -2.0, -2.0],
                    [2.0, 2.0, -2.0],
                    [2.0, 2.0, 2.0],
                    [2.0, -2.0, 2.0],
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
            }),
            &transform,
        )
    }

    #[test]
    fn authored_doodad_surface_blocks_both_triangle_sides() {
        let collider = wall(Transform::default());
        for ray in [
            Ray3d::new(Vec3::ZERO, Dir3::X),
            Ray3d::new(Vec3::new(4.0, 0.0, 0.0), Dir3::NEG_X),
        ] {
            let distance = collider
                .ray_hit(ray, 4.0)
                .expect("wall surface on either side");
            assert!((distance - 2.0).abs() < 0.0001);
        }
    }

    #[test]
    fn authored_doodad_surface_preserves_rotated_nonuniform_scaled_distance() {
        let transform = Transform::from_translation(Vec3::new(10.0, 5.0, -4.0))
            .with_rotation(Quat::from_rotation_y(0.7))
            .with_scale(Vec3::new(3.0, 2.0, 0.5));
        let collider = wall(transform);
        let ray = Ray3d::new(
            transform.translation,
            Dir3::new(transform.rotation * Vec3::X).unwrap(),
        );
        assert!((collider.ray_hit(ray, 10.0).unwrap() - 6.0).abs() < 0.001);
        assert!(collider.ray_hit(ray, 5.0).is_none());
    }

    #[test]
    fn authored_doodad_broadphase_does_not_turn_empty_space_into_a_wall() {
        let collider = wall(Transform::default());
        assert!(
            collider
                .ray_hit(Ray3d::new(Vec3::new(0.0, 3.0, 0.0), Dir3::X), 4.0)
                .is_none()
        );
    }
}
