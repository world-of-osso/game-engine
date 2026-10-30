//! Captured Stockade group15 FDID108647 faces, in native world coordinates.
use super::*;
use shared::los::LosTile;
use std::path::PathBuf;

const FEET: Vec3 = Vec3::new(108.2204, -26.26199, 3.454892);
const HEADING: Vec3 = Vec3::new(0.96152194, 0.0, -0.27472818);
const UPPER_Y: f32 = -25.606234;

fn authored_faces() -> Vec<[Vec3; 3]> {
    vec![
        // Face3153: the vertical lip hit 0.04996 yd along the old feet+0.6 ray.
        [
            Vec3::new(108.26844024658203, -25.606233596801758, 4.110162258148193),
            Vec3::new(108.26844024658203, -25.606233596801758, 1.8079357147216797),
            Vec3::new(108.26844024658203, -25.911788940429688, 2.575342893600464),
        ],
        // Face3412: the ramp supporting the captured feet.
        [
            Vec3::new(108.26844024658203, -26.522899627685547, 4.110160827636719),
            Vec3::new(108.26844024658203, -26.217344284057617, 3.3427515029907227),
            Vec3::new(106.88365173339844, -26.522899627685547, 4.110159397125244),
        ],
        // Face3041: the walkable floor beyond the lip (rise 0.6557564 yd).
        [
            Vec3::new(108.55583190917969, -25.606233596801758, 1.4387787580490112),
            Vec3::new(108.26844024658203, -25.606233596801758, 1.8079357147216797),
            Vec3::new(108.26844024658203, -25.606233596801758, 4.110162258148193),
        ],
    ]
}

fn collision_tile(faces: &[[Vec3; 3]]) -> LosTile {
    let vertices = faces.iter().flatten().copied().collect();
    let triangles = (0..faces.len() as u32)
        .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
        .collect();
    LosTile::build(32, 32, vertices, triangles)
}

fn walk(faces: &[[Vec3; 3]], snap: bool) -> Vec3 {
    walk_frames(faces, snap, 7.0, 60.0, 1)
}

fn walk_frames(faces: &[[Vec3; 3]], snap: bool, speed: f32, fps: f32, frames: usize) -> Vec3 {
    let tile = collision_tile(faces);
    let terrain = StreamedTerrain::new(PathBuf::new());
    let walls = |origin, direction, length| {
        tile.first_hit(origin, origin + direction * length)
            .map(|fraction| fraction * length)
    };
    let floor = |feet| tile.floor_height(feet);
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| {
            game_engine_core::step_clearance::step_wall_blocked(
                faces,
                from,
                to,
                top,
                shared::ground::min_walkable_normal_y(),
            )
        },
        game_object_floor: &floor,
    };
    let mut feet = FEET;
    for _ in 0..frames {
        let proposed = feet + HEADING * (speed / fps);
        let moved = ground.validate_move(feet, proposed, snap);
        assert!(
            (moved.x - feet.x).abs() <= (proposed.x - feet.x).abs(),
            "extra horizontal movement: {feet} -> {moved}, proposed {proposed}"
        );
        feet = moved;
    }
    feet
}

fn assert_authored_rate_crosses(speed: f32, fps: f32, frames: usize) {
    let moved = walk_frames(&authored_faces(), true, speed, fps, frames);
    assert!(
        moved.x > 108.32,
        "stopped at {speed} yd/s, {fps} FPS: {moved}"
    );
    assert!(
        (moved.y - UPPER_Y).abs() < 0.00001,
        "not on landing: {moved}"
    );
}

#[test]
fn blocked_horizontal_move_reports_wall_clamp() {
    let terrain = StreamedTerrain::new(PathBuf::new());
    let ground = TerrainGround {
        terrain: &terrain,
        walls: &|_, _, _| Some(0.0),
        step_wall_blocked: &|_, _, _| false,
        game_object_floor: &|_| None,
    };
    let (position, reason) = ground.validate_move_with_reason(Vec3::ZERO, Vec3::X * 0.1, true);
    assert_eq!(position, Vec3::ZERO);
    assert_eq!(reason, Some("wall_clamp"));
}

#[test]
fn ramp_lip_authored_run_144_fps() {
    assert_authored_rate_crosses(7.0, 144.0, 3);
}

#[test]
fn ramp_lip_authored_run_240_fps() {
    assert_authored_rate_crosses(7.0, 240.0, 4);
}

#[test]
fn ramp_lip_authored_walk_60_fps() {
    assert_authored_rate_crosses(2.5, 60.0, 3);
}

#[test]
fn ramp_lip_authored_stockade_advances_onto_upper_floor() {
    let faces = authored_faces();
    let tile = collision_tile(&faces);
    assert!((tile.floor_height(FEET).unwrap() - FEET.y).abs() < 0.00001);
    let moved = walk(&faces, true);
    assert!(moved.x > 108.32, "stopped at authored lip: {moved}");
    assert!(
        (moved.y - UPPER_Y).abs() < 0.00001,
        "not on landing: {moved}"
    );
}

#[test]
fn ramp_lip_wall_taller_than_step_budget_blocks() {
    let mut faces = authored_faces();
    let x = 108.26844;
    let top = FEET.y + STEP_UP_HEIGHT + 1.0;
    faces[0] = [
        Vec3::new(x, top, 2.0),
        Vec3::new(x, top, 5.0),
        Vec3::new(x, FEET.y - 1.0, 2.0),
    ];
    faces.push([
        Vec3::new(x, top, 5.0),
        Vec3::new(x, FEET.y - 1.0, 5.0),
        Vec3::new(x, FEET.y - 1.0, 2.0),
    ]);
    let moved = walk(&faces, true);
    assert!(moved.x <= 108.26844, "crossed tall wall: {moved}");
    assert!(
        (moved.y - FEET.y).abs() < 0.0001,
        "lifted without crossing: {moved}"
    );
}

#[test]
fn ramp_lip_without_walkable_landing_blocks() {
    let mut faces = authored_faces();
    faces.pop();
    let moved = walk(&faces, true);
    assert!(moved.x <= 108.26844, "crossed unsupported lip: {moved}");
}

#[test]
fn ramp_lip_thin_wall_above_budget_but_below_raised_ray_blocks() {
    let mut faces = authored_faces();
    // A 1.5 yd landing does not authorize crossing a 2.0 yd wall in front of it.
    for vertex in &mut faces[2] {
        vertex.y = FEET.y + 1.5;
    }
    let x = 108.26844;
    let bottom = FEET.y - 1.0;
    let top = FEET.y + 2.0;
    faces[0] = [
        Vec3::new(x, top, 2.0),
        Vec3::new(x, top, 5.0),
        Vec3::new(x, bottom, 2.0),
    ];
    faces.push([
        Vec3::new(x, top, 5.0),
        Vec3::new(x, bottom, 5.0),
        Vec3::new(x, bottom, 2.0),
    ]);
    let moved = walk(&faces, true);
    assert!(moved.x <= x, "crossed wall above step budget: {moved}");
}

#[test]
fn ramp_lip_beam_above_existing_wall_ray_preserves_corridor() {
    let mut faces = authored_faces();
    let x = 108.26844;
    let bottom = UPPER_Y + 0.8;
    let top = UPPER_Y + 1.3;
    faces.push([
        Vec3::new(x, bottom, 2.0),
        Vec3::new(x, top, 2.0),
        Vec3::new(x, bottom, 5.0),
    ]);
    faces.push([
        Vec3::new(x, top, 2.0),
        Vec3::new(x, top, 5.0),
        Vec3::new(x, bottom, 5.0),
    ]);
    let moved = walk(&faces, true);
    assert!(
        moved.x > 108.32,
        "new full-body collision narrowed the corridor: {moved}"
    );
}

fn walk_toward_tall_wall(floor_height: impl Fn(f32) -> f32) -> Vec3 {
    let faces = [
        [
            Vec3::new(1.0, -1.0, -2.0),
            Vec3::new(1.0, 5.0, -2.0),
            Vec3::new(1.0, 5.0, 2.0),
        ],
        [
            Vec3::new(1.0, -1.0, -2.0),
            Vec3::new(1.0, 5.0, 2.0),
            Vec3::new(1.0, -1.0, 2.0),
        ],
    ];
    let tile = collision_tile(&faces);
    let terrain = StreamedTerrain::new(PathBuf::new());
    let walls = |origin, direction, length| {
        tile.first_hit(origin, origin + direction * length)
            .map(|fraction| fraction * length)
    };
    let floor = |feet: Vec3| {
        shared::ground::select_ground(feet.y, None, [floor_height(feet.x)])
            .map(|ground| ground.height)
    };
    TerrainGround {
        terrain: &terrain,
        walls: &walls,
        step_wall_blocked: &|from, to, top| {
            game_engine_core::step_clearance::step_wall_blocked(
                &faces,
                from,
                to,
                top,
                shared::ground::min_walkable_normal_y(),
            )
        },
        game_object_floor: &floor,
    }
    .validate_move(Vec3::ZERO, Vec3::X * 3.5, true)
}

#[test]
fn ramp_lip_covered_destination_keeps_reachable_wall_approach() {
    let moved = walk_toward_tall_wall(|x| if x < 1.0 { 0.0 } else { 3.0 });
    assert!(
        (moved.x - 0.95).abs() < 0.00001,
        "lost reachable approach to wall: {moved}"
    );
    assert_eq!(moved.y, 0.0);
}

#[test]
fn ramp_lip_covered_clamped_column_does_not_enter_under_raised_floor() {
    let moved = walk_toward_tall_wall(|x| {
        if x < 0.5 {
            0.0
        } else if x < 1.5 {
            3.0
        } else {
            0.5
        }
    });
    assert_eq!(
        moved,
        Vec3::ZERO,
        "entered unsupported covered column: {moved}"
    );
}

fn overhead_slab(y: f32) -> [Vec3; 3] {
    [
        Vec3::new(107.0, y, 2.0),
        Vec3::new(110.0, y, 2.0),
        Vec3::new(108.5, y, 5.0),
    ]
}

#[test]
fn ramp_lip_ceiling_above_existing_wall_ray_preserves_corridor() {
    let mut faces = authored_faces();
    // Clear above the current body, but not above the stepped-up body.
    faces.push(overhead_slab(FEET.y + 2.2));
    let moved = walk(&faces, true);
    assert!(
        moved.x > 108.32,
        "new head clearance narrowed the corridor: {moved}"
    );
    assert!(
        (moved.y - UPPER_Y).abs() < 0.00001,
        "not on landing: {moved}"
    );
}

#[test]
fn ramp_lip_airborne_does_not_step() {
    let moved = walk(&authored_faces(), false);
    assert!(moved.x <= 108.26844, "airborne step: {moved}");
    assert_eq!(moved.y, FEET.y);
}
