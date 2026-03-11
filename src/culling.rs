use std::collections::{HashSet, VecDeque};

use bevy::camera::primitives::Frustum;
use bevy::prelude::*;

use crate::game_state_enum::GameState;

type DoodadFilter = (With<Doodad>, Without<TerrainChunk>, Without<Camera3d>);
type WmoFilter = (With<Wmo>, Without<Doodad>, Without<TerrainChunk>, Without<Camera3d>);

/// Marker for terrain chunk entities. Stores precomputed world center for distance checks.
#[derive(Component)]
pub struct TerrainChunk {
    pub world_center: Vec3,
}

/// Marker for doodad (M2 prop) root entities.
#[derive(Component)]
pub struct Doodad;

/// Marker for WMO root entities.
#[derive(Component)]
pub struct Wmo;

/// Marker for a WMO group entity (child of a Wmo root). Stores the group index
/// and its AABB in WMO-local space (from MOGI).
#[derive(Component)]
pub struct WmoGroup {
    pub group_index: u16,
    pub bbox_min: Vec3,
    pub bbox_max: Vec3,
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
    mut doodads: Query<(&Transform, &mut Visibility), DoodadFilter>,
    mut wmos: Query<(&Transform, &mut Visibility), WmoFilter>,
) {
    let Ok(cam) = camera_q.single() else { return };
    let cam_pos = cam.translation;

    if cam_pos.distance_squared(last_pos.0) < config.update_threshold_sq {
        return;
    }
    last_pos.0 = cam_pos;

    for (chunk, mut vis) in &mut chunks {
        let desired = if cam_pos.distance_squared(chunk.world_center) < config.chunk_distance_sq {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != desired {
            *vis = desired;
        }
    }

    for (tf, mut vis) in &mut doodads {
        let desired = if cam_pos.distance_squared(tf.translation) < config.doodad_distance_sq {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != desired {
            *vis = desired;
        }
    }

    for (tf, mut vis) in &mut wmos {
        let desired = if cam_pos.distance_squared(tf.translation) < config.wmo_distance_sq {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != desired {
            *vis = desired;
        }
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
            let desired = if visible_set.contains(&group.group_index) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    type CullState = SystemState<(
        Res<'static, CullingConfig>,
        ResMut<'static, LastCullPosition>,
        Query<'static, 'static, &'static Transform, With<Camera3d>>,
        Query<'static, 'static, (&'static TerrainChunk, &'static mut Visibility)>,
        Query<
            'static,
            'static,
            (&'static Transform, &'static mut Visibility),
            (With<Doodad>, Without<TerrainChunk>, Without<Camera3d>),
        >,
        Query<
            'static,
            'static,
            (&'static Transform, &'static mut Visibility),
            (
                With<Wmo>,
                Without<Doodad>,
                Without<TerrainChunk>,
                Without<Camera3d>,
            ),
        >,
    )>;

    fn setup_world(cam_pos: Vec3, threshold_sq: f32) -> (World, CullState) {
        let mut world = World::default();
        world.insert_resource(CullingConfig {
            chunk_distance_sq: threshold_sq,
            doodad_distance_sq: threshold_sq,
            wmo_distance_sq: threshold_sq,
            update_threshold_sq: 0.0,
        });
        world.insert_resource(LastCullPosition(Vec3::new(f32::MAX, 0.0, 0.0)));
        world.spawn((Camera3d::default(), Transform::from_translation(cam_pos)));
        let state = SystemState::new(&mut world);
        (world, state)
    }

    fn run_cull(world: &mut World, state: &mut CullState) {
        let (config, last_pos, camera_q, chunks, doodads, wmos) = state.get_mut(world);
        distance_cull_system(config, last_pos, camera_q, chunks, doodads, wmos);
        state.apply(world);
    }

    #[test]
    fn chunk_within_range_stays_visible() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 100.0 * 100.0);
        let e = world
            .spawn((
                TerrainChunk {
                    world_center: Vec3::new(50.0, 0.0, 0.0),
                },
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
    }

    #[test]
    fn chunk_beyond_range_gets_hidden() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 100.0 * 100.0);
        let e = world
            .spawn((
                TerrainChunk {
                    world_center: Vec3::new(200.0, 0.0, 0.0),
                },
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn doodad_culled_by_distance() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let near = world
            .spawn((
                Doodad,
                Transform::from_xyz(10.0, 0.0, 0.0),
                Visibility::Visible,
            ))
            .id();
        let far = world
            .spawn((
                Doodad,
                Transform::from_xyz(100.0, 0.0, 0.0),
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
        assert_eq!(*world.get::<Visibility>(far).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn wmo_culled_by_distance() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let near = world
            .spawn((
                Wmo,
                Transform::from_xyz(0.0, 0.0, 30.0),
                Visibility::Visible,
            ))
            .id();
        let far = world
            .spawn((
                Wmo,
                Transform::from_xyz(0.0, 0.0, 300.0),
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
        assert_eq!(*world.get::<Visibility>(far).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn hidden_object_becomes_visible_when_camera_approaches() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let e = world
            .spawn((
                Doodad,
                Transform::from_xyz(100.0, 0.0, 0.0),
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Hidden);

        // Move camera close
        let cam = world
            .query_filtered::<Entity, With<Camera3d>>()
            .single(&world)
            .unwrap();
        world.get_mut::<Transform>(cam).unwrap().translation = Vec3::new(90.0, 0.0, 0.0);
        world.resource_mut::<LastCullPosition>().0 = Vec3::new(f32::MAX, 0.0, 0.0);

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
    }

    #[test]
    fn skips_update_when_camera_hasnt_moved_enough() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        world.resource_mut::<CullingConfig>().update_threshold_sq = 1000.0 * 1000.0;
        world.resource_mut::<LastCullPosition>().0 = Vec3::ZERO;

        let e = world
            .spawn((
                Doodad,
                Transform::from_xyz(100.0, 0.0, 0.0),
                Visibility::Visible,
            ))
            .id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
    }
}
