use super::*;
use std::path::Path;
use std::time::{Duration, Instant};

use bevy::ecs::system::SystemState;

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
        blend_mesh: None,
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
                        vertex_uvs: Vec::new(),
                        vertex_depths: Vec::new(),
                    }]
                } else {
                    Vec::new()
                },
                attributes: None,
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
            extents_min: [0.0, 0.0, 0.0],
            extents_max: [0.0, 0.0, 0.0],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: None,
            path: None,
        }],
        chunk_refs: Vec::new(),
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

#[test]
#[ignore = "benchmark-style integration test; run explicitly"]
fn bench_terrain_spawn_headless() {
    const TERRAIN_SPAWN_P99_BUDGET_MS: f64 = 400.0;
    let adt_path = Path::new("data/terrain/azeroth_32_48.adt");
    if !adt_path.exists() {
        println!(
            "Skipping terrain spawn benchmark: missing {}",
            adt_path.display()
        );
        return;
    }
    let iterations = 5_usize;
    let (samples, chunk_entities, terrain_materials, images) =
        measure_headless_terrain_spawn(adt_path, iterations);
    let elapsed: Duration = samples.iter().copied().sum();
    let average = elapsed.div_f64(iterations as f64);
    let p99 = game_engine::test_harness::p99_duration(&samples).expect("benchmark samples");
    println!(
        "terrain_spawn_headless[azeroth_32_48] iterations={iterations} total_ms={:.2} avg_ms={:.2} p99_ms={:.2} chunk_entities={chunk_entities} terrain_materials={terrain_materials} images={images}",
        elapsed.as_secs_f64() * 1000.0,
        average.as_secs_f64() * 1000.0,
        p99.as_secs_f64() * 1000.0,
    );
    assert!(
        chunk_entities > 0,
        "expected spawned terrain chunk entities"
    );
    assert!(terrain_materials > 0, "expected built terrain materials");
    assert!(images > 0, "expected loaded terrain/water images");
    assert!(
        p99.as_secs_f64() * 1000.0 <= TERRAIN_SPAWN_P99_BUDGET_MS,
        "expected terrain spawn p99 <= {TERRAIN_SPAWN_P99_BUDGET_MS:.2}ms, got {:.2}ms",
        p99.as_secs_f64() * 1000.0,
    );
}

fn measure_headless_terrain_spawn(
    adt_path: &Path,
    iterations: usize,
) -> (Vec<Duration>, usize, usize, usize) {
    let mut samples = Vec::with_capacity(iterations);
    let mut final_chunk_entities = 0;
    let mut final_terrain_materials = 0;
    let mut final_images = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let mut app = game_engine::test_harness::headless_app_with(configure_terrain_benchmark_app);
        let root = spawn_headless_terrain_tile(&mut app, adt_path);
        final_chunk_entities = spawned_entity_count(app.world(), root);
        final_terrain_materials = app.world().resource::<Assets<TerrainMaterial>>().len();
        final_images = app.world().resource::<Assets<Image>>().len();
        assert!(
            final_chunk_entities > 0,
            "expected spawned terrain descendants during benchmark"
        );
        samples.push(start.elapsed());
    }
    (
        samples,
        final_chunk_entities,
        final_terrain_materials,
        final_images,
    )
}

fn configure_terrain_benchmark_app(app: &mut App) {
    app.add_plugins(bevy::transform::TransformPlugin);
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<TerrainMaterial>::default());
    app.insert_resource(Assets::<WaterMaterial>::default());
    app.insert_resource(Assets::<Image>::default());
}

fn spawn_headless_terrain_tile(app: &mut App, adt_path: &Path) -> Entity {
    let world = app.world_mut();
    let mut state: SystemState<(
        Commands,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<TerrainMaterial>>,
        ResMut<Assets<WaterMaterial>>,
        ResMut<Assets<Image>>,
    )> = SystemState::new(world);
    let (mut commands, mut meshes, mut terrain_materials, mut water_materials, mut images) =
        state.get_mut(world);
    let mut assets = TerrainOnlySpawnAssets {
        commands: &mut commands,
        meshes: &mut meshes,
        terrain_materials: &mut terrain_materials,
        water_materials: &mut water_materials,
        images: &mut images,
    };
    let mut heightmap = TerrainHeightmap::default();
    let spawned = spawn_adt_terrain_only(&mut assets, &mut heightmap, adt_path)
        .expect("spawn benchmark terrain tile");
    state.apply(world);
    app.update();
    spawned.root_entity
}

fn spawned_entity_count(world: &World, root: Entity) -> usize {
    let mut count = 1;
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            count += children.len();
            stack.extend(children.iter());
        }
    }
    count
}

#[test]
fn terrain_shader_uses_wow_mccv_diffuse_scaling() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader.contains("let vertex_color = in.color.rgb * 2.0;"),
        "expected terrain shader to apply WoW MCCV diffuse scaling"
    );
}

#[test]
fn terrain_shader_samples_layers_with_animation_offsets() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader.contains("let uv0 = animated_layer_uv(0u, uv);"),
        "expected terrain shader to build animated UVs for terrain layer 0"
    );
    assert!(
        shader.contains("let c0 = apply_layer_overbright(0u, sample_ground_tiled(0u, uv0));"),
        "expected terrain shader to sample terrain layer 0 with animated UVs before applying overbright"
    );
}

#[test]
fn terrain_shader_uses_height_maps_for_layer_blending() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader.contains("let h0 = sample_height_tiled(0u, uv0).a;"),
        "expected terrain shader to sample layer 0 height from the MHID height texture"
    );
    assert!(
        shader.contains("paint.x * height_weight(h0, layer_params(0u), blend_strength)"),
        "expected terrain shader to weight layer 0 using sampled height texture data"
    );
}

#[test]
fn terrain_shader_applies_overbright_layer_multiplier() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader.contains("let c0 = apply_layer_overbright(0u, sample_ground_tiled(0u, uv0));"),
        "expected terrain shader to apply per-layer overbright before terrain blending"
    );
}

#[test]
fn terrain_shader_applies_shadow_map_overlay() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader.contains("let static_shadow = textureSample(shadow_map, shadow_sampler, uv).r;"),
        "expected terrain shader to sample the packed MCSH shadow map"
    );
    assert!(
        shader
            .contains("let shadow_light = mix(STATIC_SHADOW_MIN_BRIGHTNESS, 1.0, static_shadow);"),
        "expected terrain shader to turn MCSH into a darkening factor instead of a hard binary mask"
    );
    assert!(
        shader.contains(
            "let shaded_color = vec4<f32>(color.rgb * vertex_color * shadow_light, color.a);"
        ),
        "expected terrain shader to multiply terrain lighting by the static shadow factor"
    );
}

#[test]
fn terrain_shader_samples_environment_map_for_reflective_layers() {
    let shader = std::fs::read_to_string("assets/shaders/terrain.wgsl")
        .expect("terrain shader should be readable");

    assert!(
        shader
            .contains("let reflection = sample_environment_reflection(pbr_input.N, pbr_input.V);"),
        "expected terrain shader to sample the sky cubemap for reflective terrain layers"
    );
    assert!(
        shader.contains("let reflective_weight = dot(weights, reflection_mask);"),
        "expected terrain shader to derive reflection strength from per-layer cubemap flags"
    );
}
