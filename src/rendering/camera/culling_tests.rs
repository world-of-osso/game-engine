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
    Query<'static, 'static, (&'static WmoGroup, &'static mut Visibility, &'static ChildOf)>,
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

fn run_portal_cull(world: &mut World, state: &mut PortalCullState) {
    let (camera_q, wmo_q, group_q) = state.get_mut(world);
    wmo_portal_cull_system(camera_q, wmo_q, group_q);
    state.apply(world);
}

fn unit_test_frustum() -> Frustum {
    Frustum::from_clip_from_world(&Mat4::IDENTITY)
}

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
    let group0 = spawn_portal_test_group(world, 0, Vec3::splat(-0.5), Vec3::splat(0.5));
    let group1 = spawn_portal_test_group(
        world,
        1,
        Vec3::new(2.0, -0.5, -0.5),
        Vec3::new(3.0, 0.5, 0.5),
    );
    world.entity_mut(root).add_children(&[group0, group1]);
    (root, group0, group1)
}

fn spawn_portal_test_group(
    world: &mut World,
    group_index: u16,
    bbox_min: Vec3,
    bbox_max: Vec3,
) -> Entity {
    world
        .spawn((
            WmoGroup {
                group_index,
                bbox_min,
                bbox_max,
                is_antiportal: false,
            },
            Visibility::Visible,
        ))
        .id()
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
    world.spawn((
        Camera3d::default(),
        GlobalTransform::IDENTITY,
        unit_test_frustum(),
    ));
    let (_root, group0, group1) = spawn_portal_test_wmo(&mut world, vec![Vec3::new(5.0, 5.0, 5.0)]);
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(
        *world.get::<Visibility>(group0).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        *world.get::<Visibility>(group1).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn portal_culling_keeps_groups_visible_through_visible_portals() {
    let mut world = World::default();
    world.spawn((
        Camera3d::default(),
        GlobalTransform::IDENTITY,
        unit_test_frustum(),
    ));
    let (_root, group0, group1) =
        spawn_portal_test_wmo(&mut world, vec![Vec3::new(0.25, 0.25, 0.25)]);
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(
        *world.get::<Visibility>(group0).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        *world.get::<Visibility>(group1).unwrap(),
        Visibility::Visible
    );
}

#[test]
fn antiportal_groups_occlude_groups_behind_them() {
    let mut world = World::default();
    world.spawn((
        Camera3d::default(),
        GlobalTransform::IDENTITY,
        unit_test_frustum(),
    ));
    let (root, group0, group1) =
        spawn_portal_test_wmo(&mut world, vec![Vec3::new(0.25, 0.25, 0.25)]);
    let antiportal = world
        .spawn((
            WmoGroup {
                group_index: 2,
                bbox_min: Vec3::new(1.0, -0.25, -0.25),
                bbox_max: Vec3::new(1.5, 0.25, 0.25),
                is_antiportal: true,
            },
            Visibility::Visible,
        ))
        .id();
    world.entity_mut(root).add_child(antiportal);
    let mut state = PortalCullState::new(&mut world);

    run_portal_cull(&mut world, &mut state);

    assert_eq!(
        *world.get::<Visibility>(group0).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        *world.get::<Visibility>(group1).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        *world.get::<Visibility>(antiportal).unwrap(),
        Visibility::Hidden
    );
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
