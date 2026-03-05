use bevy::prelude::*;

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
            wmo_distance_sq: 500.0 * 500.0,
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
            .add_systems(Update, distance_cull_system);
    }
}

fn distance_cull_system(
    config: Res<CullingConfig>,
    mut last_pos: ResMut<LastCullPosition>,
    camera_q: Query<&Transform, With<Camera3d>>,
    mut chunks: Query<(&TerrainChunk, &mut Visibility)>,
    mut doodads: Query<
        (&Transform, &mut Visibility),
        (With<Doodad>, Without<TerrainChunk>, Without<Camera3d>),
    >,
    mut wmos: Query<
        (&Transform, &mut Visibility),
        (With<Wmo>, Without<Doodad>, Without<TerrainChunk>, Without<Camera3d>),
    >,
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
            (With<Wmo>, Without<Doodad>, Without<TerrainChunk>, Without<Camera3d>),
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
        let e = world.spawn((
            TerrainChunk { world_center: Vec3::new(50.0, 0.0, 0.0) },
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
    }

    #[test]
    fn chunk_beyond_range_gets_hidden() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 100.0 * 100.0);
        let e = world.spawn((
            TerrainChunk { world_center: Vec3::new(200.0, 0.0, 0.0) },
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn doodad_culled_by_distance() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let near = world.spawn((
            Doodad,
            Transform::from_xyz(10.0, 0.0, 0.0),
            Visibility::Visible,
        )).id();
        let far = world.spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
        assert_eq!(*world.get::<Visibility>(far).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn wmo_culled_by_distance() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let near = world.spawn((
            Wmo,
            Transform::from_xyz(0.0, 0.0, 30.0),
            Visibility::Visible,
        )).id();
        let far = world.spawn((
            Wmo,
            Transform::from_xyz(0.0, 0.0, 300.0),
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(near).unwrap(), Visibility::Visible);
        assert_eq!(*world.get::<Visibility>(far).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn hidden_object_becomes_visible_when_camera_approaches() {
        let (mut world, mut state) = setup_world(Vec3::ZERO, 50.0 * 50.0);
        let e = world.spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Hidden);

        // Move camera close
        let cam = world.query_filtered::<Entity, With<Camera3d>>().single(&world).unwrap();
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

        let e = world.spawn((
            Doodad,
            Transform::from_xyz(100.0, 0.0, 0.0),
            Visibility::Visible,
        )).id();

        run_cull(&mut world, &mut state);
        assert_eq!(*world.get::<Visibility>(e).unwrap(), Visibility::Visible);
    }
}
