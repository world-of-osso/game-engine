use super::*;

fn flat_grid(
    index_x: u32,
    index_y: u32,
    origin_x: f32,
    origin_z: f32,
    height: f32,
) -> adt::ChunkHeightGrid {
    adt::ChunkHeightGrid {
        index_x,
        index_y,
        origin_x,
        origin_z,
        base_y: height,
        heights: [0.0; 145],
    }
}

fn rough_grid(
    index_x: u32,
    index_y: u32,
    origin_x: f32,
    origin_z: f32,
    base_y: f32,
    peak_delta: f32,
) -> adt::ChunkHeightGrid {
    let mut heights = [0.0; 145];
    heights[adt::vertex_index(8, 4)] = peak_delta;
    heights[adt::vertex_index(8, 5)] = peak_delta;
    heights[adt::vertex_index(9, 4)] = peak_delta;
    heights[adt::vertex_index(10, 4)] = peak_delta;
    heights[adt::vertex_index(10, 5)] = peak_delta;
    adt::ChunkHeightGrid {
        index_x,
        index_y,
        origin_x,
        origin_z,
        base_y,
        heights,
    }
}

fn empty_adt(height_grids: Vec<adt::ChunkHeightGrid>) -> adt::AdtData {
    adt::AdtData {
        chunks: Vec::new(),
        height_grids,
        center_surface: [0.0, 0.0, 0.0],
        chunk_positions: Vec::new(),
        water: None,
        water_error: None,
    }
}

#[test]
fn choose_safe_spawn_position_prefers_flat_chunk_over_rough_center() {
    let adt = empty_adt(vec![
        rough_grid(8, 8, 0.0, 0.0, 40.0, 24.0),
        flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
    ]);

    let spawn =
        terrain_spawn_position::choose_safe_spawn_position(&adt, None).expect("spawn position");

    assert!(
        spawn.z < 0.0,
        "expected flatter chunk north of center to win"
    );
    assert!((spawn.y - 12.0).abs() < 0.01, "expected flat chunk height");
}

#[test]
fn choose_safe_spawn_position_skips_water_chunks() {
    let mut adt = empty_adt(vec![
        flat_grid(8, 8, 0.0, 0.0, 8.0),
        flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
    ]);
    adt.water = Some(crate::asset::adt::AdtWaterData {
        chunks: (0..256)
            .map(|i| crate::asset::adt::ChunkWater {
                layers: if i == 0 {
                    vec![crate::asset::adt::WaterLayer {
                        liquid_type: 0,
                        liquid_object: 0,
                        min_height: 0.0,
                        max_height: 0.0,
                        x_offset: 0,
                        y_offset: 0,
                        width: 8,
                        height: 8,
                        exists: [0; 8],
                        vertex_heights: Vec::new(),
                    }]
                } else {
                    Vec::new()
                },
            })
            .collect(),
    });

    let spawn =
        terrain_spawn_position::choose_safe_spawn_position(&adt, None).expect("spawn position");

    assert!(spawn.z < 0.0, "expected non-water chunk to win");
    assert!((spawn.y - 12.0).abs() < 0.01, "expected dry chunk height");
}

#[test]
fn choose_safe_spawn_position_avoids_nearby_wmo_chunk() {
    let adt = empty_adt(vec![
        flat_grid(8, 8, 0.0, 0.0, 8.0),
        flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
    ]);
    let obj_data = adt_obj::AdtObjData {
        doodads: Vec::new(),
        wmos: vec![adt_obj::WmoPlacement {
            name_id: 0,
            unique_id: 0,
            position: [-adt::CHUNK_SIZE / 2.0, -adt::CHUNK_SIZE / 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: None,
            path: None,
        }],
    };

    let spawn = terrain_spawn_position::choose_safe_spawn_position(&adt, Some(&obj_data))
        .expect("spawn position");

    assert!(
        spawn.z < 0.0,
        "expected spawn to move away from occupied center chunk"
    );
    assert!(
        (spawn.y - 12.0).abs() < 0.01,
        "expected alternate flat chunk height"
    );
}

#[test]
fn bootstrap_terrain_streaming_uses_local_player_tile_when_server_did_not_seed_it() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<AdtManager>();
    app.world_mut().spawn((
        crate::camera::Player,
        Transform::from_xyz(-8912.9, 80.2, 207.8),
    ));

    let _ = app.world_mut().run_system_once(bootstrap_terrain_streaming);

    let adt_manager = app.world().resource::<AdtManager>();
    assert_eq!(adt_manager.map_name, "azeroth");
    assert_eq!(adt_manager.initial_tile, (32, 48));
    assert_eq!(adt_manager.server_requested.len(), 1);
    assert!(adt_manager.server_requested.contains(&(32, 48)));
}
