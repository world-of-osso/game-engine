use super::*;
use bevy::ecs::system::SystemState;
use bevy::math::primitives::ViewFrustum;

type CullState = SystemState<(
    Res<'static, CullingConfig>,
    ResMut<'static, LastCullPosition>,
    Query<'static, 'static, &'static Transform, With<Camera3d>>,
    Query<'static, 'static, (&'static TerrainChunk, &'static mut Visibility)>,
    Query<
        'static,
        'static,
        (
            &'static Transform,
            Option<&'static ChunkRefs>,
            &'static mut Visibility,
        ),
        (With<Doodad>, Without<TerrainChunk>, Without<Camera3d>),
    >,
    Query<
        'static,
        'static,
        (
            &'static Transform,
            Option<&'static WmoRootBounds>,
            Option<&'static ChunkRefs>,
            &'static mut Visibility,
        ),
        (
            With<Wmo>,
            Without<Doodad>,
            Without<TerrainChunk>,
            Without<Camera3d>,
        ),
    >,
)>;
type PortalCullState = SystemState<(
    Query<'static, 'static, (&'static GlobalTransform, &'static Frustum), With<Camera3d>>,
    Query<'static, 'static, (Entity, &'static GlobalTransform, &'static WmoPortalGraph), With<Wmo>>,
    WmoGroupCullQuery<'static, 'static>,
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
    let (config, last_pos, camera_q, chunks, doodads, wmos) = state
        .get_mut(world)
        .expect("distance culling test SystemState should initialize");
    distance_cull_system(config, last_pos, camera_q, chunks, doodads, wmos);
    state.apply(world);
}

fn run_portal_cull(world: &mut World, state: &mut PortalCullState) {
    let (camera_q, wmo_q, group_q) = state
        .get_mut(world)
        .expect("portal culling test SystemState should initialize");
    wmo_portal_cull_system(camera_q, wmo_q, group_q);
    state.apply(world);
}

fn unit_test_frustum() -> Frustum {
    Frustum(ViewFrustum::from_clip_from_world(&Mat4::IDENTITY))
}

/// Interior group 0 around the origin camera, portal 0 to group 1 beside it.
fn spawn_portal_test_wmo(world: &mut World, portal_verts: Vec<Vec3>) -> (Entity, Entity, Entity) {
    let root = world
        .spawn((
            Wmo,
            GlobalTransform::IDENTITY,
            WmoPortalGraph {
                adjacency: vec![vec![(0, 1)], vec![(0, 0)]],
                portal_verts: vec![portal_verts],
            },
        ))
        .id();
    let group0 = spawn_portal_test_group(world, 0, Vec3::splat(-0.5), Vec3::splat(0.5), false);
    world.entity_mut(group0).insert(floor_below_origin());
    let group1 = spawn_portal_test_group(
        world,
        1,
        Vec3::new(2.0, -0.5, -0.5),
        Vec3::new(3.0, 0.5, 0.5),
        false,
    );
    world.entity_mut(root).add_children(&[group0, group1]);
    (root, group0, group1)
}

fn spawn_portal_test_group(
    world: &mut World,
    group_index: u16,
    bbox_min: Vec3,
    bbox_max: Vec3,
    is_exterior: bool,
) -> Entity {
    world
        .spawn((
            WmoGroup {
                group_index,
                bbox_min,
                bbox_max,
                is_exterior,
                is_antiportal: false,
            },
            Visibility::Visible,
        ))
        .id()
}

/// One floor triangle 0.4 below the origin.
fn floor_below_origin() -> WmoInteriorFloor {
    WmoInteriorFloor {
        triangles: vec![[
            Vec3::new(-0.5, -0.4, -0.5),
            Vec3::new(0.5, -0.4, -0.5),
            Vec3::new(0.0, -0.4, 0.5),
        ]],
    }
}

/// A 0.1-wide portal quad facing Z around `center`.
fn small_quad_at(center: Vec3) -> Vec<Vec3> {
    quad_facing_z(center, 0.05)
}

fn quad_facing_z(center: Vec3, half_size: f32) -> Vec<Vec3> {
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .into_iter()
        .map(|(x, y)| center + Vec3::new(x * half_size, y * half_size, 0.0))
        .collect()
}

/// A 60° perspective camera at the origin looking down -Z.
fn spawn_perspective_test_camera(world: &mut World) {
    use bevy::camera::{CameraProjection, PerspectiveProjection};
    let projection = PerspectiveProjection {
        fov: 60f32.to_radians(),
        aspect_ratio: 16.0 / 9.0,
        ..default()
    };
    world.spawn((
        Camera3d::default(),
        GlobalTransform::IDENTITY,
        projection.compute_frustum(&GlobalTransform::IDENTITY),
    ));
}

fn spawn_portal_test_camera(world: &mut World) {
    world.spawn((
        Camera3d::default(),
        GlobalTransform::IDENTITY,
        unit_test_frustum(),
    ));
}

fn visibility_of(world: &World, entity: Entity) -> Visibility {
    *world.get::<Visibility>(entity).unwrap()
}

#[test]
fn chunk_within_range_stays_visible() {
    let (mut world, mut state) = setup_world(Vec3::ZERO, 100.0 * 100.0);
    let e = world
        .spawn((
            TerrainChunk {
                chunk_index: 0,
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
                chunk_index: 0,
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
    world.spawn((
        TerrainChunk {
            chunk_index: 0,
            world_center: Vec3::new(10.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let near = world
        .spawn((
            Doodad,
            Transform::from_xyz(10.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
            Visibility::Visible,
        ))
        .id();
    let far = world
        .spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
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
    world.spawn((
        TerrainChunk {
            chunk_index: 0,
            world_center: Vec3::new(0.0, 0.0, 30.0),
        },
        Visibility::Visible,
    ));
    let near = world
        .spawn((
            Wmo,
            Transform::from_xyz(0.0, 0.0, 30.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
            Visibility::Visible,
        ))
        .id();
    let far = world
        .spawn((
            Wmo,
            Transform::from_xyz(0.0, 0.0, 300.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
    assert_eq!(*world.get::<Visibility>(far).unwrap(), Visibility::Hidden);
}

#[test]
fn wmo_uses_root_bounds_for_distance_culling() {
    let (mut world, mut state) = setup_world(Vec3::new(45.0, 0.0, 0.0), 10.0 * 10.0);
    world.spawn((
        TerrainChunk {
            chunk_index: 0,
            world_center: Vec3::new(50.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let entity = world
        .spawn((
            Wmo,
            Transform::from_xyz(500.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
            WmoRootBounds {
                world_min: Vec3::new(40.0, -5.0, -5.0),
                world_max: Vec3::new(60.0, 5.0, 5.0),
            },
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(
        *world.get::<Visibility>(entity).unwrap(),
        Visibility::Visible
    );
}

#[test]
fn hidden_object_becomes_visible_when_camera_approaches() {
    let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
    world.spawn((
        TerrainChunk {
            chunk_index: 0,
            world_center: Vec3::new(100.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let e = world
        .spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
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

    world.spawn((
        TerrainChunk {
            chunk_index: 0,
            world_center: Vec3::new(100.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let e = world
        .spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![0],
            },
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
}

#[test]
fn doodad_hidden_when_all_referenced_chunks_are_hidden() {
    let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
    world.spawn((
        TerrainChunk {
            chunk_index: 1,
            world_center: Vec3::new(200.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let entity = world
        .spawn((
            Doodad,
            Transform::from_xyz(10.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![1],
            },
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(
        *world.get::<Visibility>(entity).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn wmo_stays_visible_when_any_referenced_chunk_is_visible() {
    let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
    world.spawn((
        TerrainChunk {
            chunk_index: 1,
            world_center: Vec3::new(200.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    world.spawn((
        TerrainChunk {
            chunk_index: 2,
            world_center: Vec3::new(10.0, 0.0, 0.0),
        },
        Visibility::Visible,
    ));
    let entity = world
        .spawn((
            Wmo,
            Transform::from_xyz(10.0, 0.0, 0.0),
            ChunkRefs {
                chunk_indices: vec![1, 2],
            },
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(
        *world.get::<Visibility>(entity).unwrap(),
        Visibility::Visible
    );
}

#[test]
fn portal_culling_hides_groups_behind_non_visible_portals() {
    let mut world = World::default();
    spawn_portal_test_camera(&mut world);
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, small_quad_at(Vec3::new(5.0, 5.0, 5.0)));
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Hidden);
}

#[test]
fn portal_culling_keeps_groups_visible_through_visible_portals() {
    let mut world = World::default();
    spawn_portal_test_camera(&mut world);
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, small_quad_at(Vec3::new(0.25, 0.25, 0.25)));
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Visible);
}

/// A doorway 5 yd ahead that is wider than the view: every corner lies outside the frustum,
/// yet the camera looks straight through it.
#[test]
fn a_doorway_filling_the_view_draws_the_group_behind_it() {
    let mut world = World::default();
    spawn_perspective_test_camera(&mut world);
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, quad_facing_z(Vec3::new(0.0, 0.0, -5.0), 20.0));
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Visible);
}

/// A camera passing through a doorway, 0.05 yd from its plane and so inside the near
/// plane: both sides stay drawn.
#[test]
fn a_camera_in_a_doorway_draws_the_group_beyond_it() {
    let mut world = World::default();
    spawn_perspective_test_camera(&mut world);
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, quad_facing_z(Vec3::new(0.0, 0.0, -0.05), 3.0));
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Visible);
}

/// A portal off to the side of the view and far from the camera stays closed.
#[test]
fn a_doorway_outside_the_view_hides_the_group_behind_it() {
    let mut world = World::default();
    spawn_perspective_test_camera(&mut world);
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, quad_facing_z(Vec3::new(30.0, 0.0, -5.0), 2.0));
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Hidden);
}

/// A camera inside an interior group's box but with no floor of it below stands outside:
/// exterior groups are drawn, the interior only through a visible portal, antiportals never.
#[test]
fn camera_outside_interiors_draws_exterior_groups() {
    let mut world = World::default();
    spawn_portal_test_camera(&mut world);
    let (root, group0, group1) =
        spawn_portal_test_wmo(&mut world, small_quad_at(Vec3::new(5.0, 5.0, 5.0)));
    world.entity_mut(group0).remove::<WmoInteriorFloor>();
    world.get_mut::<WmoGroup>(group1).unwrap().is_exterior = true;
    let antiportal = world
        .spawn((
            WmoGroup {
                group_index: 2,
                bbox_min: Vec3::splat(-10.0),
                bbox_max: Vec3::splat(10.0),
                is_exterior: false,
                is_antiportal: true,
            },
            Visibility::Visible,
        ))
        .id();
    world.entity_mut(root).add_child(antiportal);
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Hidden);
    assert_eq!(visibility_of(&world, group1), Visibility::Visible);
    assert_eq!(visibility_of(&world, antiportal), Visibility::Hidden);
}

/// Once an interior portal opens onto an exterior group the whole exterior is drawn,
/// including exterior groups with no portal to the camera's group.
#[test]
fn interior_portal_onto_exterior_draws_every_exterior_group() {
    let mut world = World::default();
    spawn_portal_test_camera(&mut world);
    let (root, group0, group1) =
        spawn_portal_test_wmo(&mut world, small_quad_at(Vec3::new(0.25, 0.25, 0.25)));
    world.get_mut::<WmoGroup>(group1).unwrap().is_exterior = true;
    let far_exterior =
        spawn_portal_test_group(&mut world, 2, Vec3::splat(50.0), Vec3::splat(60.0), true);
    world.entity_mut(root).add_child(far_exterior);
    world
        .get_mut::<WmoPortalGraph>(root)
        .unwrap()
        .adjacency
        .push(Vec::new());
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(visibility_of(&world, group0), Visibility::Visible);
    assert_eq!(visibility_of(&world, group1), Visibility::Visible);
    assert_eq!(visibility_of(&world, far_exterior), Visibility::Visible);
}

// --- Pure function tests ---

#[test]
fn distance_sq_to_aabb_point_outside() {
    let min = Vec3::new(10.0, 10.0, 10.0);
    let max = Vec3::new(20.0, 20.0, 20.0);
    let point = Vec3::new(0.0, 15.0, 15.0);
    let dist_sq = distance_sq_to_aabb(point, min, max);
    // Only X contributes: (10-0)^2 = 100
    assert!((dist_sq - 100.0).abs() < 0.01);
}

#[test]
fn distance_sq_to_aabb_point_inside() {
    let min = Vec3::new(0.0, 0.0, 0.0);
    let max = Vec3::new(10.0, 10.0, 10.0);
    let point = Vec3::new(5.0, 5.0, 5.0);
    assert_eq!(distance_sq_to_aabb(point, min, max), 0.0);
}

#[test]
fn distance_sq_to_aabb_point_on_surface() {
    let min = Vec3::new(0.0, 0.0, 0.0);
    let max = Vec3::new(10.0, 10.0, 10.0);
    let point = Vec3::new(10.0, 5.0, 5.0);
    assert_eq!(distance_sq_to_aabb(point, min, max), 0.0);
}

#[test]
fn distance_sq_to_aabb_corner() {
    let min = Vec3::new(0.0, 0.0, 0.0);
    let max = Vec3::new(1.0, 1.0, 1.0);
    let point = Vec3::new(2.0, 2.0, 2.0);
    // Distance to corner: (1,1,1) → sqrt(3) → sq = 3
    assert!((distance_sq_to_aabb(point, min, max) - 3.0).abs() < 0.01);
}

#[test]
fn chunk_refs_visible_no_refs_always_visible() {
    let visible_chunks = HashSet::new();
    assert!(chunk_refs_visible(None, &visible_chunks));
}

#[test]
fn chunk_refs_visible_empty_indices_always_visible() {
    let refs = ChunkRefs {
        chunk_indices: vec![],
    };
    let visible_chunks = HashSet::new();
    assert!(chunk_refs_visible(Some(&refs), &visible_chunks));
}

#[test]
fn chunk_refs_visible_none_matching() {
    let refs = ChunkRefs {
        chunk_indices: vec![5, 6, 7],
    };
    let visible_chunks: HashSet<u16> = [1, 2, 3].into_iter().collect();
    assert!(!chunk_refs_visible(Some(&refs), &visible_chunks));
}

#[test]
fn chunk_refs_visible_one_matching() {
    let refs = ChunkRefs {
        chunk_indices: vec![5, 6, 7],
    };
    let visible_chunks: HashSet<u16> = [6].into_iter().collect();
    assert!(chunk_refs_visible(Some(&refs), &visible_chunks));
}

#[test]
fn doodad_without_chunk_refs_uses_distance_only() {
    let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
    // No chunk refs → visible if within distance
    let near = world
        .spawn((
            Doodad,
            Transform::from_xyz(10.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();

    run_cull(&mut world, &mut state);
    assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
}
