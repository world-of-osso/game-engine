use glam::{Vec2, Vec3};

use super::{EmitterState, Orbit, format_lights, format_overlay};
use game_engine_core::{m2::ParticleEmitter, m2_lights::PointLight};

#[test]
fn overlay_lists_each_emitters_state_and_key_fields() {
    let torch_flame = ParticleEmitter {
        blend_type: 4,
        emitter_type: 1,
        lifespan: 1.25,
        emission_rate: 12.0,
        texture_fdid: Some(145513),
        ..Default::default()
    };
    let spline = ParticleEmitter {
        emitter_type: 3,
        ..Default::default()
    };

    let text = format_overlay(
        "club_1h_torch_a_01 (145304) [1/3]",
        &[torch_flame.clone(), torch_flame, spline],
        &[
            EmitterState::On,
            EmitterState::Off,
            EmitterState::NotSimulated,
        ],
        37,
    );

    assert!(text.starts_with("Particle Debug\nModel: club_1h_torch_a_01 (145304) [1/3]\n"));
    assert!(text.contains("Emitters: 3  Particles drawn: 37"));
    assert!(text.contains("Emitter #0 [on]\nblend=4 type=1 particle=0"));
    assert!(text.contains("Emitter #1 [off]\n"));
    assert!(text.contains("Emitter #2 [not simulated]\nblend=0 type=3"));
    assert!(text.contains("life=1.250 +/- 0.000 rate=12.000"));
    assert!(text.contains("tex=Some(145513) flags=0x0"));
}

#[test]
fn light_lines_give_bone_colour_and_attenuation() {
    let flame = PointLight {
        color: [0.513_333, 0.319_216, 0.146_667],
        attenuation_start: 1.6666,
        attenuation_end: 5.266_66,
        visible: true,
    };
    assert_eq!(
        format_lights(&[(9, flame)]),
        [
            String::new(),
            "Light #0 point bone=9 color=(0.513, 0.319, 0.147) attenuation=1.667-5.267".to_string()
        ]
    );
}

#[test]
fn overlay_without_emitters_says_so() {
    let text = format_overlay("empty (1) [1/1]", &[], &[], 0);
    assert!(text.ends_with("No particle emitters"));
    assert!(!text.contains("Emitters:"));
}

#[test]
fn orbit_starts_on_the_original_shot_and_eases_zoom_within_limits() {
    let mut orbit = Orbit::new(Vec3::new(0.0, 0.5, 0.0), 3.0);
    // Original OrbitCamera: base pitch 0.15 rad, straight down +Z at yaw 0.
    let eye = orbit.eye();
    assert!((eye - Vec3::new(0.0, 0.5 + 3.0 * 0.15f32.sin(), 3.0 * 0.15f32.cos())).length() < 1e-5);

    orbit.zoom(100.0);
    assert_eq!(orbit.target_distance, 0.5);
    orbit.ease();
    assert!((orbit.distance - (3.0 + (0.5 - 3.0) * 0.25)).abs() < 1e-5);
    orbit.zoom(-1000.0);
    assert_eq!(orbit.target_distance, 20.0);
}

#[test]
fn orbit_drag_turns_yaw_and_keeps_the_eye_off_the_vertical() {
    let mut orbit = Orbit::new(Vec3::ZERO, 3.0);
    orbit.drag(Vec2::new(-100.0, 0.0), 0.01);
    assert!((orbit.yaw - 1.0).abs() < 1e-5);
    orbit.drag(Vec2::new(0.0, 10_000.0), 0.01);
    let eye = orbit.eye();
    assert!(
        eye.y < 3.0 && eye.x.hypot(eye.z) > 0.1,
        "eye {eye:?} reached the pole"
    );
}
