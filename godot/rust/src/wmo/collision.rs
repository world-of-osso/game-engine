//! Solid WMO geometry as Godot physics bodies, for camera and player wall rays.
//!
//! WMOs render as meshes without physics; their floors are only the player's ground. The walls
//! come from the same shared collision groups (`shared::ground::WmoGroupCollision`, the
//! MOPY-collidable faces; antiportal and unreachable groups have none), placed with the same
//! transform. They are solid whether or not portal or distance culling draws the group, so they
//! live under their own root, never under a render node.
//!
//! A parsed Stormwind tile holds ~350k collidable WMO triangles; building all of their shapes in
//! one frame stalled it for ~0.5 s. Placements queue as their tile parses and build group by
//! group within a per-frame budget, the group whose bounding box is nearest the player first.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{Affine3A, Vec3};
use godot::{
    classes::{
        CollisionShape3D, ConcavePolygonShape3D, Node3D, PhysicsDirectSpaceState3D,
        PhysicsRayQueryParameters3D, StaticBody3D,
    },
    prelude::*,
};
use shared::ground::{WmoCollision, WmoGroupCollision};

use crate::terrain::streaming::StreamedTerrain;

/// Godot's default layer, which the terrain chunk bodies use.
pub(crate) const TERRAIN_LAYER: u32 = 1;
pub(crate) const WMO_LAYER: u32 = 1 << 1;

/// Main-thread time per frame for building shapes; at least one group builds every frame.
const BUILD_BUDGET: Duration = Duration::from_millis(2);

/// Which placement a body was built for: the map's global WMO or an ADT MODF unique id.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PlacementKey {
    Global,
    Modf(u32),
}

/// A queued placement and, once its first group builds, its body.
struct Placement {
    key: PlacementKey,
    /// The parsed tile that queued it; `None` for the global WMO.
    tile: Option<(u32, u32)>,
    world_from_local: Affine3A,
    local_from_world: Affine3A,
    body: Option<Gd<StaticBody3D>>,
}

/// A group of a placement whose shape is not built yet.
struct PendingGroup {
    placement: usize,
    index: usize,
    group: Arc<WmoGroupCollision>,
}

#[derive(Default)]
pub(crate) struct WmoCollisionBodies {
    root: Option<Gd<Node3D>>,
    tiles: HashSet<(u32, u32)>,
    queued: HashSet<PlacementKey>,
    placements: Vec<Placement>,
    pending: Vec<PendingGroup>,
    /// One shape per WMO group asset, shared by every placement of it. The `Arc` keeps the
    /// group, and so the pointer key, alive.
    shapes: HashMap<usize, (Arc<WmoGroupCollision>, Gd<ConcavePolygonShape3D>)>,
}

impl WmoCollisionBodies {
    /// Queue the global WMO and the WMOs of newly parsed tiles, then build pending groups
    /// nearest the player first until the frame budget runs out.
    pub fn sync(&mut self, parent: &mut Gd<Node3D>, terrain: &StreamedTerrain, player: Vec3) {
        self.queue_new(terrain);
        if self.pending.is_empty() {
            return;
        }
        let placements = &self.placements;
        // Nearest last, so the build pops it first.
        self.pending.sort_by_cached_key(|pending| {
            std::cmp::Reverse(ordered_distance(
                &placements[pending.placement],
                &pending.group,
                player,
            ))
        });
        let start = Instant::now();
        while start.elapsed() < BUILD_BUDGET
            && let Some(pending) = self.pending.pop()
        {
            self.build_group(parent, pending);
        }
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        *self = Self::default();
    }

    fn queue_new(&mut self, terrain: &StreamedTerrain) {
        if let Some(global) = terrain
            .map_wdt
            .as_ref()
            .and_then(|map| map.global_wmo.as_ref())
        {
            self.queue(PlacementKey::Global, None, &global.collision);
        }
        for (tile, parsed) in &terrain.parsed_tiles {
            if !self.tiles.insert(*tile) {
                continue;
            }
            for (unique_id, wmo) in &parsed.wmo_floors {
                self.queue(PlacementKey::Modf(*unique_id), Some(*tile), wmo);
            }
        }
    }

    /// Groups of the WMOs `tile` queued that are not built yet; `None` until it is queued.
    pub fn tile_pending(&self, tile: (u32, u32)) -> Option<usize> {
        self.tiles.contains(&tile).then(|| {
            self.pending
                .iter()
                .filter(|pending| self.placements[pending.placement].tile == Some(tile))
                .count()
        })
    }

    /// Local groups, including a WMO queued by a neighbouring tile. Collision
    /// outside the entry bubble keeps building after the loading screen hides.
    pub fn nearby_pending(&self, player: Vec3) -> Option<usize> {
        let tiles = crate::loading::nearby_tiles(player);
        tiles.iter().all(|tile| self.tiles.contains(tile)).then(|| {
            self.pending
                .iter()
                .filter(|pending| {
                    let placement = &self.placements[pending.placement];
                    group_distance(placement, &pending.group, player)
                        <= crate::terrain::object_progress::ENTRY_RADIUS
                })
                .count()
        })
    }

    fn queue(&mut self, key: PlacementKey, tile: Option<(u32, u32)>, wmo: &WmoCollision) {
        if !self.queued.insert(key) {
            return;
        }
        let placement = self.placements.len();
        let world_from_local = wmo.world_from_local();
        self.placements.push(Placement {
            key,
            tile,
            world_from_local,
            local_from_world: world_from_local.inverse(),
            body: None,
        });
        self.pending.extend(
            wmo.groups()
                .iter()
                .enumerate()
                .map(|(index, group)| PendingGroup {
                    placement,
                    index,
                    group: Arc::clone(group),
                }),
        );
    }

    fn build_group(&mut self, parent: &mut Gd<Node3D>, pending: PendingGroup) {
        let Some(shape) = self.group_shape(&pending.group) else {
            return;
        };
        let mut node = CollisionShape3D::new_alloc();
        node.set_name(&format!("Group{}", pending.index));
        node.set_shape(&shape);
        let placement = &self.placements[pending.placement];
        let mut body = match &placement.body {
            Some(body) => body.clone(),
            None => {
                let body = self.spawn_body(parent, placement.key, placement.world_from_local);
                self.placements[pending.placement].body = Some(body.clone());
                body
            }
        };
        body.add_child(&node);
    }

    fn spawn_body(
        &mut self,
        parent: &mut Gd<Node3D>,
        key: PlacementKey,
        world_from_local: Affine3A,
    ) -> Gd<StaticBody3D> {
        let mut body = StaticBody3D::new_alloc();
        body.set_name(&match key {
            PlacementKey::Global => "GlobalWmoCollision".to_owned(),
            PlacementKey::Modf(id) => format!("WmoCollision{id}"),
        });
        body.set_collision_layer(WMO_LAYER);
        body.set_collision_mask(0);
        let (scale, rotation, translation) = world_from_local.to_scale_rotation_translation();
        body.set_position(Vector3::from_array(translation.to_array()));
        body.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        body.set_scale(Vector3::from_array(scale.to_array()));
        self.root
            .get_or_insert_with(|| {
                let mut root = Node3D::new_alloc();
                root.set_name("WmoCollision");
                parent.add_child(&root);
                root
            })
            .add_child(&body);
        body
    }

    fn group_shape(&mut self, group: &Arc<WmoGroupCollision>) -> Option<Gd<ConcavePolygonShape3D>> {
        let key = Arc::as_ptr(group) as usize;
        if let Some((_, shape)) = self.shapes.get(&key) {
            return Some(shape.clone());
        }
        let faces: PackedVector3Array = group
            .collidable_triangles()
            .flatten()
            .map(|corner| Vector3::from_array(corner.to_array()))
            .collect();
        if faces.is_empty() {
            return None;
        }
        let mut shape = ConcavePolygonShape3D::new_gd();
        // Rays hit faces from either side, as the shared floor test does.
        shape.set_backface_collision_enabled(true);
        shape.set_faces(&faces);
        self.shapes.insert(key, (Arc::clone(group), shape.clone()));
        Some(shape)
    }
}

/// Distance to the first WMO wall on the ray from `origin`, if within `length`.
pub(crate) fn wall_hit(
    space: &Gd<PhysicsDirectSpaceState3D>,
    origin: Vec3,
    direction: Vec3,
    length: f32,
) -> Option<f32> {
    let from = Vector3::from_array(origin.to_array());
    let to = from + Vector3::from_array(direction.to_array()) * length;
    let mut query = PhysicsRayQueryParameters3D::create(from, to)
        .expect("Godot could not allocate wall ray parameters");
    query.set_collision_mask(WMO_LAYER);
    let hit = space.clone().intersect_ray(&query);
    let position = hit.get("position")?.to::<Vector3>();
    Some(from.distance_to(position))
}

/// Distance from the player to the group's bounding box, in whole centimetres for ordering.
fn ordered_distance(placement: &Placement, group: &WmoGroupCollision, player: Vec3) -> u32 {
    let distance = group_distance(placement, group, player);
    (distance * 100.0).min(u32::MAX as f32) as u32
}

fn group_distance(placement: &Placement, group: &WmoGroupCollision, player: Vec3) -> f32 {
    let local = placement.local_from_world.transform_point3(player);
    let (min, max) = group.local_bounds();
    let scale = placement.world_from_local.matrix3.x_axis.length();
    (local - local.clamp(min, max)).length() * scale
}

#[cfg(test)]
#[path = "collision_tests.rs"]
mod tests;
