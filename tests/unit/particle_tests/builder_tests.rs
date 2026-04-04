use super::*;

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

    let asset = build_effect_asset_with_mode(
        &emitter,
        1.0,
        1.0,
        ParticleSpawnMode::BurstOnce,
        ParticleSpawnSource::Standalone,
        &[],
    );

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
fn child_particle_effects_use_gpu_parent_events() {
    let emitter = sample_emitter();
    let asset = build_effect_asset_with_mode(
        &emitter,
        1.0,
        1.0,
        ParticleSpawnMode::Continuous,
        ParticleSpawnSource::ChildFromParentParticles,
        &[],
    );

    assert_eq!(asset.spawner, bevy_hanabi::SpawnerSettings::default());
}

#[test]
fn parent_particle_effects_emit_spawn_events_for_child_emitters() {
    let parent = sample_emitter();
    let mut child = sample_emitter();
    child.emission_rate = 90.0;

    let asset = build_effect_asset_with_mode(
        &parent,
        1.0,
        1.0,
        ParticleSpawnMode::Continuous,
        ParticleSpawnSource::Standalone,
        &[child],
    );

    assert_eq!(asset.update_modifiers().count(), 2);
}

#[test]
fn child_emitter_event_count_uses_per_frame_approximation() {
    let mut child = sample_emitter();
    child.emission_rate = 90.0;

    assert_eq!(child_emitter_event_count(&child, 1.0), 2);
}

#[test]
fn default_emitters_use_global_simulation_space() {
    let emitter = sample_emitter();

    assert!(!emitter_uses_follow_position(&emitter));
    assert_eq!(emitter_simulation_space(&emitter), SimulationSpace::Global);
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
fn no_global_scale_flag_skips_particle_density_multiplier() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_NO_GLOBAL_SCALE;
    emitter.emission_rate_variation = 4.0;
    let graphics = GraphicsOptions {
        particle_density: 50,
        render_scale: 1.0,
        ..GraphicsOptions::default()
    };

    assert!(
        (scaled_emission_rate(&emitter, graphics.particle_density_multiplier()) - 22.0).abs()
            < 0.0001
    );
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
