//! Live authored meshes intersecting the interval below the admitted low wall ray.
use game_engine_core::player_physics_data::WALL_RAY_HEIGHT;
use glam::Vec3;
use godot::{
    classes::{
        BoxShape3D, CollisionShape3D, ConcavePolygonShape3D, PhysicsDirectSpaceState3D,
        PhysicsShapeQueryParameters3D, StaticBody3D,
    },
    prelude::*,
};

pub(crate) fn step_wall_blocked(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    support_top: f32,
) -> bool {
    let triangles = query_wall_triangles(space, from, to);
    game_engine_core::step_clearance::step_wall_blocked(
        &triangles,
        from,
        to,
        support_top,
        shared::ground::min_walkable_normal_y(),
    )
}

fn query_wall_triangles(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
) -> Vec<[Vec3; 3]> {
    let mut triangles = Vec::new();
    visit_nearby_triangles(space, from, to, |_, _, _, face| triangles.push(face));
    triangles
}

pub(crate) fn diagnose_step_wall(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    support_top: f32,
) -> serde_json::Value {
    let mut triangles = Vec::new();
    let mut identities = Vec::new();
    visit_nearby_triangles(space, from, to, |body, shape, index, face| {
        triangles.push(face);
        identities.push((
            body.get_path().to_string(),
            shape.get_path().to_string(),
            index,
        ));
    });
    let blockers = game_engine_core::step_clearance::step_wall_blocking_triangles(
        &triangles,
        from,
        to,
        support_top,
        shared::ground::min_walkable_normal_y(),
    );
    let hits: Vec<_> = blockers.into_iter().map(|(index, height)| {
        let (body, shape, face_index) = &identities[index];
        serde_json::json!({"collider":body,"shape":shape,"face_index":face_index,"triangle":triangles[index].map(|point| point.to_array()),"height_at_landing":height})
    }).collect();
    serde_json::json!({"from":from.to_array(),"landing_feet":to.to_array(),"support_top":support_top,"blocking_triangles":hits})
}

fn visit_nearby_triangles(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    mut visit: impl FnMut(&Gd<StaticBody3D>, &Gd<CollisionShape3D>, usize, [Vec3; 3]),
) {
    let mut query = wall_neighbour_query(from, to);
    let mut excluded = Array::<Rid>::new();
    loop {
        let hits = space.clone().intersect_shape(&query);
        if hits.is_empty() {
            return;
        }
        for hit in hits.iter_shared() {
            let body = hit
                .get("collider")
                .expect("Step query collider")
                .to::<Gd<StaticBody3D>>();
            visit_body_triangles(&body, &mut visit);
            // Inspect every shape on this body before excluding it. Repeating the
            // query consumes all bodies even when Godot fills its result batch.
            excluded.push(body.get_rid());
        }
        query.set_exclude(&excluded);
    }
}

fn wall_neighbour_query(from: Vec3, to: Vec3) -> Gd<PhysicsShapeQueryParameters3D> {
    let mut shape = BoxShape3D::new_gd();
    // Deliberately over-broad XZ bounds, not a player radius or collision tolerance.
    // Only exact triangles entering the swept vertical plane decide blocking.
    let size = (to - from).abs() + Vec3::splat(WALL_RAY_HEIGHT);
    shape.set_size(Vector3::from_array(size.to_array()));
    let center = from.lerp(to, 0.5) + Vec3::Y * (WALL_RAY_HEIGHT / 2.0);
    let mut query = PhysicsShapeQueryParameters3D::new_gd();
    query.set_shape(&shape);
    query.set_transform(Transform3D::new(
        Basis::IDENTITY,
        Vector3::from_array(center.to_array()),
    ));
    query.set_collision_mask(super::collision::WMO_LAYER | crate::game_objects::GAME_OBJECT_LAYER);
    query
}

fn visit_body_triangles(
    body: &Gd<StaticBody3D>,
    visit: &mut impl FnMut(&Gd<StaticBody3D>, &Gd<CollisionShape3D>, usize, [Vec3; 3]),
) {
    for child in body.get_children().iter_shared() {
        if let Ok(collision) = child.try_cast::<CollisionShape3D>()
            && !collision.is_disabled()
        {
            visit_shape_triangles(body, &collision, visit);
        }
    }
}

fn visit_shape_triangles(
    body: &Gd<StaticBody3D>,
    collision: &Gd<CollisionShape3D>,
    visit: &mut impl FnMut(&Gd<StaticBody3D>, &Gd<CollisionShape3D>, usize, [Vec3; 3]),
) {
    let shape = collision
        .get_shape()
        .expect("Live collision shape")
        .try_cast::<ConcavePolygonShape3D>()
        .expect("WMO/GO authored triangle collision");
    let transform = collision.get_global_transform();
    let faces = shape.get_faces();
    for (index, face) in faces.as_slice().chunks_exact(3).enumerate() {
        let triangle = [face[0], face[1], face[2]].map(|point| {
            let world = transform * point;
            Vec3::new(world.x, world.y, world.z)
        });
        visit(body, collision, index, triangle);
    }
}
