//! Read-only live movement collision diagnostics.
use glam::Vec3;
use godot::{
    classes::{Node, PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D},
    prelude::*,
};
use serde_json::{Value, json};

impl crate::GameClient {
    pub(super) fn probe_ground(
        &self,
        feet: [f32; 3],
        direction: [f32; 2],
    ) -> Result<String, String> {
        let feet = Vec3::from_array(feet);
        let direction = Vec3::new(direction[0], 0.0, direction[1]);
        if !feet.is_finite() || !direction.is_finite() || direction.length_squared() == 0.0 {
            return Err("probe-ground requires finite coordinates and nonzero direction".into());
        }
        let direction = direction.normalize();
        let player = self
            .world
            .local_player_node()
            .ok_or("probe-ground needs a local player")?;
        let space = player
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
            .ok_or("probe-ground needs physics space")?;
        let walls = |origin, direction, length| {
            crate::wmo::collision::wall_hit(&space, origin, direction, length)
        };
        let ground = crate::ground::TerrainGround {
            terrain: &self.terrain,
            walls: &walls,
            step_wall_blocked: &|from, to, top| {
                crate::wmo::step_clearance::step_wall_blocked(&space, from, to, top)
            },
            game_object_floor: &|point| crate::game_objects::floor_hit(&space, point),
        };
        let mut rays = Vec::new();
        for layer in [2_u32, 8, 32] {
            for distance in [0.0_f32, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0] {
                let column = feet + direction * distance;
                rays.push(physics_ray(
                    &space,
                    layer,
                    "column",
                    column + Vec3::Y * 1.6,
                    column - Vec3::Y * 2.0,
                )?);
            }
            for height in [0.1_f32, 0.6, 1.0, 1.6, 2.2] {
                let origin = feet + Vec3::Y * height;
                rays.push(physics_ray(
                    &space,
                    layer,
                    "wall",
                    origin,
                    origin + direction * 2.0,
                )?);
            }
        }
        serde_json::to_string(
            &json!({"admission": ground.probe_admission(feet, direction), "rays": rays}),
        )
        .map_err(|error| format!("probe-ground JSON: {error}"))
    }
}

fn physics_ray(
    space: &Gd<PhysicsDirectSpaceState3D>,
    layer: u32,
    kind: &str,
    from: Vec3,
    to: Vec3,
) -> Result<Value, String> {
    let mut query = PhysicsRayQueryParameters3D::create(
        Vector3::from_array(from.to_array()),
        Vector3::from_array(to.to_array()),
    )
    .ok_or("probe-ground ray allocation failed")?;
    query.set_collision_mask(layer);
    query.set_hit_back_faces(kind == "column");
    let hit = space.clone().intersect_ray(&query);
    if hit.is_empty() {
        return Ok(
            json!({"layer": layer, "kind": kind, "from": from.to_array(), "to": to.to_array(), "hit": null}),
        );
    }
    let node = hit
        .get("collider")
        .ok_or("probe-ground hit lacks collider")?
        .try_to::<Gd<Node>>()
        .map_err(|e| format!("probe-ground collider: {e}"))?;
    let point = hit
        .get("position")
        .ok_or("probe-ground hit lacks position")?
        .to::<Vector3>();
    let normal = hit
        .get("normal")
        .ok_or("probe-ground hit lacks normal")?
        .to::<Vector3>();
    Ok(
        json!({"layer":layer,"kind":kind,"from":from.to_array(),"to":to.to_array(),"hit":{"collider":node.get_path().to_string(),"position":[point.x,point.y,point.z],"normal":[normal.x,normal.y,normal.z],"distance": (Vec3::new(point.x, point.y, point.z) - from).length()}}),
    )
}
