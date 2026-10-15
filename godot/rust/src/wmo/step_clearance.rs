//! Broad-phase nearby live collision meshes, then exact zero-radius body-plane sweep.
use game_engine_core::step_clearance::BODY_HEIGHT;
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
    let triangles = query_body_triangles(space, from, to);
    game_engine_core::step_clearance::step_body_blocked(
        &triangles,
        from,
        to,
        support_top,
        shared::ground::min_walkable_normal_y(),
    )
}

fn query_body_triangles(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
) -> Vec<[Vec3; 3]> {
    let mut query = body_neighbour_query(from, to);
    let mut excluded = Array::<Rid>::new();
    let mut triangles = Vec::new();
    loop {
        let hits = space.clone().intersect_shape(&query);
        if hits.is_empty() {
            return triangles;
        }
        for hit in hits.iter_shared() {
            let body = hit
                .get("collider")
                .expect("Step query collider")
                .to::<Gd<StaticBody3D>>();
            append_body_triangles(&body, &mut triangles);
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

fn append_body_triangles(body: &Gd<StaticBody3D>, triangles: &mut Vec<[Vec3; 3]>) {
    for child in body.get_children().iter_shared() {
        if let Ok(collision) = child.try_cast::<CollisionShape3D>()
            && !collision.is_disabled()
        {
            append_shape_triangles(&collision, triangles);
        }
    }
}

fn append_shape_triangles(collision: &Gd<CollisionShape3D>, triangles: &mut Vec<[Vec3; 3]>) {
    let shape = collision
        .get_shape()
        .expect("Live collision shape")
        .try_cast::<ConcavePolygonShape3D>()
        .expect("WMO/GO authored triangle collision");
    let transform = collision.get_global_transform();
    let faces = shape.get_faces();
    triangles.extend(faces.as_slice().chunks_exact(3).map(|face| {
        [face[0], face[1], face[2]].map(|point| {
            let world = transform * point;
            Vec3::new(world.x, world.y, world.z)
        })
    }));
}
