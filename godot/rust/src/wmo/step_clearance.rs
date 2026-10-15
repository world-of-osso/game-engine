//! Broad-phase nearby live collision meshes, then exact zero-radius body-plane sweep.
use game_engine_core::step_clearance::{BODY_HEIGHT, blocks_step_sweep};
use glam::Vec3;
use godot::{
    classes::{
        BoxShape3D, CollisionShape3D, ConcavePolygonShape3D, PhysicsDirectSpaceState3D,
        PhysicsShapeQueryParameters3D, StaticBody3D,
    },
    prelude::*,
};

pub(crate) fn step_body_blocked(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    support_top: f32,
) -> bool {
    let mut query = body_neighbour_query(from, to);
    let mut excluded = Array::<Rid>::new();
    loop {
        let hits = space.clone().intersect_shape(&query);
        if hits.is_empty() {
            return false;
        }
        for hit in hits.iter_shared() {
            let body = hit
                .get("collider")
                .expect("Step query collider")
                .to::<Gd<StaticBody3D>>();
            if body_blocks_step(&body, from, to, support_top) {
                return true;
            }
            // Inspect every shape on this body before excluding it. Repeating the
            // query consumes all bodies even when Godot fills its result batch.
            excluded.push(body.get_rid());
        }
        query.set_exclude(&excluded);
    }
}

fn body_neighbour_query(from: Vec3, to: Vec3) -> Gd<PhysicsShapeQueryParameters3D> {
    let mut shape = BoxShape3D::new_gd();
    // Deliberately over-broad XZ bounds, not a player radius or collision tolerance.
    // Only exact triangles entering the swept vertical plane decide blocking.
    let size = (to - from).abs() + Vec3::splat(BODY_HEIGHT);
    shape.set_size(Vector3::from_array(size.to_array()));
    let center = from.lerp(to, 0.5) + Vec3::Y * (BODY_HEIGHT / 2.0);
    let mut query = PhysicsShapeQueryParameters3D::new_gd();
    query.set_shape(&shape);
    query.set_transform(Transform3D::new(
        Basis::IDENTITY,
        Vector3::from_array(center.to_array()),
    ));
    query.set_collision_mask(super::collision::WMO_LAYER | crate::game_objects::GAME_OBJECT_LAYER);
    query
}

fn body_blocks_step(body: &Gd<StaticBody3D>, from: Vec3, to: Vec3, support_top: f32) -> bool {
    body.get_children().iter_shared().any(|child| {
        let Ok(collision) = child.try_cast::<CollisionShape3D>() else {
            return false;
        };
        if collision.is_disabled() {
            return false;
        }
        let shape = collision
            .get_shape()
            .expect("Live collision shape")
            .try_cast::<ConcavePolygonShape3D>()
            .expect("WMO/GO authored triangle collision");
        let transform = collision.get_global_transform();
        let faces = shape.get_faces();
        faces.as_slice().chunks_exact(3).any(|face| {
            let triangle = [face[0], face[1], face[2]].map(|point| {
                let world = transform * point;
                Vec3::new(world.x, world.y, world.z)
            });
            blocks_step_sweep(
                triangle,
                from,
                to,
                support_top,
                shared::ground::min_walkable_normal_y(),
            )
        })
    })
}
