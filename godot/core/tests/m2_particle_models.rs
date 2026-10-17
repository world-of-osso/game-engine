use game_engine_core::{
    m2::ParticleEmitter,
    m2_particles::{EmitterSim, ViewBasis},
};
use glam::{Mat3, Mat4, Vec3};

#[test]
fn p5_parent_at_exact_lifespan_retires() {
    let mut e = emitter();
    let mut sim = EmitterSim::new(&e, 17);
    sim.update(
        &e,
        0.015,
        Mat4::IDENTITY,
        Mat3::IDENTITY,
        1.0,
        &mut |_, _| None,
    );
    assert_eq!(sim.particles().len(), 1);
    e.lifespan = sim.particles()[0].age + 0.05;
    e.emission_rate = 0.0;
    sim.set_animation(
        &e,
        &game_engine_core::m2::AnimTime {
            sequence: 0,
            time_ms: 0,
            global_ms: 0,
            global_sequences: &[],
        },
    );
    sim.update(
        &e,
        0.05,
        Mat4::IDENTITY,
        Mat3::IDENTITY,
        1.0,
        &mut |_, _| None,
    );
    assert!(
        sim.particles().is_empty(),
        "a parent at its lifespan cannot survive to emit children"
    );
}

fn emitter() -> ParticleEmitter {
    ParticleEmitter {
        flags: 0x20000,
        emitter_type: 1,
        emission_rate: 100.0,
        lifespan: 1.0,
        emission_speed: 0.0,
        area_length: 0.0,
        area_width: 0.0,
        twinkle_percent: 1.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        ..Default::default()
    }
}

#[test]
fn p5_geometry_reference_is_not_a_billboard() {
    let e = ParticleEmitter {
        particle_model_filename: Some("fixture/arrow.m2".into()),
        ..emitter()
    };
    let mut sim = EmitterSim::new(&e, 17);
    sim.update(
        &e,
        0.025,
        Mat4::IDENTITY,
        Mat3::IDENTITY,
        1.0,
        &mut |_, _| None,
    );
    assert!(!sim.particles().is_empty());
    let mut quads = Vec::new();
    sim.quads(
        &e,
        &ViewBasis {
            right: Vec3::X,
            up: Vec3::Y,
            back: Vec3::Z,
        },
        &mut quads,
    );
    assert!(
        quads.is_empty(),
        "geometry references must not draw replacement quads"
    );
}
