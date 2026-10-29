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

fn triangles_block_step(triangles: &[[Vec3; 3]], from: Vec3, to: Vec3, top: f32) -> bool {
    game_engine_core::step_clearance::step_wall_blocked(
        triangles,
        from,
        to,
        top,
        shared::ground::min_walkable_normal_y(),
    )
}

#[test]
fn ramp_lip_full_stockade_walks_from_captured_feet_onto_landing() {
    let terrain = load("stormwindjail", (32, 32), |terrain| {
        terrain.map_wdt.is_some()
    });
    let collision = &terrain
        .map_wdt
        .as_ref()
        .unwrap()
        .global_wmo
        .as_ref()
        .unwrap()
        .collision;
    let start = Vec3::new(108.2204, -26.26199, 3.454892);
    let triangles = world_triangles([collision], start, 8.0);
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = start;
    for _ in 0..7 {
        let frame = crate::gameplay::MovementFrame {
            direction: [0.96152194, 0.0, -0.27472818],
            speed: 7.0,
            vertical: 0.0,
        };
        feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
    }
    assert!(feet.x > 108.8, "authored WMO stopped walk at {feet}");
    assert!(
        (feet.y + 25.606234).abs() < 0.001,
        "not on upper floor: {feet}"
    );
}

#[test]
fn neltharus_gorek_upper_route_stays_on_authored_floor() {
    let terrain = load("2519", (32, 32), |terrain| {
        terrain.parsed_tiles.contains_key(&(32, 32))
    });
    let wmos = &terrain.parsed_tiles[&(32, 32)].wmo_floors;
    assert!(!wmos.is_empty(), "Neltharus WMO collision is missing");
    let start = Vec3::new(-220.0404, 133.3703, 556.2401);
    let triangles = world_triangles(wmos.iter().map(|(_, wmo)| wmo), start, 100.0);
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    let route = [
        (-229.17159, 540.3221),
        (-229.54646, 539.6685),
        (-230.1866, 538.5526),
        (-230.541, 537.9348),
        (-233.18022, 533.3333),
        (-234.18103, 531.5893),
        (-234.6583, 530.32135),
        (-234.836, 529.7375),
        (-235.5357, 527.4384),
        (-235.72304, 526.82294),
        (-236.07706, 525.6597),
        (-235.67775, 525.26044),
    ];
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = start;
    for (x, z) in route {
        let target = Vec3::new(x, feet.y, z);
        for _ in 0..400 {
            let offset = (target - feet).with_y(0.0);
            if offset.length() < 0.55 {
                break;
            }
            let direction = offset.normalize();
            let frame = crate::gameplay::MovementFrame {
                direction: direction.to_array(),
                speed: 7.0,
                vertical: 0.0,
            };
            feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
            assert!(
                feet.y > 131.0,
                "fell from upper route at {feet}, target {target}"
            );
        }
        assert!(
            (target - feet).with_y(0.0).length() < 0.55,
            "blocked at {feet}, target {target}"
        );
    }
}

#[test]
fn neltharus_guarded_route_crosses_recorded_second_drop() {
    let terrain = load("2519", (33, 32), |terrain| {
        terrain.parsed_tiles.contains_key(&(33, 32))
    });
    let wmos = &terrain.parsed_tiles[&(33, 32)].wmo_floors;
    assert!(!wmos.is_empty(), "Neltharus WMO collision is missing");
    let start = Vec3::new(-268.8306, 58.54118, 603.0437);
    let target = Vec3::new(-268.5627, 58.54118, 602.3644);
    let triangles = world_triangles(wmos.iter().map(|(_, wmo)| wmo), start, 20.0);
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = start;
    for _ in 0..7 {
        let frame = crate::gameplay::MovementFrame {
            direction: (target - start).normalize().to_array(),
            speed: 7.0,
            vertical: 0.0,
        };
        feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
        assert!(feet.y > 57.0, "fell from guarded route at {feet}");
    }
    assert!(
        (target - feet).with_y(0.0).length() < 0.2,
        "blocked at {feet}"
    );
}

#[test]
fn neltharus_player_keeps_ground_at_recorded_upper_route() {
    let terrain = load("2519", (33, 32), |terrain| {
        terrain.parsed_tiles.contains_key(&(33, 32))
    });
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &|_, _, _| None,
        step_wall_blocked: &|_, _, _| false,
        game_object_floor: &|_| None,
    };
    let feet = Vec3::new(-201.5208, 133.4988, 588.5245);
    let support = ground
        .trace_support(feet)
        .expect("recorded upper route has collision support");
    assert!((support.height - 133.3707).abs() < 0.1, "{support:?}");
    let start = Vec3::new(-200.8241, 133.4988, 589.739);
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut current = start;
    for _ in 0..12 {
        let frame = crate::gameplay::MovementFrame {
            direction: (feet - start).normalize().to_array(),
            speed: 7.0,
            vertical: 0.0,
        };
        current = movement.predict(
            current,
            frame,
            false,
            &ground,
            &mut |_, to, _| to,
            1.0 / 60.0,
        );
        assert!(current.y > 130.0, "player fell at {current}");
    }
}

#[test]
fn neltharus_return_crosses_recorded_gorek_lip() {
    let terrain = load("2519", (32, 32), |terrain| {
        terrain.parsed_tiles.contains_key(&(32, 32))
    });
    let start = Vec3::new(-248.251, 123.114, 468.9084);
    let target = Vec3::new(-247.91733, 124.4, 468.22916);
    let triangles = world_triangles(
        terrain
            .parsed_tiles
            .values()
            .flat_map(|tile| tile.wmo_floors.iter().map(|(_, wmo)| wmo)),
        start,
        30.0,
    );
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    let direction = (target - start).with_y(0.0).normalize();
    for distance in [0.116667, 0.3, 0.75, 1.0, 1.25, 1.6, 2.0] {
        let point = start + direction * distance;
        let wmos = || {
            terrain
                .parsed_tiles
                .values()
                .flat_map(|tile| tile.wmo_floors.iter().map(|(_, wmo)| wmo))
        };
        let solid = shared::ground::collision_height(point, None, wmos());
        let walkable = shared::ground::ground_at(point, None, wmos());
        eprintln!(
            "probe distance={distance} point={point:?} solid={solid:?} walkable={walkable:?} wall={:?}",
            walls(start + Vec3::Y * 0.6, direction, distance + 0.05)
        );
    }
    let landing = (start + direction * 0.75).with_y(124.090256);
    let raised = start.with_y(landing.y);
    eprintln!(
        "raised clearance wall={:?} step_blocked={}",
        walls(raised + Vec3::Y * 0.6, direction, 0.8),
        triangles_block_step(&triangles, raised, landing, start.y + 1.6)
    );
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = start;
    for tick in 0..12 {
        let frame = crate::gameplay::MovementFrame {
            direction: direction.to_array(),
            speed: 7.0,
            vertical: 0.0,
        };
        let next = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
        eprintln!(
            "tick={tick} before={feet:?} after={next:?} support={:?}",
            ground.trace_support(feet)
        );
        assert!(next.y > 120.0, "fell at {next:?}");
        feet = next;
    }
    assert!(
        (feet - start).with_y(0.0).length() > 0.5,
        "uphill return blocked at {feet:?}"
    );
}

#[test]
fn neltharus_steep_face_supports_player_and_blocks_uphill_motion() {
    let terrain = load("2519", (32, 32), |terrain| {
        terrain.parsed_tiles.contains_key(&(32, 32))
    });
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &|_, _, _| None,
        step_wall_blocked: &|_, _, _| false,
        game_object_floor: &|_| None,
    };
    let start = Vec3::new(-233.50781, 133.42572, 532.7626);
    for distance in [0.1, 0.3, 0.5, 0.776, 1.0, 1.6] {
        let point = start + Vec3::new(-distance, 0.0, 0.0);
        let wmos = || {
            terrain
                .parsed_tiles
                .values()
                .flat_map(|tile| tile.wmo_floors.iter().map(|(_, wmo)| wmo))
        };
        eprintln!(
            "steep distance={distance} point={point:?} solid={:?} walkable={:?}",
            shared::ground::collision_height(point, None, wmos()),
            shared::ground::ground_at(point, None, wmos())
        );
    }
    let support = ground.trace_support(start).expect("solid WMO support");
    assert_eq!(support.source, "wmo");
    assert_eq!(support.wmo_placement, Some(33788514));
    assert_eq!(support.wmo_group, Some(8));
    assert!((support.height - start.y).abs() < 0.02);
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = start;
    for _ in 0..30 {
        let frame = crate::gameplay::MovementFrame {
            direction: [-1.0, 0.0, 0.0],
            speed: 7.0,
            vertical: 0.0,
        };
        feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
        assert!(feet.y > 133.0, "fell through steep WMO face: {feet}");
    }
    assert!(
        feet.x > start.x - 0.2,
        "walked uphill through steep face: {feet}"
    );
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
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
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
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
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
            *feet = movement.predict(*feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
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

/// Outside the Jasperlode Mine (azeroth_33_49, WMO 111538), on the terrain in front of the
/// entrance ramp. Kobold Miner 281565 spawns on the mouth floor at WoW z 62.95 (TDB).
const JASPERLODE_OUTSIDE: [f32; 3] = [-9205.0, -599.0, 61.8];
/// Up the entrance ramp (WMO group 6, from WoW x -9197) between the frame's posts and the
/// rock (lane y -598.5 to -599.5), over the terrain hole of the mouth, then along the tunnel
/// floor to the exploration trigger (Jasperlode 87, centre -9077.3, -552.9): a breadth-first
/// search over `validate_move` on this ground keeping a yard clear of every blocked move,
/// in WoW (x, y) legs of at most 4.5 yd (2.25 through the mouth).
/// `godot/tests/world_quest_flow.gd` walks the same route.
const JASPERLODE_ROUTE: [[f32; 2]; 49] = [
    [-9202.75, -599.0],
    [-9200.5, -599.0],
    [-9198.25, -599.0],
    [-9196.0, -599.0],
    [-9193.75, -599.0],
    [-9191.5, -599.0],
    [-9189.25, -599.0],
    [-9187.0, -599.0],
    [-9184.75, -599.0],
    [-9182.5, -599.0],
    [-9180.25, -599.0],
    [-9178.0, -599.0],
    [-9175.75, -599.0],
    [-9173.5, -599.0],
    [-9171.25, -599.0],
    [-9169.0, -599.0],
    [-9166.75, -599.0],
    [-9164.5, -599.0],
    [-9162.25, -599.0],
    [-9160.0, -599.0],
    [-9160.0, -598.0],
    [-9158.0, -596.0],
    [-9153.8, -596.0],
    [-9149.6, -596.0],
    [-9145.4, -596.0],
    [-9141.2, -596.0],
    [-9137.0, -596.0],
    [-9134.5, -593.5],
    [-9132.0, -591.0],
    [-9129.5, -588.5],
    [-9129.5, -586.0],
    [-9129.5, -583.5],
    [-9126.9, -580.9],
    [-9124.3, -578.3],
    [-9121.7, -575.7],
    [-9119.1, -573.1],
    [-9116.5, -570.5],
    [-9114.0, -570.5],
    [-9112.0, -569.0],
    [-9110.0, -567.5],
    [-9106.0, -567.5],
    [-9102.0, -567.5],
    [-9098.0, -567.5],
    [-9094.9, -564.4],
    [-9091.8, -561.3],
    [-9088.7, -558.2],
    [-9085.6, -555.1],
    [-9082.5, -552.0],
    [-9079.0, -550.5],
];
/// The route of the live quest run that stopped at the mouth (WoW -9175.0, -595.6, 62.0) and,
/// at collapsing frame rates, fell through the world: on the terrain under the entrance rock
/// straight east into the mouth's terrain hole.
const JASPERLODE_UNDER_THE_ROCK: [f32; 3] = [-9185.0, -598.0, 61.5];

fn jasperlode() -> (StreamedTerrain, Vec<[Vec3; 3]>) {
    let terrain = load("azeroth", (33, 49), |terrain| {
        terrain.parsed_tiles.contains_key(&(33, 49))
    });
    let triangles = world_triangles(
        terrain.parsed_tiles[&(33, 49)]
            .wmo_floors
            .iter()
            .map(|(_, wmo)| wmo),
        wow(-9140.0, -575.0, 60.0),
        120.0,
    );
    (terrain, triangles)
}

/// Run along `route` (WoW positions) from `start` at `delta` seconds a frame for at most
/// `seconds`, steering at the next point; the feet at the end and the lowest feet height.
fn run_route(
    ground: &TerrainGround<'_>,
    start: [f32; 3],
    route: &[[f32; 2]],
    delta: f32,
    seconds: f32,
) -> (Vec3, f32) {
    let mut movement = crate::gameplay::PlayerMovement::default();
    let mut feet = wow(start[0], start[1], start[2]);
    let mut lowest = feet.y;
    let mut next = 0;
    for _ in 0..(seconds / delta) as usize {
        let [x, y] = route[next];
        let to = (wow(x, y, 0.0) - feet).with_y(0.0);
        if to.length() < (shared::movement::RUN_SPEED * delta).max(1.0) {
            if next + 1 == route.len() {
                break;
            }
            next += 1;
            continue;
        }
        let frame = crate::gameplay::MovementFrame {
            direction: to.normalize().to_array(),
            speed: shared::movement::RUN_SPEED,
            vertical: 0.0,
        };
        feet = movement.predict(feet, frame, false, ground, &mut |_, to, _| to, delta);
        lowest = lowest.min(feet.y);
    }
    (feet, lowest)
}

fn walk_into_jasperlode(delta: f32) {
    let (terrain, triangles) = jasperlode();
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    let (feet, lowest) = run_route(&ground, JASPERLODE_OUTSIDE, &JASPERLODE_ROUTE, delta, 60.0);
    let center = wow(-9077.3, -552.9, 60.3);
    assert!(
        (feet - center).with_y(0.0).length() < 4.0,
        "stopped at WoW ({:.1}, {:.1}, {:.1})",
        feet.x,
        -feet.z,
        feet.y
    );
    assert!(lowest > 55.0, "fell to {lowest}");
}

/// Retail walks up the Jasperlode Mine's entrance ramp, over the terrain hole of the mouth
/// (no terrain there: TrinityCore `GridMap::getHeight` returns `INVALID_HEIGHT` in a hole),
/// and along the tunnel floor to the exploration trigger.
#[test]
fn player_walks_up_the_jasperlode_ramp_into_the_mine() {
    walk_into_jasperlode(1.0 / 60.0);
}

/// A collapsing frame rate takes the same walk: the move does not skip the ground rules.
#[test]
fn player_walks_into_the_jasperlode_mine_at_two_frames_a_second() {
    walk_into_jasperlode(0.5);
}

/// The hole of the mine mouth has no terrain; the terrain beside it does.
#[test]
fn jasperlode_mouth_hole_has_no_terrain_height() {
    let (terrain, _) = jasperlode();
    let beside = wow(-9176.0, -595.8, 0.0);
    let hole = wow(-9172.0, -595.8, 0.0);
    let height = terrain
        .height_at(beside.x, beside.z)
        .expect("terrain beside the hole");
    assert!((height - 61.5).abs() < 0.1, "{height}");
    assert_eq!(terrain.height_at(hole.x, hole.z), None);
}

/// The live run's walk from under the entrance rock east into the mouth stops at the hole's
/// edge at 60 frames a second and at two; it never drops the player through the world.
#[test]
fn walking_into_the_jasperlode_hillside_never_drops_the_player() {
    let (terrain, triangles) = jasperlode();
    let walls = |origin, direction, length| ray_hit(&triangles, origin, direction, length);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| triangles_block_step(&triangles, from, to, top),
        game_object_floor: &|_| None,
    };
    for delta in [1.0 / 60.0, 0.5, 2.0] {
        let (feet, lowest) = run_route(
            &ground,
            JASPERLODE_UNDER_THE_ROCK,
            &[[-9137.3, -592.9]],
            delta,
            10.0,
        );
        assert!(
            lowest > 60.0,
            "{delta} s frames fell to {lowest} at WoW ({:.1}, {:.1})",
            feet.x,
            -feet.z
        );
    }
}
