use bevy::prelude::Vec3;
use bevy_hanabi::{AlphaMode, ExprWriter};

use super::{
    active_cell_track, build_color_gradient, build_effect_asset, build_expr_modifiers,
    build_size_gradient, emitter_alpha_mode, emitter_rate_scale, emitter_spawn_radius,
    emitter_translation, is_fire_effect, sample_cell_track_frame,
};
use crate::asset::m2_anim::M2Bone;
use crate::asset::m2_particle::M2ParticleEmitter;

fn sample_emitter() -> M2ParticleEmitter {
    M2ParticleEmitter {
        flags: 0,
        position: [0.0, 0.0, 0.0],
        bone_index: 0,
        texture_index: 0,
        texture_fdid: None,
        blend_type: 4,
        emitter_type: 1,
        tile_rows: 4,
        tile_cols: 4,
        emission_speed: 1.0,
        speed_variation: 0.0,
        vertical_range: 0.1,
        horizontal_range: std::f32::consts::TAU,
        gravity: 0.0,
        lifespan: 1.0,
        emission_rate: 20.0,
        area_length: 0.1,
        area_width: 0.1,
        drag: 0.0,
        colors: [[255.0, 128.0, 64.0]; 3],
        color_keys: Vec::new(),
        opacity: [1.0, 1.0, 0.0],
        opacity_keys: Vec::new(),
        scales: [[0.1, 0.1], [0.2, 0.2], [0.05, 0.05]],
        scale_keys: Vec::new(),
        head_cell_track: [0, 0, 0],
        tail_cell_track: [0, 0, 0],
        burst_multiplier: 1.0,
        mid_point: 0.5,
    }
}

fn sample_bone(pivot: [f32; 3]) -> M2Bone {
    M2Bone {
        key_bone_id: 0,
        flags: 0,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot,
    }
}

#[test]
fn textured_emitters_declare_hanabi_texture_slot() {
    let mut emitter = sample_emitter();
    emitter.texture_fdid = Some(145513);

    let asset = build_effect_asset(&emitter, 1.0);

    assert_eq!(asset.texture_layout().layout.len(), 1);
    assert_eq!(asset.texture_layout().layout[0].name, "color");
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
    let bones = vec![sample_bone([0.25, 0.5, 0.75])];

    let translation = emitter_translation(&emitter, &bones);

    assert_eq!(translation, Vec3::new(1.0, 3.0, -2.0));
}

#[test]
fn sphere_emitters_use_area_as_spawn_radius() {
    let mut emitter = sample_emitter();
    emitter.emitter_type = 1;
    emitter.area_length = 0.4;
    emitter.area_width = 0.2;

    assert_eq!(emitter_spawn_radius(&emitter), 0.2);
}

#[test]
fn non_sphere_emitters_do_not_expand_spawn_radius() {
    let mut emitter = sample_emitter();
    emitter.emitter_type = 0;
    emitter.area_length = 0.4;
    emitter.area_width = 0.2;

    assert_eq!(emitter_spawn_radius(&emitter), 0.0);
}

#[test]
fn fire_emitters_use_four_x_rate_scale() {
    let mut emitter = sample_emitter();
    emitter.texture_fdid = Some(145513);

    assert!(is_fire_effect(&emitter));
    assert_eq!(emitter_rate_scale(&emitter), 4.0);
}

#[test]
fn non_fire_emitters_keep_default_rate_scale() {
    let mut emitter = sample_emitter();
    emitter.texture_fdid = None;

    assert!(!is_fire_effect(&emitter));
    assert_eq!(emitter_rate_scale(&emitter), 1.0);
}

#[test]
fn burst_multiplier_scales_particle_size_gradient() {
    let mut emitter = sample_emitter();
    emitter.burst_multiplier = 2.5;

    let gradient = build_size_gradient(&emitter, 1.0);
    let keys = gradient.keys();

    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0].value, Vec3::new(0.25, 0.25, 1.0));
    assert_eq!(keys[1].value, Vec3::new(0.5, 0.5, 1.0));
    assert_eq!(keys[2].value, Vec3::new(0.125, 0.125, 1.0));
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
    assert_eq!(keys[0].value, Vec3::new(0.3, 0.6, 1.0));
    assert!((keys[1].ratio() - 0.25).abs() < 0.0001);
    assert!((keys[1].value.x - 0.9).abs() < 0.0001);
    assert!((keys[1].value.y - 1.2).abs() < 0.0001);
    assert_eq!(keys[1].value.z, 1.0);
    assert!((keys[2].ratio() - 0.75).abs() < 0.0001);
    assert!((keys[2].value.x - 1.5).abs() < 0.0001);
    assert!((keys[2].value.y - 1.8).abs() < 0.0001);
    assert_eq!(keys[2].value.z, 1.0);
    assert_eq!(keys[3].ratio(), 1.0);
    assert_eq!(keys[3].value, Vec3::new(2.1, 2.4, 1.0));
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

    let translation = emitter_translation(&emitter, &model.bones);

    let expected = Vec3::new(0.63709766, -0.07413276, 0.0009614461);
    assert!(
        translation.distance(expected) < 0.00001,
        "translation={translation:?}"
    );
}
