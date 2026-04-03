use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::{App, Entity, GlobalTransform, Vec3};
use bevy_hanabi::{AlphaMode, Attribute, ExprWriter, SimulationSpace};

use super::visuals::has_authored_size_variation;
use super::{
    FlipbookSpriteMode, PARTICLE_FLAG_BONE_SCALE, PARTICLE_FLAG_CLAMP_TAIL_TO_AGE,
    PARTICLE_FLAG_NEGATE_SPIN, PARTICLE_FLAG_RANDOM_TEXTURE, PARTICLE_FLAG_SIZE_VARIATION_2D,
    PARTICLE_FLAG_SPHERE_INVERT, PARTICLE_FLAG_TAIL_PARTICLES, PARTICLE_FLAG_VELOCITY_ORIENT,
    PARTICLE_FLAG_WIND_DYNAMIC, PARTICLE_FLAG_WIND_ENABLED, PARTICLE_FLAG_WORLD_SPACE,
    PARTICLE_FLAG_XY_QUAD, ParticleSpawnMode, PositionInitModifier, active_cell_track,
    build_color_gradient, build_effect_asset, build_effect_asset_with_mode, build_expr_modifiers,
    build_position_modifier, build_size_gradient, emitter_alpha_mode, emitter_parent_entity,
    emitter_scale_source, emitter_simulation_space, emitter_spawn_offset, emitter_spawn_radius,
    emitter_translation, emitter_uses_bone_scale, emitter_uses_follow_position,
    emitter_uses_project_particle, emitter_uses_sphere_invert_velocity, flipbook_sprite_mode,
    gravity_accel_bevy, has_authored_spin, has_authored_twinkle, has_authored_wind, lifetime_range,
    orient_mode, projected_particle_spawn_y, scaled_emission_rate, wind_accel_bevy,
    wind_strength_at_age,
};
use crate::asset::m2_anim::M2Bone;
use crate::asset::m2_particle::M2ParticleEmitter;
use crate::client_options::GraphicsOptions;
use crate::terrain_heightmap::TerrainHeightmap;
use bevy_hanabi::OrientMode;

struct SampleMotionDefaults {
    emission_speed: f32,
    speed_variation: f32,
    vertical_range: f32,
    horizontal_range: f32,
    gravity: f32,
    gravity_vector: [f32; 3],
    lifespan: f32,
    lifespan_variation: f32,
    emission_rate: f32,
    emission_rate_variation: f32,
    area_length: f32,
    area_width: f32,
    z_source: f32,
    tail_length: f32,
    drag: f32,
    base_spin: f32,
    base_spin_variation: f32,
    spin: f32,
    spin_variation: f32,
    wind_vector: [f32; 3],
    wind_time: f32,
    follow_speed1: f32,
    follow_scale1: f32,
    follow_speed2: f32,
    follow_scale2: f32,
}

struct SampleVisualDefaults {
    colors: [[f32; 3]; 3],
    color_keys: Vec<(f32, [f32; 3])>,
    opacity: [f32; 3],
    opacity_keys: Vec<(f32, f32)>,
    scales: [[f32; 2]; 3],
    scale_keys: Vec<(f32, [f32; 2])>,
    twinkle_speed: f32,
    twinkle_percent: f32,
    twinkle_scale_min: f32,
    twinkle_scale_max: f32,
    head_cell_track: [u16; 3],
    tail_cell_track: [u16; 3],
    burst_multiplier: f32,
    mid_point: f32,
}

fn sample_motion_defaults() -> SampleMotionDefaults {
    SampleMotionDefaults {
        emission_speed: 1.0,
        speed_variation: 0.0,
        vertical_range: 0.1,
        horizontal_range: std::f32::consts::TAU,
        gravity: 0.0,
        gravity_vector: [0.0, 0.0, 0.0],
        lifespan: 1.0,
        lifespan_variation: 0.0,
        emission_rate: 20.0,
        emission_rate_variation: 0.0,
        area_length: 0.1,
        area_width: 0.1,
        z_source: 0.0,
        tail_length: 1.0,
        drag: 0.0,
        base_spin: 0.0,
        base_spin_variation: 0.0,
        spin: 0.0,
        spin_variation: 0.0,
        wind_vector: [0.0, 0.0, 0.0],
        wind_time: 0.0,
        follow_speed1: 0.0,
        follow_scale1: 0.0,
        follow_speed2: 0.0,
        follow_scale2: 0.0,
    }
}

fn sample_visual_defaults() -> SampleVisualDefaults {
    SampleVisualDefaults {
        colors: [[255.0, 128.0, 64.0]; 3],
        color_keys: Vec::new(),
        opacity: [1.0, 1.0, 0.0],
        opacity_keys: Vec::new(),
        scales: [[0.1, 0.1], [0.2, 0.2], [0.05, 0.05]],
        scale_keys: Vec::new(),
        twinkle_speed: 0.0,
        twinkle_percent: 0.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        head_cell_track: [0, 0, 0],
        tail_cell_track: [0, 0, 0],
        burst_multiplier: 1.0,
        mid_point: 0.5,
    }
}

fn sample_emitter() -> M2ParticleEmitter {
    let motion = sample_motion_defaults();
    let visuals = sample_visual_defaults();
    let mut emitter = M2ParticleEmitter {
        blend_type: 4,
        emitter_type: 1,
        tile_rows: 4,
        tile_cols: 4,
        ..M2ParticleEmitter::default()
    };
    apply_sample_motion_defaults(&mut emitter, motion);
    apply_sample_visual_defaults(&mut emitter, visuals);
    emitter
}

fn apply_sample_motion_defaults(emitter: &mut M2ParticleEmitter, motion: SampleMotionDefaults) {
    emitter.emission_speed = motion.emission_speed;
    emitter.speed_variation = motion.speed_variation;
    emitter.vertical_range = motion.vertical_range;
    emitter.horizontal_range = motion.horizontal_range;
    emitter.gravity = motion.gravity;
    emitter.gravity_vector = motion.gravity_vector;
    emitter.lifespan = motion.lifespan;
    emitter.lifespan_variation = motion.lifespan_variation;
    emitter.emission_rate = motion.emission_rate;
    emitter.emission_rate_variation = motion.emission_rate_variation;
    emitter.area_length = motion.area_length;
    emitter.area_width = motion.area_width;
    emitter.z_source = motion.z_source;
    emitter.tail_length = motion.tail_length;
    emitter.drag = motion.drag;
    emitter.base_spin = motion.base_spin;
    emitter.base_spin_variation = motion.base_spin_variation;
    emitter.spin = motion.spin;
    emitter.spin_variation = motion.spin_variation;
    emitter.wind_vector = motion.wind_vector;
    emitter.wind_time = motion.wind_time;
    emitter.follow_speed1 = motion.follow_speed1;
    emitter.follow_scale1 = motion.follow_scale1;
    emitter.follow_speed2 = motion.follow_speed2;
    emitter.follow_scale2 = motion.follow_scale2;
}

fn apply_sample_visual_defaults(emitter: &mut M2ParticleEmitter, visuals: SampleVisualDefaults) {
    emitter.colors = visuals.colors;
    emitter.color_keys = visuals.color_keys;
    emitter.opacity = visuals.opacity;
    emitter.opacity_keys = visuals.opacity_keys;
    emitter.scales = visuals.scales;
    emitter.scale_keys = visuals.scale_keys;
    emitter.twinkle_speed = visuals.twinkle_speed;
    emitter.twinkle_percent = visuals.twinkle_percent;
    emitter.twinkle_scale_min = visuals.twinkle_scale_min;
    emitter.twinkle_scale_max = visuals.twinkle_scale_max;
    emitter.head_cell_track = visuals.head_cell_track;
    emitter.tail_cell_track = visuals.tail_cell_track;
    emitter.burst_multiplier = visuals.burst_multiplier;
    emitter.mid_point = visuals.mid_point;
}

fn sample_cell_track_frame(
    track: [u16; 3],
    mid_point: f32,
    age_ratio: f32,
    total_cells: u32,
) -> u32 {
    let mid = mid_point.clamp(0.01, 0.99);
    let t = age_ratio.clamp(0.0, 1.0);
    let frame = if t < mid {
        let segment_t = (t / mid).clamp(0.0, 1.0);
        (track[0] as f32) + ((track[1] as f32) - (track[0] as f32)) * segment_t
    } else {
        let segment_t = ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0);
        (track[1] as f32) + ((track[2] as f32) - (track[1] as f32)) * segment_t
    };
    frame
        .floor()
        .clamp(0.0, total_cells.saturating_sub(1) as f32) as u32
}

#[test]
fn textured_emitters_declare_hanabi_texture_slot() {
    let mut emitter = sample_emitter();
    emitter.texture_fdid = Some(145513);

    let asset = build_effect_asset(&emitter, 1.0, 1.0);

    assert_eq!(asset.texture_layout().layout.len(), 1);
    assert_eq!(asset.texture_layout().layout[0].name, "color");
}

#[test]
fn burst_once_spawn_mode_builds_once_spawner() {
    let emitter = sample_emitter();

    let asset = build_effect_asset_with_mode(&emitter, 1.0, 1.0, ParticleSpawnMode::BurstOnce);

    assert!(asset.spawner.is_once());
    assert!(asset.spawner.starts_active());
    assert_eq!(asset.spawner.cycle_count(), 1);
}

#[test]
fn untextured_emitters_do_not_declare_hanabi_texture_slot() {
    let emitter = sample_emitter();
    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(modifiers.module.texture_layout().layout.is_empty());
}

#[test]
fn emitter_translation_uses_raw_model_position() {
    let mut emitter = sample_emitter();
    emitter.position = [1.0, 2.0, 3.0];

    let translation = emitter_translation(&emitter);

    assert_eq!(translation, Vec3::new(1.0, 3.0, -2.0));
}

#[test]
fn sphere_emitters_use_max_area_as_spawn_radius() {
    let mut emitter = sample_emitter();
    emitter.emitter_type = 2;
    emitter.area_length = 0.4;
    emitter.area_width = 0.2;

    assert_eq!(emitter_spawn_radius(&emitter), 0.4);
}

#[test]
fn plane_emitters_use_attribute_position_modifier() {
    let mut emitter = sample_emitter();
    let writer = ExprWriter::new();
    emitter.area_length = 0.4;
    emitter.area_width = 0.2;

    let position = build_position_modifier(&emitter, &writer, 1.0);

    assert!(matches!(position, PositionInitModifier::Attribute(_)));
}

#[test]
fn z_source_emitters_still_use_position_attribute_spawn() {
    let mut emitter = sample_emitter();
    emitter.z_source = 2.5;
    let writer = ExprWriter::new();

    let position = build_position_modifier(&emitter, &writer, 1.0);
    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(matches!(position, PositionInitModifier::Attribute(_)));
    assert!(modifiers.init.vel.attribute == Attribute::VELOCITY);
}

#[test]
fn point_emitters_do_not_expand_spawn_radius() {
    let mut emitter = sample_emitter();
    emitter.emitter_type = 0;
    emitter.area_length = 0.4;
    emitter.area_width = 0.2;

    assert_eq!(emitter_spawn_radius(&emitter), 0.0);
}

#[test]
fn world_space_emitters_skip_bone_parent_transform() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_WORLD_SPACE;
    emitter.position = [1.0, 2.0, 3.0];
    emitter.bone_index = 0;
    let bones = vec![M2Bone {
        key_bone_id: 0,
        flags: 0,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot: [4.0, 5.0, 6.0],
    }];

    let offset = emitter_spawn_offset(&emitter, &bones);

    assert_eq!(offset, emitter_translation(&emitter));
}

#[test]
fn world_space_emitters_use_model_parent_even_with_bone_entity() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_WORLD_SPACE;

    let parent = Entity::from_bits(11);
    let bone = Some(Entity::from_bits(22));

    assert_eq!(emitter_parent_entity(&emitter, bone, parent), parent);
}

#[test]
fn project_particle_is_disabled_for_world_space_emitters() {
    let mut emitter = sample_emitter();
    emitter.flags = super::PARTICLE_FLAG_PROJECT_PARTICLE;
    assert!(emitter_uses_project_particle(&emitter));

    emitter.flags |= PARTICLE_FLAG_WORLD_SPACE;
    assert!(!emitter_uses_project_particle(&emitter));
}

#[test]
fn project_particle_snaps_spawn_height_to_loaded_terrain() {
    let data =
        std::fs::read("data/terrain/azeroth_32_48.adt").expect("expected test ADT terrain tile");
    let adt = crate::asset::adt::load_adt(&data).expect("expected ADT to parse");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);

    let [bx, _by, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let terrain_y = heightmap
        .height_at(bx, bz)
        .expect("expected terrain height at reference point");

    let mut app = App::new();
    let entity = app
        .world_mut()
        .spawn(GlobalTransform::from_translation(Vec3::new(
            bx,
            terrain_y - 8.0,
            bz,
        )))
        .id();
    let mut emitter = sample_emitter();
    emitter.flags = super::PARTICLE_FLAG_PROJECT_PARTICLE;
    let comp = super::ParticleEmitterComp {
        emitter,
        bone_entity: None,
        scale_source: entity,
        spawn_mode: ParticleSpawnMode::Continuous,
        pending_texture: None,
    };

    let delta = app
        .world_mut()
        .run_system_once(move |query: bevy::prelude::Query<&GlobalTransform>| {
            projected_particle_spawn_y(entity, &comp, &query, Some(&heightmap))
                .expect("projected particle should snap to terrain")
        })
        .expect("system should run");

    assert!(
        (delta - 8.0).abs() < 0.01,
        "delta={delta} terrain_y={terrain_y}"
    );
}

#[test]
fn bone_scale_uses_bone_entity_only_without_world_space() {
    let mut emitter = sample_emitter();
    let parent = Entity::from_bits(11);
    let bone = Some(Entity::from_bits(22));

    emitter.flags = PARTICLE_FLAG_BONE_SCALE;
    assert!(emitter_uses_bone_scale(&emitter));
    assert_eq!(emitter_scale_source(&emitter, bone, parent), bone.unwrap());

    emitter.flags = PARTICLE_FLAG_WORLD_SPACE | PARTICLE_FLAG_BONE_SCALE;
    assert!(!emitter_uses_bone_scale(&emitter));
    assert_eq!(emitter_scale_source(&emitter, bone, parent), parent);
}

#[test]
fn follow_position_emitters_use_local_simulation_space() {
    let mut emitter = sample_emitter();
    emitter.flags = super::PARTICLE_FLAG_FOLLOW_POSITION;

    assert!(emitter_uses_follow_position(&emitter));
    assert_eq!(emitter_simulation_space(&emitter), SimulationSpace::Local);

    let effect = build_effect_asset(&emitter, 1.0, 1.0);
    assert_eq!(effect.simulation_space, SimulationSpace::Local);
}

#[test]
fn default_emitters_use_global_simulation_space() {
    let emitter = sample_emitter();

    assert!(!emitter_uses_follow_position(&emitter));
    assert_eq!(emitter_simulation_space(&emitter), SimulationSpace::Global);
}

#[test]
fn sphere_invert_flag_switches_sphere_emitters_to_implosion_velocity() {
    let mut emitter = sample_emitter();
    emitter.emitter_type = 2;
    emitter.flags = PARTICLE_FLAG_SPHERE_INVERT;

    assert!(emitter_uses_sphere_invert_velocity(&emitter));

    emitter.emitter_type = 1;
    assert!(!emitter_uses_sphere_invert_velocity(&emitter));
}

#[test]
fn burst_multiplier_scales_particle_size_gradient() {
    let mut emitter = sample_emitter();
    emitter.burst_multiplier = 2.5;

    let gradient = build_size_gradient(&emitter, 1.0);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0].value, Vec3::new(0.5, 0.5, 1.0));
    assert_eq!(keys[1].value, Vec3::new(1.0, 1.0, 1.0));
    assert_eq!(keys[2].value, Vec3::new(0.25, 0.25, 1.0));
}

#[test]
fn particle_density_scales_emission_rate() {
    let mut emitter = sample_emitter();
    emitter.emission_rate_variation = 4.0;
    let graphics = GraphicsOptions {
        particle_density: 50,
        render_scale: 1.0,
        ..GraphicsOptions::default()
    };

    assert!(
        (scaled_emission_rate(&emitter, graphics.particle_density_multiplier()) - 11.0).abs()
            < 0.0001
    );
}

#[test]
fn particle_density_defaults_to_full_rate() {
    let mut emitter = sample_emitter();
    emitter.emission_rate_variation = 4.0;

    assert!(
        (scaled_emission_rate(
            &emitter,
            GraphicsOptions::default().particle_density_multiplier()
        ) - 22.0)
            .abs()
            < 0.0001
    );
}

#[test]
fn size_gradient_uses_full_scale_key_timeline_when_present() {
    let mut emitter = sample_emitter();
    emitter.scale_keys = vec![
        (0.0, [0.1, 0.2]),
        (0.25, [0.3, 0.4]),
        (0.75, [0.5, 0.6]),
        (1.0, [0.7, 0.8]),
    ];
    emitter.burst_multiplier = 2.0;

    let gradient = build_size_gradient(&emitter, 1.5);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0].ratio(), 0.0);
    assert_eq!(keys[0].value, Vec3::new(0.6, 1.2, 1.0));
    assert!((keys[1].ratio() - 0.25).abs() < 0.0001);
    assert!((keys[1].value.x - 1.8).abs() < 0.0001);
    assert!((keys[1].value.y - 2.4).abs() < 0.0001);
    assert_eq!(keys[1].value.z, 1.0);
    assert!((keys[2].ratio() - 0.75).abs() < 0.0001);
    assert!((keys[2].value.x - 3.0).abs() < 0.0001);
    assert!((keys[2].value.y - 3.6).abs() < 0.0001);
    assert_eq!(keys[2].value.z, 1.0);
    assert_eq!(keys[3].ratio(), 1.0);
    assert_eq!(keys[3].value, Vec3::new(4.2, 4.8, 1.0));
}

#[test]
fn trail_particles_stretch_length_over_lifetime() {
    let mut emitter = sample_emitter();
    emitter.particle_type = 1;
    emitter.emission_speed = 3.0;
    emitter.lifespan = 2.0;

    let gradient = build_size_gradient(&emitter, 1.0);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 3);
    assert!((keys[0].value.x - 0.2).abs() < 0.0001);
    assert!((keys[1].value.x - 2.2).abs() < 0.0001);
    assert!((keys[2].value.x - 3.7).abs() < 0.0001);
    assert!((keys[2].value.y - 0.1).abs() < 0.0001);
}

#[test]
fn tail_particle_flag_uses_authored_tail_length_multiplier() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_TAIL_PARTICLES;
    emitter.emission_speed = 3.0;
    emitter.tail_length = 2.0;

    let gradient = build_size_gradient(&emitter, 1.0);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 3);
    assert!((keys[0].value.x - 6.2).abs() < 0.0001);
    assert!((keys[1].value.x - 6.4).abs() < 0.0001);
    assert!((keys[2].value.x - 6.1).abs() < 0.0001);
    assert!((keys[2].value.y - 0.1).abs() < 0.0001);
}

#[test]
fn clamp_tail_to_age_limits_tail_growth_until_tail_length() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_TAIL_PARTICLES | PARTICLE_FLAG_CLAMP_TAIL_TO_AGE;
    emitter.emission_speed = 3.0;
    emitter.tail_length = 2.0;
    emitter.lifespan = 4.0;

    let gradient = build_size_gradient(&emitter, 1.0);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 3);
    assert!((keys[0].value.x - 0.2).abs() < 0.0001);
    assert!((keys[1].value.x - 6.4).abs() < 0.0001);
    assert!((keys[2].value.x - 6.1).abs() < 0.0001);
}

#[test]
fn lifetime_range_expands_symmetrically_from_authored_variation() {
    let mut emitter = sample_emitter();
    emitter.lifespan = 2.0;
    emitter.lifespan_variation = 0.75;

    assert_eq!(lifetime_range(&emitter), (1.25, 2.75));
}

#[test]
fn lifetime_range_clamps_non_positive_results() {
    let mut emitter = sample_emitter();
    emitter.lifespan = 0.05;
    emitter.lifespan_variation = 1.0;

    assert_eq!(lifetime_range(&emitter), (0.1, 1.1));
}

#[test]
fn trail_particles_orient_along_velocity() {
    let mut emitter = sample_emitter();
    emitter.particle_type = 1;

    assert!(matches!(orient_mode(&emitter), OrientMode::AlongVelocity));
}

#[test]
fn velocity_orient_flag_orients_particles_along_velocity() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_VELOCITY_ORIENT;

    assert!(matches!(orient_mode(&emitter), OrientMode::AlongVelocity));
}

#[test]
fn xy_quad_flag_uses_parallel_camera_depth_plane() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_XY_QUAD;

    assert!(matches!(
        orient_mode(&emitter),
        OrientMode::ParallelCameraDepthPlane
    ));
}

#[test]
fn tail_particles_flag_does_not_force_velocity_orient() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_TAIL_PARTICLES;

    assert!(matches!(
        orient_mode(&emitter),
        OrientMode::FaceCameraPosition
    ));
}

#[test]
fn twinkle_emitters_declare_blink_modifiers() {
    let mut emitter = sample_emitter();
    emitter.twinkle_speed = 3.0;
    emitter.twinkle_percent = 0.8;
    emitter.twinkle_scale_min = 0.5;
    emitter.twinkle_scale_max = 1.5;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(has_authored_twinkle(&emitter));
    assert!(modifiers.twinkle.is_some());
    assert!(modifiers.init.twinkle_phase.is_some());
    assert!(modifiers.init.twinkle_enabled.is_some());
}

#[test]
fn size_variation_emitters_declare_per_particle_scale_modifiers() {
    let mut emitter = sample_emitter();
    emitter.scale_variation = 0.4;
    emitter.flags |= PARTICLE_FLAG_SIZE_VARIATION_2D;
    emitter.scale_variation_y = 0.2;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(has_authored_size_variation(&emitter));
    assert!(modifiers.size_variation.is_some());
    assert!(modifiers.init.size_variation.is_some());
}

#[test]
fn full_visibility_identity_scale_disables_twinkle() {
    let mut emitter = sample_emitter();
    emitter.twinkle_speed = 3.0;
    emitter.twinkle_percent = 1.0;
    emitter.twinkle_scale_min = 1.0;
    emitter.twinkle_scale_max = 1.0;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(!has_authored_twinkle(&emitter));
    assert!(modifiers.twinkle.is_none());
    assert!(modifiers.init.twinkle_phase.is_none());
    assert!(modifiers.init.twinkle_enabled.is_none());
}

#[test]
fn zero_visibility_twinkle_still_builds_blink_path() {
    let mut emitter = sample_emitter();
    emitter.twinkle_speed = 3.0;
    emitter.twinkle_percent = 0.0;
    emitter.twinkle_scale_min = 0.5;
    emitter.twinkle_scale_max = 1.5;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(has_authored_twinkle(&emitter));
    assert!(modifiers.twinkle.is_some());
    assert!(modifiers.init.twinkle_phase.is_some());
    assert!(modifiers.init.twinkle_enabled.is_some());
}

#[test]
fn spin_emitters_declare_authored_rotation() {
    let mut emitter = sample_emitter();
    emitter.base_spin_variation = std::f32::consts::TAU;
    emitter.spin = 0.8;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(has_authored_spin(&emitter));
    assert!(modifiers.orient_rotation.is_some());
    assert!(modifiers.init.rotation.is_some());
    assert!(modifiers.init.angular_velocity.is_some());
}

#[test]
fn negate_spin_flag_allocates_stable_spin_sign_attribute() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_NEGATE_SPIN;
    emitter.spin = 0.8;

    let modifiers = build_expr_modifiers(&emitter, 1.0);

    assert!(modifiers.init.spin_sign.is_some());
    assert!(modifiers.init.rotation.is_some());
    assert!(modifiers.init.angular_velocity.is_some());
}

#[test]
fn wind_vector_maps_to_bevy_axes_and_scale() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_WIND_ENABLED;
    emitter.wind_vector = [1.0, 2.0, 3.0];
    emitter.wind_time = 2.0;

    assert!(has_authored_wind(&emitter));
    assert_eq!(wind_accel_bevy(&emitter, 1.5), Vec3::new(1.5, 4.5, -3.0));
}

#[test]
fn dynamic_wind_flag_disables_static_wind_path() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_WIND_ENABLED | PARTICLE_FLAG_WIND_DYNAMIC;
    emitter.wind_vector = [1.0, 2.0, 3.0];
    emitter.wind_time = 2.0;

    assert!(!has_authored_wind(&emitter));
}

#[test]
fn gravity_vector_maps_to_bevy_axes() {
    let mut emitter = sample_emitter();
    emitter.gravity_vector = [1.0, 2.0, -3.0];

    assert_eq!(gravity_accel_bevy(&emitter), Vec3::new(1.0, -3.0, -2.0));
}

#[test]
fn wind_strength_stops_after_wind_time() {
    assert_eq!(wind_strength_at_age(0.0, 1.5), 1.0);
    assert_eq!(wind_strength_at_age(1.5, 1.5), 1.0);
    assert_eq!(wind_strength_at_age(1.5001, 1.5), 0.0);
    assert_eq!(wind_strength_at_age(0.5, 0.0), 0.0);
}

#[test]
fn color_gradient_uses_full_opacity_key_timeline_when_present() {
    let mut emitter = sample_emitter();
    emitter.colors = [[255.0, 0.0, 0.0], [0.0, 255.0, 0.0], [0.0, 0.0, 255.0]];
    emitter.mid_point = 0.5;
    emitter.opacity_keys = vec![(0.0, 0.1), (0.25, 0.4), (0.75, 0.8), (1.0, 0.2)];

    let gradient = build_color_gradient(&emitter);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0].ratio(), 0.0);
    assert_eq!(keys[0].value.w, 0.1);
    assert!((keys[1].ratio() - 0.25).abs() < 0.0001);
    assert!((keys[1].value.x - 0.5).abs() < 0.0001);
    assert!((keys[1].value.y - 0.5).abs() < 0.0001);
    assert_eq!(keys[1].value.w, 0.4);
    assert!((keys[2].ratio() - 0.75).abs() < 0.0001);
    assert!((keys[2].value.y - 0.5).abs() < 0.0001);
    assert!((keys[2].value.z - 0.5).abs() < 0.0001);
    assert_eq!(keys[2].value.w, 0.8);
    assert_eq!(keys[3].ratio(), 1.0);
    assert!((keys[3].value.w - 0.2).abs() < 0.0001);
}

#[test]
fn color_gradient_merges_color_and_opacity_key_times() {
    let mut emitter = sample_emitter();
    emitter.color_keys = vec![
        (0.0, [255.0, 0.0, 0.0]),
        (0.5, [0.0, 255.0, 0.0]),
        (1.0, [0.0, 0.0, 255.0]),
    ];
    emitter.opacity_keys = vec![(0.0, 0.2), (0.25, 0.4), (1.0, 0.8)];

    let gradient = build_color_gradient(&emitter);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0].ratio(), 0.0);
    assert!((keys[1].ratio() - 0.25).abs() < 0.0001);
    assert!((keys[2].ratio() - 0.5).abs() < 0.0001);
    assert_eq!(keys[3].ratio(), 1.0);
    assert!((keys[1].value.x - 0.5).abs() < 0.0001);
    assert!((keys[1].value.y - 0.5).abs() < 0.0001);
    assert_eq!(keys[1].value.w, 0.4);
    assert_eq!(keys[2].value.y, 1.0);
    assert!((keys[2].value.w - 0.53333336).abs() < 0.0001);
}

#[test]
fn active_cell_track_prefers_head_track() {
    let mut emitter = sample_emitter();
    emitter.head_cell_track = [2, 4, 6];
    emitter.tail_cell_track = [7, 8, 9];

    assert_eq!(active_cell_track(&emitter), Some([2, 4, 6]));
}

#[test]
fn active_cell_track_falls_back_to_tail_track() {
    let mut emitter = sample_emitter();
    emitter.tail_cell_track = [3, 5, 7];

    assert_eq!(active_cell_track(&emitter), Some([3, 5, 7]));
}

#[test]
fn atlas_emitters_without_authored_cell_track_use_first_cell() {
    let emitter = sample_emitter();

    assert_eq!(
        flipbook_sprite_mode(&emitter),
        Some(FlipbookSpriteMode::FirstCell)
    );
}

#[test]
fn atlas_emitters_with_random_texture_use_random_cell_mode() {
    let mut emitter = sample_emitter();
    emitter.flags |= PARTICLE_FLAG_RANDOM_TEXTURE;

    assert_eq!(
        flipbook_sprite_mode(&emitter),
        Some(FlipbookSpriteMode::RandomCell)
    );
}

#[test]
fn atlas_emitters_with_authored_cell_track_use_track_mode() {
    let mut emitter = sample_emitter();
    emitter.head_cell_track = [2, 4, 6];

    assert_eq!(
        flipbook_sprite_mode(&emitter),
        Some(FlipbookSpriteMode::CellTrack([2, 4, 6]))
    );
}

#[test]
fn sample_cell_track_frame_uses_midpoint_segments() {
    let track = [2, 6, 10];

    assert_eq!(sample_cell_track_frame(track, 0.25, 0.0, 16), 2);
    assert_eq!(sample_cell_track_frame(track, 0.25, 0.25, 16), 6);
    assert_eq!(sample_cell_track_frame(track, 0.25, 0.625, 16), 8);
    assert_eq!(sample_cell_track_frame(track, 0.25, 1.0, 16), 10);
}

#[test]
fn particle_blend_type_zero_is_opaque() {
    let writer = ExprWriter::new();
    let alpha_mode = emitter_alpha_mode(0, writer.lit(0.5_f32).expr());

    assert!(matches!(alpha_mode, AlphaMode::Opaque));
}

#[test]
fn particle_blend_type_one_uses_alpha_key() {
    let writer = ExprWriter::new();
    let alpha_mode = emitter_alpha_mode(1, writer.lit(0.5_f32).expr());

    assert!(matches!(alpha_mode, AlphaMode::Mask(_)));
}

#[test]
fn particle_blend_type_three_uses_alpha_blend() {
    let writer = ExprWriter::new();
    let alpha_mode = emitter_alpha_mode(3, writer.lit(0.5_f32).expr());

    assert!(matches!(alpha_mode, AlphaMode::Blend));
}

#[test]
fn particle_blend_type_four_is_additive() {
    let writer = ExprWriter::new();
    let alpha_mode = emitter_alpha_mode(4, writer.lit(0.5_f32).expr());

    assert!(matches!(alpha_mode, AlphaMode::Add));
}

#[test]
fn torch_emitter_translation_matches_particle_position() {
    let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
    if !path.exists() {
        return;
    }

    let skin_fdids = [0_u32; 3];
    let model = crate::asset::m2::load_m2_uncached(path, &skin_fdids).unwrap();
    let emitter = model.particle_emitters.into_iter().next().unwrap();

    let translation = emitter_translation(&emitter);

    let expected = Vec3::new(0.63709766, -0.07413276, 0.0009614461);
    assert!(
        translation.distance(expected) < 0.00001,
        "translation={translation:?}"
    );
}
