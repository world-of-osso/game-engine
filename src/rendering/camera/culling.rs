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
    pub is_antiportal: bool,
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

/// BFS from camera group through portals visible in the frustum.
fn bfs_visible_groups(
    start_group: u16,
    graph: &WmoPortalGraph,
    frustum: &Frustum,
    wmo_transform: &GlobalTransform,
) -> HashSet<u16> {
    let mut visible = HashSet::new();
    visible.insert(start_group);
    let mut queue = VecDeque::new();
    queue.push_back(start_group);

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

fn group_center(group: &WmoGroup) -> Vec3 {
    (group.bbox_min + group.bbox_max) * 0.5
}

fn antiportal_occludes_group(
    camera_local: Vec3,
    group: &WmoGroup,
    antiportal_groups: &[WmoGroup],
) -> bool {
    let group_center = group_center(group);
    antiportal_groups.iter().any(|antiportal| {
        antiportal.group_index != group.group_index
            && segment_intersects_aabb(
                camera_local,
                group_center,
                antiportal.bbox_min,
                antiportal.bbox_max,
            )
    })
}

fn segment_intersects_aabb(start: Vec3, end: Vec3, min: Vec3, max: Vec3) -> bool {
    let delta = end - start;
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = 1.0;

    for axis in 0..3 {
        let start_axis = start[axis];
        let delta_axis = delta[axis];
        let min_axis = min[axis];
        let max_axis = max[axis];

        if delta_axis.abs() <= f32::EPSILON {
            if start_axis < min_axis || start_axis > max_axis {
                return false;
            }
            continue;
        }

        let inv_delta = delta_axis.recip();
        let mut axis_t0 = (min_axis - start_axis) * inv_delta;
        let mut axis_t1 = (max_axis - start_axis) * inv_delta;
        if axis_t0 > axis_t1 {
            std::mem::swap(&mut axis_t0, &mut axis_t1);
        }
        t_min = t_min.max(axis_t0);
        t_max = t_max.min(axis_t1);
        if t_min > t_max {
            return false;
        }
    }

    true
}

/// Portal-based visibility culling for WMO interiors.
fn wmo_portal_cull_system(
    camera_q: Query<(&GlobalTransform, &Frustum), With<Camera3d>>,
    wmo_q: Query<(Entity, &GlobalTransform, &WmoPortalGraph), With<Wmo>>,
    mut group_q: Query<(&WmoGroup, &mut Visibility, &ChildOf)>,
) {
    let Ok((cam_gtf, frustum)) = camera_q.single() else {
        return;
    };
    let cam_pos = cam_gtf.translation();

    for (wmo_entity, wmo_gtf, graph) in &wmo_q {
        let local_cam = wmo_gtf.affine().inverse().transform_point3(cam_pos);

        // Collect group info for camera detection (immutable pass)
        let camera_group = find_camera_group_from_query(local_cam, wmo_entity, &group_q);
        let antiportal_groups = antiportal_groups_from_query(wmo_entity, &group_q);

        // Not inside any group = outside the WMO, skip portal culling
        let Some(cam_group) = camera_group else {
            continue;
        };

        let visible_set = bfs_visible_groups(cam_group, graph, frustum, wmo_gtf);

        // Apply visibility (mutable pass)
        for (group, mut vis, child_of) in &mut group_q {
            if child_of.parent() != wmo_entity {
                continue;
            }
            let visible_through_portals = visible_set.contains(&group.group_index);
            let hidden_by_antiportal =
                antiportal_occludes_group(local_cam, group, &antiportal_groups);
            let should_show_group =
                !group.is_antiportal && visible_through_portals && !hidden_by_antiportal;
            let desired = if should_show_group {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if *vis != desired {
                *vis = desired;
            }
        }
    }
}

/// Find which group the camera is in, filtering by WMO parent.
fn find_camera_group_from_query(
    local_cam: Vec3,
    wmo_entity: Entity,
    group_q: &Query<(&WmoGroup, &mut Visibility, &ChildOf)>,
) -> Option<u16> {
    for (group, _, child_of) in group_q.iter() {
        if child_of.parent() != wmo_entity {
            continue;
        }
        if local_cam.x >= group.bbox_min.x
            && local_cam.y >= group.bbox_min.y
            && local_cam.z >= group.bbox_min.z
            && local_cam.x <= group.bbox_max.x
            && local_cam.y <= group.bbox_max.y
            && local_cam.z <= group.bbox_max.z
        {
            return Some(group.group_index);
        }
    }
    None
}

fn antiportal_groups_from_query(
    wmo_entity: Entity,
    group_q: &Query<(&WmoGroup, &mut Visibility, &ChildOf)>,
) -> Vec<WmoGroup> {
    group_q
        .iter()
        .filter_map(|(group, _, child_of)| {
            (child_of.parent() == wmo_entity && group.is_antiportal).then_some(*group)
        })
        .collect()
}

#[cfg(test)]
#[path = "culling_tests.rs"]
mod tests;
