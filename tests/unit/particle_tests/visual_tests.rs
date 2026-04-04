use super::*;

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
fn offset_by_spin_flag_declares_render_offset_modifier() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_OFFSET_BY_SPIN;

    assert!(build_offset_by_spin_modifier(&emitter).is_some());
}

#[test]
fn offset_by_spin_modifier_tracks_negate_spin_flag() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_OFFSET_BY_SPIN | PARTICLE_FLAG_NEGATE_SPIN;

    let modifier = build_offset_by_spin_modifier(&emitter).expect("modifier");

    assert!(modifier.negate_spin);
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
