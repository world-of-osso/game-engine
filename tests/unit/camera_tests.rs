use super::*;
use crate::asset::adt;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_tile::bevy_to_tile_coords;

const TEST_WATER_STEP: f32 = adt::CHUNK_SIZE / 8.0;

fn flat_grid(origin_x: f32, origin_z: f32, height: f32) -> adt::ChunkHeightGrid {
    adt::ChunkHeightGrid {
        index_x: 0,
        index_y: 0,
        origin_x,
        origin_z,
        base_y: height,
        heights: [0.0; 145],
    }
}

fn swim_sample_tile() -> (u32, u32) {
    let sample_x = -TEST_WATER_STEP * 0.25;
    let sample_z = TEST_WATER_STEP * 0.25;
    bevy_to_tile_coords(sample_x, sample_z)
}

fn flat_water_layer(height: f32) -> adt::WaterLayer {
    adt::WaterLayer {
        liquid_type: 0,
        liquid_object: 0,
        min_height: height,
        max_height: height,
        x_offset: 0,
        y_offset: 0,
        width: 1,
        height: 1,
        exists: [1, 0, 0, 0, 0, 0, 0, 0],
        vertex_heights: vec![height; 4],
        vertex_uvs: Vec::new(),
        vertex_depths: Vec::new(),
    }
}

fn chunk_water(index: usize, water_height: f32) -> adt::ChunkWater {
    let layers = if index == 0 {
        vec![flat_water_layer(water_height)]
    } else {
        Vec::new()
    };
    adt::ChunkWater {
        layers,
        attributes: None,
    }
}

fn swim_adt(ground_height: f32, water_height: f32) -> adt::AdtData {
    adt::AdtData {
        chunks: Vec::new(),
        blend_mesh: None,
        flight_bounds: None,
        height_grids: vec![flat_grid(0.0, 0.0, ground_height)],
        center_surface: [0.0, 0.0, 0.0],
        chunk_positions: vec![[0.0, 0.0, ground_height]; 256],
        water: Some(adt::AdtWaterData {
            chunks: (0..256)
                .map(|index| chunk_water(index, water_height))
                .collect(),
        }),
        water_error: None,
    }
}

fn swim_heightmap(ground_height: f32, water_height: f32) -> TerrainHeightmap {
    let (tile_y, tile_x) = swim_sample_tile();
    let mut heightmap = TerrainHeightmap::default();
    let adt = swim_adt(ground_height, water_height);
    heightmap.register_tile(tile_y, tile_x, &adt, None);
    heightmap
}

fn jump_heightmap() -> TerrainHeightmap {
    let data = std::fs::read("data/terrain/azeroth_32_48.adt")
        .expect("expected test ADT data/terrain/azeroth_32_48.adt");
    let adt = crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);
    heightmap
}

fn jump_fixture(heightmap: &TerrainHeightmap) -> (Transform, MovementState, CharacterPhysics) {
    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let ground_y = heightmap
        .height_at(bx, bz)
        .expect("expected terrain at sample position");
    let transform = Transform::from_xyz(bx, ground_y + 0.2, bz);
    let movement = MovementState {
        direction: MoveDirection::None,
        running: true,
        jumping: true,
        autorun: false,
        swimming: false,
    };
    let physics = CharacterPhysics {
        vertical_velocity: -1.0,
        grounded: true,
    };
    (transform, movement, physics)
}

#[test]
fn spawn_wow_camera_adds_spatial_listener() {
    let mut world = World::new();
    let entity = spawn_wow_camera(&mut world.commands());
    world.flush();
    let listener = world
        .get::<SpatialListener>(entity)
        .expect("camera should have spatial listener");
    assert!(listener.right_ear_offset.x > listener.left_ear_offset.x);
}

#[test]
fn test_zoom_interpolation() {
    let mut distance: f32 = 15.0;
    let target_distance: f32 = 5.0;
    let zoom_speed: f32 = 8.0;
    let dt: f32 = 0.016;

    for _ in 0..10 {
        let t = (zoom_speed * dt).min(1.0);
        distance = distance.lerp(target_distance, t);
    }

    assert!(distance < 15.0, "distance should decrease toward target");
    assert!(distance > 5.0, "should not reach target in 10 frames");
    assert!(
        distance < 10.0,
        "expected significant progress, got {}",
        distance
    );
}

#[test]
fn jump_state_stays_active_until_player_reaches_ground() {
    let heightmap = jump_heightmap();
    let (mut transform, mut movement, mut physics) = jump_fixture(&heightmap);
    let keys = ButtonInput::<KeyCode>::default();
    let mouse_buttons = ButtonInput::<MouseButton>::default();
    let bindings = InputBindings::default();

    apply_horizontal_movement(HorizontalMovementContext {
        transform: &mut transform,
        movement: &mut movement,
        physics: &mut physics,
        keys: &keys,
        mouse_buttons: &mouse_buttons,
        bindings: &bindings,
        terrain: Some(&heightmap),
        proposed: None,
    });

    assert!(
        movement.jumping,
        "jumping should stay active until the player actually reaches the ground"
    );
}

#[test]
fn proposed_ground_movement_is_absent_without_input() {
    assert_eq!(
        build_proposed_ground_movement(Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO, 7.0, 0.5),
        None
    );
}

#[test]
fn proposed_ground_movement_advances_in_normalized_input_direction() {
    let proposed = build_proposed_ground_movement(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(3.0, 0.0, 4.0),
        10.0,
        0.5,
    )
    .expect("movement proposal");

    assert!((proposed.x - 4.0).abs() < 0.001);
    assert_eq!(proposed.y, 2.0);
    assert!((proposed.z - 7.0).abs() < 0.001);
}

#[test]
fn autorun_toggle_sets_forward_animation_without_forward_key() {
    let keys = ButtonInput::<KeyCode>::default();
    let mouse_buttons = ButtonInput::<MouseButton>::default();
    let bindings = InputBindings::default();

    let (direction, anim_dir) = compute_movement_input(
        &keys,
        &mouse_buttons,
        &bindings,
        true,
        false,
        &CharacterFacing { yaw: 0.0 },
    );

    assert_eq!(anim_dir, MoveDirection::Forward);
    assert_eq!(direction, Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn waypoint_pathing_sets_forward_animation_without_forward_key() {
    let keys = ButtonInput::<KeyCode>::default();
    let mouse_buttons = ButtonInput::<MouseButton>::default();
    let bindings = InputBindings::default();

    let (direction, anim_dir) = compute_movement_input(
        &keys,
        &mouse_buttons,
        &bindings,
        false,
        true,
        &CharacterFacing { yaw: 0.0 },
    );

    assert_eq!(anim_dir, MoveDirection::Forward);
    assert_eq!(direction, Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn backward_input_cancels_autorun_toggle() {
    let mut keys = ButtonInput::<KeyCode>::default();
    let mouse_buttons = ButtonInput::<MouseButton>::default();
    let bindings = InputBindings::default();
    let mut movement = MovementState {
        autorun: true,
        ..Default::default()
    };

    keys.press(KeyCode::KeyS);
    sync_player_movement_toggles(&keys, &mouse_buttons, &bindings, &mut movement);

    assert!(!movement.autorun);
}

#[test]
fn forward_input_counts_as_manual_override_for_pathing() {
    let mut keys = ButtonInput::<KeyCode>::default();
    let mouse_buttons = ButtonInput::<MouseButton>::default();
    let bindings = InputBindings::default();

    keys.press(KeyCode::KeyW);

    assert!(has_manual_movement_override(
        &keys,
        &mouse_buttons,
        &bindings,
        None,
    ));
}

#[test]
fn modal_close_clears_autorun() {
    let mut movement = MovementState {
        direction: MoveDirection::Forward,
        autorun: true,
        ..Default::default()
    };

    let modal = crate::scenes::game_menu::UiModalOpen;
    let closed = close_player_movement_for_modal(Some(&modal), &mut movement);

    assert!(closed);
    assert!(!movement.autorun);
    assert_eq!(movement.direction, MoveDirection::None);
}

#[test]
fn backward_speed_uses_shared_backpedal_multiplier() {
    let speed = movement_speed_multiplier(MoveDirection::Backward) * RUN_SPEED;
    let expected = RUN_SPEED * shared::movement::BACKPEDAL_MULTIPLIER;
    assert!((speed - expected).abs() < f32::EPSILON);
}

#[test]
fn strafe_speed_uses_shared_strafe_multiplier() {
    let speed = movement_speed_multiplier(MoveDirection::Left) * RUN_SPEED;
    let expected = RUN_SPEED * shared::movement::STRAFE_MULTIPLIER;
    assert!((speed - expected).abs() < f32::EPSILON);
}

#[test]
fn strafe_speed_remains_faster_than_backpedal() {
    let strafe = movement_speed_multiplier(MoveDirection::Right) * RUN_SPEED;
    let backpedal = movement_speed_multiplier(MoveDirection::Backward) * RUN_SPEED;
    assert!(strafe < RUN_SPEED);
    assert!(strafe > backpedal);
}

#[test]
fn deep_water_sets_swimming_state() {
    let heightmap = swim_heightmap(0.0, 2.0);
    let position = Vec3::new(-TEST_WATER_STEP * 0.25, 0.0, TEST_WATER_STEP * 0.25);
    assert!(is_swimming(position, &heightmap));
}

#[test]
fn shallow_water_does_not_set_swimming_state() {
    let heightmap = swim_heightmap(0.0, 0.6);
    let position = Vec3::new(-TEST_WATER_STEP * 0.25, 0.0, TEST_WATER_STEP * 0.25);
    assert!(!is_swimming(position, &heightmap));
}
