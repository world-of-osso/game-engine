//! Bounded game-object floor rays answer exactly as unbounded ones: a fixture compares
//! `FloorRays::floor` with `floor_hit` over a grid around the bodies a test places.
use super::{FloorRays, floor_hit};
use godot::{
    classes::{Node3D, StaticBody3D},
    prelude::*,
};
use std::collections::HashMap;

#[derive(GodotClass)]
#[class(init, base = Node3D)]
pub(crate) struct FloorRayParity {
    base: Base<Node3D>,
}

#[godot_api]
impl FloorRayParity {
    /// Casts both rays at every grid point of `min..=max` (`step` yards apart) over this
    /// node's `StaticBody3D` children; returns `points casts hits mismatches` or an error.
    #[func]
    fn compare_floor_rays(&mut self, min: Vector3, max: Vector3, step: f32) -> GString {
        let Some(space) = self
            .base()
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
        else {
            return GString::from("no physics space");
        };
        let bodies: Vec<Gd<StaticBody3D>> = self
            .base()
            .get_children()
            .iter_shared()
            .filter_map(|child| child.try_cast::<StaticBody3D>().ok())
            .collect();
        let mut shapes = HashMap::new();
        let mut rays = FloorRays::new(space.clone(), bodies, &mut shapes);
        let (mut points, mut hits, mut mismatches) = (0, 0, Vec::new());
        let axis = |low: f32, high: f32| {
            let count = ((high - low) / step).round() as i32;
            (0..=count).map(move |index| low + index as f32 * step)
        };
        for x in axis(min.x, max.x) {
            for y in axis(min.y, max.y) {
                for z in axis(min.z, max.z) {
                    let feet = glam::Vec3::new(x, y, z);
                    let bounded = rays.floor(feet);
                    let unbounded = floor_hit(&space, feet);
                    points += 1;
                    hits += usize::from(unbounded.is_some());
                    if bounded.map(f32::to_bits) != unbounded.map(f32::to_bits) {
                        mismatches.push(format!("{feet}: {bounded:?} != {unbounded:?}"));
                    }
                }
            }
        }
        let casts = rays.casts;
        let report = format!(
            "points={points} casts={casts} hits={hits} mismatches={} {}",
            mismatches.len(),
            mismatches
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        );
        GString::from(report.as_str())
    }
}
