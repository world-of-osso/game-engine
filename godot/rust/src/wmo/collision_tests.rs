//! The camera against the real WMO collision faces the physics bodies are built from.

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use game_engine_core::{camera_control_data::CameraState, camera_follow_data::EYE_HEIGHT};
use glam::Vec3;
use shared::ground::WmoCollision;

use crate::{camera::follow_pose, ground::TerrainGround, terrain::streaming::StreamedTerrain};

/// WoW `(x, y, z)` to this client's world `(x, z, -y)`.
fn wow(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, z, -y)
}

fn load(map: &str, tile: (u32, u32), ready: impl Fn(&StreamedTerrain) -> bool) -> StreamedTerrain {
    let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut terrain = StreamedTerrain::new(data_root.clone());
    terrain.request_map(map.into(), tile).unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    while !ready(&terrain) {
        terrain.poll().expect("terrain worker alive");
        assert!(
            Instant::now() < deadline,
            "{map} timed out: {:?}",
            terrain.state().map_error
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    terrain
}

/// World triangles of `wmos` within `radius` of `center`, placed as the physics bodies are.
fn world_triangles<'a>(
    wmos: impl IntoIterator<Item = &'a WmoCollision>,
    center: Vec3,
    radius: f32,
) -> Vec<[Vec3; 3]> {
    wmos.into_iter()
        .flat_map(|wmo| {
            let world_from_local = wmo.world_from_local();
            wmo.groups()
                .iter()
                .flat_map(|group| group.collidable_triangles().collect::<Vec<_>>())
                .map(move |corners| corners.map(|c| world_from_local.transform_point3(c)))
        })
        .filter(|corners| {
            corners
                .iter()
                .any(|corner| corner.distance(center) < radius)
        })
        .collect()
}

/// Nearest two-sided hit within `length`, as the Godot ray query reports it.
fn ray_hit(triangles: &[[Vec3; 3]], origin: Vec3, direction: Vec3, length: f32) -> Option<f32> {
    triangles
        .iter()
        .filter_map(|&[a, b, c]| {
            let (edge1, edge2) = (b - a, c - a);
            let p = direction.cross(edge2);
            let det = edge1.dot(p);
            if det.abs() < 1e-8 {
                return None;
            }
            let s = origin - a;
            let u = s.dot(p) / det;
            let q = s.cross(edge1);
            let v = direction.dot(q) / det;
            let t = edge2.dot(q) / det;
            (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && (0.0..=length).contains(&t)).then_some(t)
        })
        .reduce(f32::min)
}

/// Whether a solid face lies between the eye and the camera: the view is from outside a wall.
fn wall_between(triangles: &[[Vec3; 3]], eye: Vec3, camera: Vec3) -> bool {
    let offset = camera - eye;
    ray_hit(triangles, eye, offset.normalize(), offset.length()).is_some()
}

struct Follow<'a> {
    state: CameraState,
    position: Vec3,
    terrain: &'a StreamedTerrain,
    triangles: &'a [[Vec3; 3]],
}

impl Follow<'_> {
    fn step(&mut self, player: Vec3) -> Vec3 {
        let triangles = self.triangles;
        let pose = follow_pose(
            &mut self.state,
            self.position,
            player,
            1.0 / 60.0,
            self.terrain,
            |origin, direction, length| ray_hit(triangles, origin, direction, length),
        );
        self.position = pose.position;
        pose.position
    }
}

fn camera(yaw: f32, pitch: f32, distance: f32) -> CameraState {
    CameraState {
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        distance,
        target_distance: distance,
        ..Default::default()
    }
}

/// Stormwind's Stockade entrance stairwell (`sw_magicdistrict` 321999 group 58, on
/// azeroth_30_48): the player walks from the top step (WoW -8774, 838) to the doorway with the
/// camera low behind them, the original client's `camera_collision.rs` walk. The stairwell walls
/// hold the camera in, and no frame frames the player from behind a wall.
#[test]
fn camera_walking_down_the_stockade_stairs_stays_inside_the_stairwell() {
    let terrain = load("azeroth", (30, 48), |terrain| {
        terrain.parsed_tiles.contains_key(&(30, 48))
    });
    let top = wow(-8774.0, 838.0, 92.1);
    let doorway = wow(-8766.1, 845.5, 88.0);
    let triangles = world_triangles(
        terrain.parsed_tiles[&(30, 48)]
            .wmo_floors
            .iter()
            .map(|(_, wmo)| wmo),
        top,
        40.0,
    );
    assert!(!triangles.is_empty(), "no WMO faces around the stairwell");
    let mut follow = Follow {
        state: camera(-50.0, 10.0, 10.0),
        position: top + Vec3::Y * 2.0,
        terrain: &terrain,
        triangles: &triangles,
    };
    for _ in 0..300 {
        follow.step(top);
    }
    let mut outside = Vec::new();
    for step in 0..=90 {
        let player = top.lerp(doorway, step as f32 / 90.0);
        let position = follow.step(player);
        if wall_between(&triangles, player + Vec3::Y * EYE_HEIGHT, position) {
            outside.push(step);
        }
    }
    let mut position = follow.position;
    for _ in 0..60 {
        position = follow.step(doorway);
    }
    let eye = doorway + Vec3::Y * EYE_HEIGHT;
    assert_eq!(outside, Vec::<u32>::new(), "frames behind a wall");
    assert!(
        position.distance(eye) < 9.0,
        "the stairwell walls hold the camera in, got {:.1} yd",
        position.distance(eye)
    );
    assert!(!wall_between(&triangles, eye, position));
}

/// Inside the Stockade (`stormwindjail`, global WMO 108631) on a cell-block floor, a 15 yd
/// camera at every yaw settles where the eye still sees it, pulled in by the walls where the
/// orbit would pass through them.
#[test]
fn stockade_interior_walls_keep_the_camera_in_sight_at_every_yaw() {
    let terrain = load("stormwindjail", (32, 32), |terrain| {
        terrain
            .map_wdt
            .as_ref()
            .is_some_and(|map| map.global_wmo.is_some())
    });
    let wmo = &terrain
        .map_wdt
        .as_ref()
        .unwrap()
        .global_wmo
        .as_ref()
        .unwrap();
    assert_eq!(wmo.asset.root_fdid, 108631);
    let player = Vec3::new(103.0, -34.9, -76.0);
    let eye = player + Vec3::Y * EYE_HEIGHT;
    let triangles = world_triangles([&wmo.collision], player, 40.0);
    let mut distances = Vec::new();
    for yaw in (0..360).step_by(45) {
        let mut follow = Follow {
            state: camera(yaw as f32, 20.0, 15.0),
            position: player + Vec3::Y * 2.0,
            terrain: &terrain,
            triangles: &triangles,
        };
        let mut position = follow.position;
        for _ in 0..300 {
            position = follow.step(player);
        }
        assert!(
            !wall_between(&triangles, eye, position),
            "yaw {yaw}: camera behind a wall at {position}"
        );
        distances.push((yaw, position.distance(eye)));
    }
    assert!(
        distances.iter().any(|&(_, distance)| distance < 12.0),
        "no yaw pulled in: {distances:?}"
    );
}

fn stairwell() -> (StreamedTerrain, Vec<[Vec3; 3]>) {
    let terrain = load("azeroth", (30, 48), |terrain| {
        terrain.parsed_tiles.contains_key(&(30, 48))
    });
    let triangles = world_triangles(
        terrain.parsed_tiles[&(30, 48)]
            .wmo_floors
            .iter()
            .map(|(_, wmo)| wmo),
        wow(-8774.0, 838.0, 92.1),
        40.0,
    );
    (terrain, triangles)
}

/// The original client's stairs test: on the Stockade entrance stairs (WoW -8774, 838) an 8 yd
/// move sideways, across the stairwell (WMO-local -Y of the district, yaw 38.5°), meets the
/// stairwell wall 6.2 yd away. It went the full 8.0 yd through the wall without WMO walls.
#[test]
fn player_on_the_stockade_stairs_is_blocked_by_the_stairwell_wall() {
    let (terrain, triangles) = stairwell();
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
    };
    let current = wow(-8774.0, 838.0, 92.1);
    let proposed = current + wow(-0.622, 0.783, 0.0) * 8.0;

    let moved = ground.validate_move(current, proposed, true);

    let walked = (moved - current).with_y(0.0).length();
    assert!(
        walked < 6.3,
        "walked {walked:.2} yd through the stairwell wall"
    );
}

/// Between the stairwell walls the player still runs down the stairs from the top step to the
/// lower steps, and back up to the top.
#[test]
fn player_runs_down_and_back_up_the_stockade_stairs_between_the_walls() {
    let (terrain, triangles) = stairwell();
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
    };
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = Vec3::new(-8786.0, 96.1, -836.0);
    let mut run = |feet: &mut Vec3, direction: [f32; 3]| {
        for _ in 0..120 {
            let frame = crate::gameplay::MovementFrame {
                direction,
                speed: shared::movement::RUN_SPEED,
                vertical: 0.0,
            };
            *feet = movement.predict(*feet, frame, false, &ground, 1.0 / 60.0);
        }
    };
    run(&mut feet, [1.0, 0.0, 0.0]);
    assert!(feet.x > -8773.0, "stopped on the way down at {feet}");
    assert!(
        (90.5..92.5).contains(&feet.y),
        "not on the lower steps: {feet}"
    );
    run(&mut feet, [-1.0, 0.0, 0.0]);
    assert!(feet.x < -8784.0, "stopped on the way up at {feet}");
    assert!(feet.y > 95.5, "not back on the top step: {feet}");
}
