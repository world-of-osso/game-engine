//! Retail scenery distance for placed doodads: build 12340 `CMapObj` classifies a
//! placement by the largest axis of its transformed M2 render box and draws it only
//! within that class's far radius of the box center. Constants and rules as
//! reproduced by solarityclient `crates/runtime/src/application/m2_spatial.rs`
//! (`SceneryDistance::new`/`opacity`), with `environmentDetail` at its default 1.
//! The retail fade band before the far radius is not reproduced: a doodad is drawn
//! opaque up to the radius where retail opacity reaches zero.
//!
//! A drawn doodad also samples its animation at the shared [`AnimationLod`] rate
//! from its box center distance and whether its box is in the view frustum; an
//! undrawn one does not animate (solarityclient `terrain_frame/m2.rs` culls static
//! placements from their bounds before advancing their playback).

use glam::{Affine3A, Vec3};

use crate::{animation::lod::AnimationLod, wmo::portals::HalfSpace};

/// Inclusive upper size limits of classes 0..=3; anything larger is class 4.
const SIZE_LIMITS: [f32; 4] = [1.0, 4.0, 15.0, 100.0];
/// Far radius per class, in yards.
const FAR_RADII: [f32; 5] = [30.0, 100.0, 200.0, 750.0, 1250.0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SceneryDistance {
    center: Vec3,
    far: f32,
    world_min: Vec3,
    world_max: Vec3,
}

impl SceneryDistance {
    /// `min`/`max`: the M2 header render box in model space (already in engine axes).
    pub fn new(min: Vec3, max: Vec3, world_from_model: Affine3A) -> Self {
        let corners = (0..8).map(|index| {
            let pick = |bit: usize, axis: usize| {
                if index & bit == 0 {
                    min[axis]
                } else {
                    max[axis]
                }
            };
            world_from_model.transform_point3(Vec3::new(pick(1, 0), pick(2, 1), pick(4, 2)))
        });
        let (world_min, world_max) = corners.fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(low, high), corner| (low.min(corner), high.max(corner)),
        );
        let size = (world_max - world_min).max_element();
        let class = SIZE_LIMITS
            .iter()
            .position(|limit| size <= *limit)
            .unwrap_or(FAR_RADII.len() - 1);
        Self {
            center: world_from_model.transform_point3((min + max) * 0.5),
            far: FAR_RADII[class],
            world_min,
            world_max,
        }
    }

    pub fn visible_from(&self, camera: Vec3) -> bool {
        self.center.distance_squared(camera) <= self.far * self.far
    }

    /// Animation rate of the placement seen from `camera` through `frustum` (inside
    /// half spaces): frozen when not drawn or when its world box is out of view.
    pub fn animation_lod(&self, camera: Vec3, frustum: &[HalfSpace]) -> AnimationLod {
        if !self.visible_from(camera) {
            return AnimationLod::Frozen;
        }
        AnimationLod::new(self.center.distance(camera), self.box_in_frustum(frustum))
    }

    /// Whether any part of the world box can be inside every half space: the corner
    /// farthest along each plane normal must not be behind it.
    fn box_in_frustum(&self, frustum: &[HalfSpace]) -> bool {
        frustum.iter().all(|plane| {
            let corner = Vec3::select(
                plane.normal.cmpge(Vec3::ZERO),
                self.world_max,
                self.world_min,
            );
            plane.normal.dot(corner) + plane.d >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::Quat;

    use super::*;

    fn cube(size: f32) -> (Vec3, Vec3) {
        (Vec3::splat(-size / 2.0), Vec3::splat(size / 2.0))
    }

    fn far_radius(size: f32, world_from_model: Affine3A) -> f32 {
        let (min, max) = cube(size);
        let scenery = SceneryDistance::new(min, max, world_from_model);
        let origin = world_from_model.transform_point3(Vec3::ZERO);
        let along = |distance: f32| origin + Vec3::X * distance;
        assert!(scenery.visible_from(along(scenery.far)));
        assert!(!scenery.visible_from(along(scenery.far + 0.01)));
        scenery.far
    }

    #[test]
    fn size_classes_use_inclusive_limits_and_retail_radii() {
        let placed = Affine3A::from_translation(Vec3::new(100.0, 5.0, -40.0));
        assert_eq!(far_radius(0.5, placed), 30.0);
        assert_eq!(far_radius(1.0, placed), 30.0);
        assert_eq!(far_radius(1.01, placed), 100.0);
        assert_eq!(far_radius(4.0, placed), 100.0);
        assert_eq!(far_radius(15.0, placed), 200.0);
        assert_eq!(far_radius(100.0, placed), 750.0);
        assert_eq!(far_radius(100.5, placed), 1250.0);
    }

    #[test]
    fn placement_scale_and_rotation_change_the_class() {
        assert_eq!(
            far_radius(3.0, Affine3A::from_scale(Vec3::splat(2.0))),
            200.0
        );
        // A 0.9 yd cube turned 45 degrees spans 1.27 yd on X and Z.
        let turned = Affine3A::from_quat(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4));
        assert_eq!(far_radius(0.9, turned), 100.0);
    }

    /// Looking down -Z from the origin with a 90 degree horizontal field of view.
    fn view() -> Vec<HalfSpace> {
        [
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, -1.0),
            Vec3::new(0.0, -1.0, -1.0),
            Vec3::NEG_Z,
        ]
        .into_iter()
        .map(|normal| HalfSpace {
            normal: normal.normalize(),
            d: 0.0,
        })
        .collect()
    }

    /// A 10 yd tree (class 2, drawn to 200 yd) whose box center is at `center`.
    fn tree_at(center: Vec3) -> SceneryDistance {
        let (min, max) = cube(10.0);
        SceneryDistance::new(min, max, Affine3A::from_translation(center))
    }

    #[test]
    fn drawn_doodads_animate_at_the_shared_lod_rate() {
        use crate::animation::lod::AnimationLod::{Frozen, Full, Half};
        let lod = |center: Vec3| tree_at(center).animation_lod(Vec3::ZERO, &view());
        assert_eq!(lod(Vec3::new(0.0, 0.0, -30.0)), Full);
        assert_eq!(lod(Vec3::new(0.0, 0.0, -30.5)), Half);
        assert_eq!(lod(Vec3::new(0.0, 0.0, -60.0)), Half);
        assert_eq!(lod(Vec3::new(0.0, 0.0, -60.5)), Frozen);
        // Behind the camera, and beside it outside the field of view: off screen.
        assert_eq!(lod(Vec3::new(0.0, 0.0, 10.0)), Frozen);
        assert_eq!(lod(Vec3::new(25.0, 0.0, -10.0)), Frozen);
        // The box center is outside the view but its near half reaches into it.
        assert_eq!(lod(Vec3::new(14.0, 0.0, -10.0)), Full);
        // Beyond its 200 yd scenery radius it is not drawn, so it does not animate.
        let small = {
            let (min, max) = cube(0.5);
            SceneryDistance::new(min, max, Affine3A::from_translation(Vec3::NEG_Z * 31.0))
        };
        assert!(!small.visible_from(Vec3::ZERO));
        assert_eq!(small.animation_lod(Vec3::ZERO, &view()), Frozen);
    }

    #[test]
    fn distance_is_measured_from_the_transformed_box_center() {
        // A tree whose box rises 10 yd above its origin: center 5 yd up, class 2.
        let scenery = SceneryDistance::new(
            Vec3::new(-2.0, 0.0, -2.0),
            Vec3::new(2.0, 10.0, 2.0),
            Affine3A::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        );
        assert!(scenery.visible_from(Vec3::new(0.0, 205.0, 0.0)));
        assert!(!scenery.visible_from(Vec3::new(0.0, -195.5, 0.0)));
    }
}
