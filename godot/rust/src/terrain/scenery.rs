//! Retail scenery distance for placed doodads: build 12340 `CMapObj` classifies a
//! placement by the largest axis of its transformed M2 render box, and fades it out
//! linearly over its class's fade band before the far radius of the box center.
//! Constants and rules as reproduced by solarityclient
//! `crates/runtime/src/application/m2_spatial.rs` (`SceneryDistance::new`/`opacity`).
//! Retail scales the far radius of classes 1..=3 by `environmentDetail`; this client
//! has no such setting, so the stock default 1 applies and the radii are unscaled.
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
/// Width per class of the band before the far radius over which opacity falls to 0.
const FADE_BANDS: [f32; 5] = [5.0, 10.0, 15.0, 20.0, 50.0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SceneryDistance {
    center: Vec3,
    class: usize,
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
            class,
            world_min,
            world_max,
        }
    }

    /// Retail placement opacity from `camera`: 1 before the fade band, 0 beyond the
    /// far radius, linear between, snapped to 1 above 0.99 and to 0 at or below 0.01.
    pub fn opacity(&self, camera: Vec3) -> f32 {
        let distance_squared = self.center.distance_squared(camera);
        let far = FAR_RADII[self.class];
        if distance_squared > far * far {
            return 0.0;
        }
        let fade = FADE_BANDS[self.class];
        let start = far - fade;
        if distance_squared <= start * start {
            return 1.0;
        }
        let opacity = 1.0 - (distance_squared.sqrt() - start) / fade;
        if opacity > 0.99 {
            1.0
        } else if opacity <= 0.01 {
            0.0
        } else {
            opacity
        }
    }

    pub fn visible_from(&self, camera: Vec3) -> bool {
        self.opacity(camera) > 0.0
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

    /// The far radius of a cube, checked against its fade band: opaque at the band
    /// start, half faded in its middle, undrawn at the far radius.
    fn far_radius(size: f32, world_from_model: Affine3A) -> f32 {
        let (min, max) = cube(size);
        let scenery = SceneryDistance::new(min, max, world_from_model);
        let origin = world_from_model.transform_point3(Vec3::ZERO);
        let along = |distance: f32| origin + Vec3::X * distance;
        let (far, fade) = (FAR_RADII[scenery.class], FADE_BANDS[scenery.class]);
        assert_eq!(scenery.opacity(along(far - fade)), 1.0);
        assert!((scenery.opacity(along(far - fade / 2.0)) - 0.5).abs() < 1e-4);
        assert!(scenery.visible_from(along(far - fade * 0.02)));
        assert!(!scenery.visible_from(along(far)));
        far
    }

    #[test]
    fn opacity_fades_over_each_class_band_and_snaps_at_its_ends() {
        let placed = Affine3A::from_translation(Vec3::new(100.0, 5.0, -40.0));
        for (size, far, fade) in [
            (0.5, 30.0, 5.0),
            (2.0, 100.0, 10.0),
            (10.0, 200.0, 15.0),
            (50.0, 750.0, 20.0),
            (200.0, 1250.0, 50.0),
        ] {
            let (min, max) = cube(size);
            let scenery = SceneryDistance::new(min, max, placed);
            let at = |distance: f32| {
                scenery.opacity(placed.transform_point3(Vec3::new(0.0, 0.0, distance)))
            };
            let start = far - fade;
            assert_eq!(at(0.0), 1.0, "{size}");
            assert_eq!(at(start), 1.0, "{size}: band start");
            // 0.991 snaps up to opaque; 0.98 does not.
            assert_eq!(at(start + fade * 0.009), 1.0, "{size}");
            assert!((at(start + fade * 0.02) - 0.98).abs() < 1e-3, "{size}");
            assert!((at(start + fade * 0.75) - 0.25).abs() < 1e-3, "{size}");
            // 0.02 stays; 0.005 snaps to undrawn, as does the far radius itself.
            assert!((at(far - fade * 0.02) - 0.02).abs() < 1e-3, "{size}");
            assert_eq!(at(far - fade * 0.005), 0.0, "{size}");
            assert_eq!(at(far), 0.0, "{size}");
            assert_eq!(at(far + 1.0), 0.0, "{size}");
        }
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
        assert_eq!(scenery.opacity(Vec3::new(0.0, 190.0, 0.0)), 1.0);
        assert!(scenery.visible_from(Vec3::new(0.0, 204.5, 0.0)));
        assert!(!scenery.visible_from(Vec3::new(0.0, -195.0, 0.0)));
    }
}
