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
