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
    let tile = collision_tile(faces);
    let terrain = StreamedTerrain::new(PathBuf::new());
    let walls = |origin, direction, length| {
        tile.first_hit(origin, origin + direction * length)
            .map(|fraction| fraction * length)
    };
    let floor = |feet| tile.floor_height(feet);
    TerrainGround {
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
    }
    .validate_move(FEET, FEET + HEADING * (7.0 / 60.0), snap)
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
