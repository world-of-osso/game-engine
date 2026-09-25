use std::collections::{HashSet, VecDeque};

use bevy::camera::primitives::Frustum;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::game_state_enum::GameState;

type DoodadFilter = (With<Doodad>, Without<TerrainChunk>, Without<Camera3d>);
type WmoFilter = (
    With<Wmo>,
    Without<Doodad>,
    Without<TerrainChunk>,
    Without<Camera3d>,
);
type DoodadCullQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        Option<&'static ChunkRefs>,
        &'static mut Visibility,
    ),
    DoodadFilter,
>;
type WmoCullQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        Option<&'static WmoRootBounds>,
        Option<&'static ChunkRefs>,
        &'static mut Visibility,
    ),
    WmoFilter,
>;

/// Marker for terrain chunk entities. Stores precomputed world center for distance checks.
#[derive(Component)]
pub struct TerrainChunk {
    pub chunk_index: u16,
    pub world_center: Vec3,
}

/// ADT chunk indices that reference a spawned doodad or WMO.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ChunkRefs {
    pub chunk_indices: Vec<u16>,
}

/// Marker for doodad (M2 prop) root entities.
#[derive(Component)]
pub struct Doodad;

#[path = "doodad_collision.rs"]
mod doodad_collision;
pub use doodad_collision::{DoodadCollider, DoodadVisualBounds, compute_world_aabb};

/// Marker for WMO root entities.
#[derive(Component)]
pub struct Wmo;

/// Absolute world-space bounds from ADT MODF placement extents.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WmoRootBounds {
    pub world_min: Vec3,
    pub world_max: Vec3,
}

/// Marker for a WMO group entity (child of a Wmo root). Stores the group index
/// and its AABB in WMO-local space (from MOGI).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WmoGroup {
    pub group_index: u16,
    pub bbox_min: Vec3,
    pub bbox_max: Vec3,
    /// MOGP `EXTERIOR` (0x8). Groups without it are interiors.
    pub is_exterior: bool,
    pub is_antiportal: bool,
}

/// Render triangles of an interior WMO group in WMO-local Bevy space. Portal culling uses
/// them to tell whether the camera stands inside the group or only inside its bounding box.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct WmoInteriorFloor {
    pub triangles: Vec<[Vec3; 3]>,
}

/// Portal culling data stored on the WMO root entity.
/// Contains the portal graph needed for BFS visibility traversal.
#[derive(Component)]
pub struct WmoPortalGraph {
    /// Per-group list of (portal_index, destination_group_index).
    pub adjacency: Vec<Vec<(usize, u16)>>,
    /// Portal polygon vertices in WMO-local space (converted to Bevy coords).
    pub portal_verts: Vec<Vec<Vec3>>,
}

/// Distance thresholds for culling. Objects beyond these distances are hidden.
#[derive(Resource)]
pub struct CullingConfig {
    pub chunk_distance_sq: f32,
    pub doodad_distance_sq: f32,
    pub wmo_distance_sq: f32,
    pub update_threshold_sq: f32,
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            chunk_distance_sq: 400.0 * 400.0,
            doodad_distance_sq: 200.0 * 200.0,
            wmo_distance_sq: 2000.0 * 2000.0,
            update_threshold_sq: 5.0 * 5.0,
        }
    }
}

#[derive(Resource, Default)]
struct LastCullPosition(Vec3);

pub struct CullingPlugin;

impl Plugin for CullingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CullingConfig>()
            .init_resource::<LastCullPosition>()
            .add_systems(
                Update,
                (distance_cull_system, wmo_portal_cull_system).run_if(in_state(GameState::InWorld)),
            );
    }
}

fn distance_cull_system(
    config: Res<CullingConfig>,
    mut last_pos: ResMut<LastCullPosition>,
    camera_q: Query<&Transform, With<Camera3d>>,
    mut chunks: Query<(&TerrainChunk, &mut Visibility)>,
    mut doodads: DoodadCullQuery,
    mut wmos: WmoCullQuery,
) {
    let Ok(cam) = camera_q.single() else { return };
    let cam_pos = cam.translation;

    if should_skip_cull_update(cam_pos, &mut last_pos, &config) {
        return;
    }

    let visible_chunks = update_chunk_visibility(cam_pos, config.chunk_distance_sq, &mut chunks);
    update_transform_visibility(
        cam_pos,
        config.doodad_distance_sq,
        &visible_chunks,
        &mut doodads,
    );
    update_wmo_visibility(cam_pos, config.wmo_distance_sq, &visible_chunks, &mut wmos);
}

fn should_skip_cull_update(
    cam_pos: Vec3,
    last_pos: &mut LastCullPosition,
    config: &CullingConfig,
) -> bool {
    if cam_pos.distance_squared(last_pos.0) < config.update_threshold_sq {
        return true;
    }
    last_pos.0 = cam_pos;
    false
}

fn update_chunk_visibility(
    cam_pos: Vec3,
    max_distance_sq: f32,
    chunks: &mut Query<(&TerrainChunk, &mut Visibility)>,
) -> HashSet<u16> {
    let mut visible_chunks = HashSet::new();
    for (chunk, mut vis) in chunks {
        let visible = cam_pos.distance_squared(chunk.world_center) < max_distance_sq;
        apply_visibility(visible, &mut vis);
        if visible {
            visible_chunks.insert(chunk.chunk_index);
        }
    }
    visible_chunks
}

fn update_transform_visibility<F>(
    cam_pos: Vec3,
    max_distance_sq: f32,
    visible_chunks: &HashSet<u16>,
    query: &mut Query<(&Transform, Option<&ChunkRefs>, &mut Visibility), F>,
) where
    F: QueryFilter,
{
    for (tf, chunk_refs, mut vis) in query {
        let visible = cam_pos.distance_squared(tf.translation) < max_distance_sq
            && chunk_refs_visible(chunk_refs, visible_chunks);
        apply_visibility(visible, &mut vis);
    }
}

fn update_wmo_visibility(
    cam_pos: Vec3,
    max_distance_sq: f32,
    visible_chunks: &HashSet<u16>,
    wmos: &mut WmoCullQuery,
) {
    for (transform, bounds, chunk_refs, mut visibility) in wmos {
        let distance_sq = bounds
            .map(|bounds| distance_sq_to_aabb(cam_pos, bounds.world_min, bounds.world_max))
            .unwrap_or_else(|| cam_pos.distance_squared(transform.translation));
        let visible =
            distance_sq < max_distance_sq && chunk_refs_visible(chunk_refs, visible_chunks);
        apply_visibility(visible, &mut visibility);
    }
}

fn distance_sq_to_aabb(point: Vec3, min: Vec3, max: Vec3) -> f32 {
    let dx = if point.x < min.x {
        min.x - point.x
    } else if point.x > max.x {
        point.x - max.x
    } else {
        0.0
    };
    let dy = if point.y < min.y {
        min.y - point.y
    } else if point.y > max.y {
        point.y - max.y
    } else {
        0.0
    };
    let dz = if point.z < min.z {
        min.z - point.z
    } else if point.z > max.z {
        point.z - max.z
    } else {
        0.0
    };
    dx * dx + dy * dy + dz * dz
}

fn chunk_refs_visible(chunk_refs: Option<&ChunkRefs>, visible_chunks: &HashSet<u16>) -> bool {
    let Some(chunk_refs) = chunk_refs else {
        return true;
    };
    chunk_refs.chunk_indices.is_empty()
        || chunk_refs
            .chunk_indices
            .iter()
            .any(|chunk_index| visible_chunks.contains(chunk_index))
}

fn apply_visibility(visible: bool, vis: &mut Visibility) {
    let desired = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *vis != desired {
        *vis = desired;
    }
}

// ── WMO portal culling ──────────────────────────────────────────────────────

type WmoGroupCullQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static WmoGroup,
        Option<&'static WmoInteriorFloor>,
        &'static mut Visibility,
        &'static ChildOf,
    ),
>;

/// BFS from the start groups through portals visible in the frustum.
fn bfs_visible_groups(
    start_groups: &[u16],
    graph: &WmoPortalGraph,
    frustum: &Frustum,
    wmo_transform: &GlobalTransform,
) -> HashSet<u16> {
    let mut visible: HashSet<u16> = start_groups.iter().copied().collect();
    let mut queue: VecDeque<u16> = start_groups.iter().copied().collect();

    while let Some(current) = queue.pop_front() {
        let Some(neighbors) = graph.adjacency.get(current as usize) else {
            continue;
        };
        for &(portal_idx, dest_group) in neighbors {
            if visible.contains(&dest_group) {
                continue;
            }
            if portal_in_frustum(graph, portal_idx, frustum, wmo_transform) {
                visible.insert(dest_group);
                queue.push_back(dest_group);
            }
        }
    }

    visible
}

/// Check if a portal polygon has any vertex inside the camera frustum.
fn portal_in_frustum(
    graph: &WmoPortalGraph,
    portal_idx: usize,
    frustum: &Frustum,
    wmo_transform: &GlobalTransform,
) -> bool {
    let Some(verts) = graph.portal_verts.get(portal_idx) else {
        return false;
    };
    if verts.is_empty() {
        return true; // No geometry = assume visible
    }
    // Check if any portal vertex is inside all frustum half-spaces
    for local_v in verts {
        let world_v = wmo_transform.transform_point(*local_v);
        if point_in_frustum(world_v, frustum) {
            return true;
        }
    }
    false
}

/// Test if a point is inside all 6 frustum half-spaces.
fn point_in_frustum(point: Vec3, frustum: &Frustum) -> bool {
    let point = Vec3A::from(point);
    for half_space in &frustum.half_spaces {
        let normal = half_space.normal();
        let d = half_space.d();
        if normal.dot(point) + d < 0.0 {
            return false;
        }
    }
    true
}

/// Height of the highest floor triangle directly below `point` (WMO-local Bevy space, Y up).
fn floor_height_below(floor: &WmoInteriorFloor, point: Vec3) -> Option<f32> {
    floor
        .triangles
        .iter()
        .filter_map(|triangle| triangle_height_at(triangle, point.x, point.z))
        .filter(|height| *height <= point.y)
        .reduce(f32::max)
}

/// Height of the triangle's plane at (x, z) when that point lies inside its XZ projection.
fn triangle_height_at(triangle: &[Vec3; 3], x: f32, z: f32) -> Option<f32> {
    let [a, b, c] = *triangle;
    let det = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
    if det.abs() <= f32::EPSILON {
        return None;
    }
    let wa = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / det;
    let wb = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / det;
    let wc = 1.0 - wa - wb;
    (wa >= 0.0 && wb >= 0.0 && wc >= 0.0).then_some(wa * a.y + wb * b.y + wc * c.y)
}

fn bbox_contains(group: &WmoGroup, point: Vec3) -> bool {
    point.cmpge(group.bbox_min).all() && point.cmple(group.bbox_max).all()
}

/// Portal-based visibility culling for WMO groups.
fn wmo_portal_cull_system(
    camera_q: Query<(&GlobalTransform, &Frustum), With<Camera3d>>,
    wmo_q: Query<(Entity, &GlobalTransform, &WmoPortalGraph), With<Wmo>>,
    mut group_q: WmoGroupCullQuery,
) {
    let Ok((cam_gtf, frustum)) = camera_q.single() else {
        return;
    };
    let cam_pos = cam_gtf.translation();

    for (wmo_entity, wmo_gtf, graph) in &wmo_q {
        cull_wmo_portal_visibility(wmo_entity, wmo_gtf, graph, frustum, cam_pos, &mut group_q);
    }
}

/// Retail/WebWowViewerCpp traversal: inside an interior group, draw what its portals reach and
/// the whole exterior once a portal opens onto it; otherwise draw every exterior group and the
/// interiors whose portals are in view.
fn cull_wmo_portal_visibility(
    wmo_entity: Entity,
    wmo_gtf: &GlobalTransform,
    graph: &WmoPortalGraph,
    frustum: &Frustum,
    cam_pos: Vec3,
    group_q: &mut WmoGroupCullQuery,
) {
    let local_cam = wmo_gtf.affine().inverse().transform_point3(cam_pos);
    let exterior_groups = exterior_groups_from_query(wmo_entity, group_q);

    let visible_set = match find_camera_interior_group(local_cam, wmo_entity, group_q) {
        Some(cam_group) => {
            let mut visible = bfs_visible_groups(&[cam_group], graph, frustum, wmo_gtf);
            if exterior_groups.iter().any(|group| visible.contains(group)) {
                visible.extend(bfs_visible_groups(
                    &exterior_groups,
                    graph,
                    frustum,
                    wmo_gtf,
                ));
            }
            visible
        }
        None => bfs_visible_groups(&exterior_groups, graph, frustum, wmo_gtf),
    };
    apply_portal_group_visibility(wmo_entity, &visible_set, group_q);
}

fn apply_portal_group_visibility(
    wmo_entity: Entity,
    visible_set: &HashSet<u16>,
    group_q: &mut WmoGroupCullQuery,
) {
    for (group, _, mut vis, child_of) in group_q {
        if child_of.parent() != wmo_entity {
            continue;
        }
        let should_show_group = !group.is_antiportal && visible_set.contains(&group.group_index);
        apply_visibility(should_show_group, &mut vis);
    }
}

/// The interior group the camera stands in: its bounding box contains the camera and it has
/// a floor below it. Among several, the closest floor wins (WebWowViewerCpp
/// `getGroupWmoThatCameraIsInside`).
fn find_camera_interior_group(
    local_cam: Vec3,
    wmo_entity: Entity,
    group_q: &WmoGroupCullQuery,
) -> Option<u16> {
    group_q
        .iter()
        .filter(|(group, _, _, child_of)| {
            child_of.parent() == wmo_entity && bbox_contains(group, local_cam)
        })
        .filter_map(|(group, floor, _, _)| {
            let floor_height = floor_height_below(floor?, local_cam)?;
            Some((group.group_index, floor_height))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(group_index, _)| group_index)
}

fn exterior_groups_from_query(wmo_entity: Entity, group_q: &WmoGroupCullQuery) -> Vec<u16> {
    group_q
        .iter()
        .filter(|(group, _, _, child_of)| child_of.parent() == wmo_entity && group.is_exterior)
        .map(|(group, _, _, _)| group.group_index)
        .collect()
}

#[cfg(test)]
#[path = "culling_tests.rs"]
mod tests;
