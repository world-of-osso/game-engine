//! Swept airborne contact: physics broadphase, annotated capsule narrowphase, tangential response.

use std::collections::{HashMap, HashSet};

use game_engine_core::elastic_tree::deflect_motion;
use glam::Vec3;
use godot::{
    classes::{BoxShape3D, PhysicsDirectSpaceState3D, PhysicsShapeQueryParameters3D, StaticBody3D},
    prelude::*,
};

use super::elastic_tree::{TREE_LAYER, TreeHit, WowElasticTree};

/// One broad collision envelope for the first mounted-flight prototype.
pub(crate) const MOUNT_CONTACT_RADIUS: f32 = 1.25;
pub(crate) const MOUNT_CONTACT_HEIGHT: f32 = 1.0;
const CONTACT_MARGIN: f32 = 0.01;
const MAX_CONTACTS: usize = 4;

struct PlacedHit {
    tree: Gd<WowElasticTree>,
    hit: TreeHit,
}

/// A continuous sphere sweep, not an endpoint overlap. Elastic branches may yield;
/// trunks remove only inward movement, preserving the player's ability to steer past.
pub(crate) fn move_airborne(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    radius: f32,
    delta: f32,
) -> Vec3 {
    if !delta.is_finite()
        || delta <= 0.0
        || !radius.is_finite()
        || radius <= 0.0
        || !from.is_finite()
        || !to.is_finite()
    {
        godot_error!("Invalid airborne tree sweep: {from} -> {to}, radius {radius}");
        return from;
    }
    let mut position = from;
    let mut remaining = to - from;
    let mut contacted: HashMap<InstanceId, Vec<usize>> = HashMap::new();
    for _ in 0..MAX_CONTACTS {
        let length = remaining.length();
        if length <= f32::EPSILON {
            return position;
        }
        let end = position + remaining;
        let Some(mut placed) = first_hit(space, position, end, radius, &contacted) else {
            return end;
        };
        let fraction = (placed.hit.contact.fraction - CONTACT_MARGIN / length).max(0.0);
        position += remaining * fraction;
        let incoming = remaining;
        remaining *= 1.0 - fraction;
        let resistance = placed
            .tree
            .bind_mut()
            .apply_hit(&placed.hit, incoming / delta);
        remaining = deflect_motion(remaining, placed.hit.contact.normal, resistance);
        if let Some(index) = placed.hit.branch {
            contacted
                .entry(placed.tree.instance_id())
                .or_default()
                .push(index);
        }
    }
    // Several rigid surfaces can enclose the mover. Do not advance through unchecked geometry.
    position
}

fn first_hit(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    radius: f32,
    contacted: &HashMap<InstanceId, Vec<usize>>,
) -> Option<PlacedHit> {
    nearby_trees(space, from, to, radius)
        .into_iter()
        .filter_map(|tree| {
            let excluded = contacted
                .get(&tree.instance_id())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let hit = tree.bind().sweep(from, to, radius, excluded)?;
            Some(PlacedHit { tree, hit })
        })
        .min_by(|a, b| a.hit.contact.fraction.total_cmp(&b.hit.contact.fraction))
}

/// Godot's spatial broadphase restricts work to bodies overlapping the swept sphere's box.
fn nearby_trees(
    space: &Gd<PhysicsDirectSpaceState3D>,
    from: Vec3,
    to: Vec3,
    radius: f32,
) -> Vec<Gd<WowElasticTree>> {
    let min = from.min(to) - Vec3::splat(radius);
    let max = from.max(to) + Vec3::splat(radius);
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(Vector3::from_array((max - min).to_array()));
    let mut query = PhysicsShapeQueryParameters3D::new_gd();
    query.set_shape(&shape);
    query.set_transform(
        Transform3D::IDENTITY.translated(Vector3::from_array(((min + max) * 0.5).to_array())),
    );
    query.set_collision_mask(TREE_LAYER);
    let hits = space.clone().intersect_shape(&query);
    let mut seen = HashSet::new();
    hits.iter_shared()
        .filter_map(|hit| {
            let body = hit.get("collider")?.try_to::<Gd<StaticBody3D>>().ok()?;
            let tree = body.get_parent()?.try_cast::<WowElasticTree>().ok()?;
            seen.insert(tree.instance_id()).then_some(tree)
        })
        .collect()
}
