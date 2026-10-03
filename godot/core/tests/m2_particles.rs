//! M2 particle emitters: authored parsing of the Stockade portal and the shared CPU
//! simulation rules (WebWowViewerCpp `particleEmitter.cpp` and its generators).
use game_engine_core::{
    m2::{self, ParticleEmitter},
    m2_particles::{
        self, EmitterSim, ParticleShape, PixelShader, Quad, ViewBasis, appearance,
        integrate_particle, particle_lifespan, pool_capacity, sample_keys, twinkle_scale,
    },
};
use glam::{Mat3, Mat4, Vec2, Vec3};

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read(format!("{fdid}.m2")), &read(format!("{fdid}00.skin"))).unwrap()
}

fn portal() -> Vec<ParticleEmitter> {
    model(197007).particle_emitters
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-4
}

/// Godot axes: WoW (x, y, z) -> (x, z, -y).
fn wow_to_godot() -> Mat3 {
    Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)
}

/// `instanceportal.m2` (197007): six sphere emitters on bones 2..7, 2.74 yd above the
/// model origin; values read from the MD20 emitter records, TXID and EXP2.
#[test]
fn instance_portal_parses_its_six_authored_emitters() {
    let emitters = portal();
    assert_eq!(emitters.len(), 6);
    let column = |f: fn(&ParticleEmitter) -> u32| emitters.iter().map(f).collect::<Vec<_>>();
    assert_eq!(
        column(|e| e.flags),
        [
            0x7682_0030,
            0x6683_0231,
            0x6683_0230,
            0x7683_0230,
            0x7683_0230,
            0x6292_1230
        ]
    );
    assert_eq!(column(|e| e.bone_index.into()), [2, 3, 4, 5, 6, 7]);
    assert_eq!(column(|e| e.blend_type.into()), [4, 4, 4, 7, 7, 4]);
    assert_eq!(column(|e| e.emitter_type.into()), [2; 6]);
    assert_eq!(
        column(|e| e.texture_fdid.unwrap()),
        [7361546, 7361554, 7361554, 7361550, 7361550, 7361557]
    );
    assert_eq!(column(|e| e.tile_rows.into()), [1, 2, 2, 2, 2, 1]);
    let rates: Vec<f32> = emitters.iter().map(|e| e.emission_rate).collect();
    assert_eq!(rates, [200.0, 100.0, 100.0, 100.0, 100.0, 12.0]);
    let lives: Vec<(f32, f32)> = emitters
        .iter()
        .map(|e| (e.lifespan, e.lifespan_variation))
        .collect();
    let expected_lives = [
        (1.0, 0.3),
        (0.9, 0.05),
        (0.5, 0.1),
        (0.5, 0.1),
        (0.9, 0.05),
        (0.45, 0.1),
    ];
    for (actual, expected) in lives.iter().zip(expected_lives) {
        assert!(
            close(actual.0, expected.0) && close(actual.1, expected.1),
            "{lives:?}"
        );
    }
    // Every MD20 zSource track holds 255; the EXP2 record's 0 replaces it.
    assert!(emitters.iter().all(|e| e.z_source == 0.0));
    let first = &emitters[0];
    assert!(close(first.emission_speed, -2.777_778));
    assert!(close(first.area_length, 4.444_444_5) && close(first.area_width, 4.444_444_5));
    assert!(close(first.vertical_range, std::f32::consts::PI) && first.horizontal_range == 0.0);
    assert!(close(first.position[2], 2.737_066));
    assert_eq!(first.head_cell_keys, vec![(0.0, 1), (1.0, 1)]);
    assert!(emitters[1].head_cell_keys.is_empty());
    let multi = first.multi_texture.as_ref().unwrap();
    assert_eq!(
        multi.texture_fdids,
        [Some(7361546), Some(7361548), Some(7361548)]
    );
    // EXP2: colour and alpha multipliers 1; emitter 5's alpha cutoff ramp.
    assert!(
        emitters
            .iter()
            .all(|e| e.color_mult == 1.0 && e.alpha_mult == 1.0)
    );
    let cutoff: Vec<(f32, f32)> = emitters[5].alpha_cutoff_keys.clone();
    let expected_cutoff = [(0.0, 15.0), (28685.0, 30.0), (32767.0, 82.0)];
    assert_eq!(cutoff.len(), 3);
    for ((time, value), (raw_time, raw_value)) in cutoff.iter().zip(expected_cutoff) {
        assert!(close(*time, raw_time / 32767.0) && close(*value, raw_value / 32767.0));
    }
}

#[test]
fn portal_emitters_are_spheres_with_their_authored_pixel_shaders() {
    let emitters = portal();
    assert!(
        emitters
            .iter()
            .all(|e| m2_particles::shape(e) == Some(ParticleShape::Sphere))
    );
    let shaders: Vec<PixelShader> = emitters.iter().map(m2_particles::pixel_shader).collect();
    use PixelShader::{Mod, ThreeColorThreeAlpha};
    assert_eq!(
        shaders,
        [
            ThreeColorThreeAlpha,
            Mod,
            Mod,
            ThreeColorThreeAlpha,
            ThreeColorThreeAlpha,
            Mod
        ]
    );
}

/// Pools hold authored (rate + variation) x (lifespan + variation) x 1.15 particles
/// (solarityclient `reserve_authored_capacity`, build 12340 `0x009F23CC`), capped at
/// WebWowViewerCpp's 500 quads per emitter (`particleEmitter.cpp:462-464`).
#[test]
fn pool_capacity_follows_authored_rate_and_lifetime_with_a_hard_cap() {
    let emitters = portal();
    let capacities: Vec<usize> = emitters.iter().map(pool_capacity).collect();
    assert_eq!(capacities, [299, 110, 69, 69, 110, 8]);
    let mut storm = emitters[0].clone();
    storm.emission_rate = 10_000.0;
    storm.lifespan = 10.0;
    assert_eq!(pool_capacity(&storm), m2_particles::EMITTER_CAPACITY_CAP);
    assert_eq!(m2_particles::EMITTER_CAPACITY_CAP, 500);
}

#[test]
fn lifetime_keys_interpolate_linearly_and_clamp() {
    let emitters = portal();
    let alpha = &emitters[1].opacity_keys;
    assert!(close(sample_keys(alpha, 0.25, 1.0), 0.121_232_42));
    assert!(close(sample_keys(alpha, 0.5, 1.0), 6425.0 / 32767.0));
    assert_eq!(sample_keys(alpha, 1.5, 1.0), 0.0);
    assert_eq!(sample_keys(alpha, -0.5, 1.0), 0.0);
    assert_eq!(sample_keys::<f32>(&[], 0.5, 1.0), 1.0);
    let scale = sample_keys(&emitters[1].scale_keys, 0.75, [1.0, 1.0]);
    assert!(close(scale[0], 0.480_045_2) && close(scale[1], 0.480_045_2));
}

/// Age over the maximum lifespan (0.9 + 0.05) selects the colour, opacity and scale;
/// twinkle 1.5 multiplies emitter 1's scale; colour is authored bytes / 255.
#[test]
fn appearance_samples_colour_opacity_and_size_over_normalized_age() {
    let emitters = portal();
    let quarter = appearance(&emitters[1], 0.2375, 0x1234);
    assert!(close(quarter.color.x, 71.000_61 / 255.0), "{quarter:?}");
    assert!(close(quarter.color.z, (210.0 - 40.0 * 0.499_985) / 255.0));
    assert!(close(quarter.alpha, 0.121_232_42));
    let late = appearance(&emitters[1], 0.7125, 0x1234);
    assert!(close(late.scale.x, 0.480_045_2) && close(late.scale.y, 0.480_045_2));
    assert_eq!(twinkle_scale(&emitters[1], 0.3, 7), Some(1.5));
    assert_eq!(twinkle_scale(&emitters[5], 0.3, 7), Some(0.4));
    // Emitter 5 varies size by +-20%, the same factor on both axes.
    for seed in [1u16, 999, 40_000, 65_535] {
        let sized = appearance(&emitters[5], 0.0, seed);
        assert!(
            sized.scale.x >= 6.474 * 0.8 && sized.scale.x <= 6.475 * 1.2,
            "{sized:?}"
        );
        assert_eq!(sized.scale.x, sized.scale.y);
    }
    // 2x2 atlas with RANDOM_TEXTURE and no head track: a cell 0..4 fixed per seed.
    let cells: Vec<u32> = (0..64u16)
        .map(|s| appearance(&emitters[1], 0.1, s * 997).head_cell)
        .collect();
    assert!(cells.iter().all(|&cell| cell < 4));
    assert!((0..4).all(|cell| cells.contains(&cell)), "{cells:?}");
    assert_eq!(
        appearance(&emitters[1], 0.1, 5).head_cell,
        appearance(&emitters[1], 0.8, 5).head_cell
    );
}

#[test]
fn twinkle_hides_particles_outside_its_percentage() {
    let mut emitter = portal()[1].clone();
    emitter.twinkle_percent = 0.0;
    emitter.twinkle_scale_min = 1.0;
    emitter.twinkle_scale_max = 2.0;
    let shown = (0..200u16)
        .filter(|&seed| twinkle_scale(&emitter, 0.05 * f32::from(seed), seed).is_some())
        .count();
    assert_eq!(shown, 0);
    emitter.twinkle_percent = 1.0;
    let scales: Vec<f32> = (0..50u16)
        .filter_map(|seed| twinkle_scale(&emitter, 0.0, seed))
        .collect();
    assert_eq!(scales.len(), 50);
    assert!(scales.iter().all(|s| (1.0..=2.0).contains(s)));
}

/// A particle's lifespan varies by its signed seed: lifespan + seed/32767 x variation.
#[test]
fn particle_lifespan_varies_by_signed_seed() {
    let emitter = &portal()[1];
    assert!(close(particle_lifespan(emitter, 0x7FFF), 0.95));
    assert!(close(particle_lifespan(emitter, 0x8001), 0.85));
    assert!(close(particle_lifespan(emitter, 0), 0.9));
}

/// Position += v dt + g dt^2 / 2, then v += g dt and v *= 1 - min(drag dt, 1).
#[test]
fn integration_is_ballistic_with_linear_drag() {
    let (position, velocity) = integrate_particle(Vec3::ZERO, Vec3::X, 1.0, Vec3::NEG_Y, 0.5);
    assert_eq!(position, Vec3::new(1.0, -0.5, 0.0));
    assert_eq!(velocity, Vec3::new(0.5, -0.5, 0.0));
    let (_, stopped) = integrate_particle(Vec3::ZERO, Vec3::X, 0.1, Vec3::ZERO, 20.0);
    assert_eq!(stopped, Vec3::ZERO);
}

/// Emitter 1 at 60 fps in model space (flag 0x10): 100/s for about 0.9 s keeps about
/// 90 alive, below its 110 pool; the sphere shell (radius 4.44, horizontal range 0)
/// launches inward (speed -3.33) in the emitter's XZ plane.
#[test]
fn portal_sphere_emitter_reaches_steady_state_on_its_shell() {
    let emitter = &portal()[1];
    let mut sim = EmitterSim::new(emitter, 42);
    assert_eq!(sim.capacity(), 110);
    for _ in 0..120 {
        sim.update(emitter, 1.0 / 60.0, Mat4::IDENTITY, wow_to_godot(), 1.0);
    }
    let live = sim.particles().len();
    assert!((75..=105).contains(&live), "live {live}");
    for particle in sim.particles() {
        assert_eq!(particle.position.y, 0.0, "{particle:?}");
        let radius = particle.position.length();
        assert!(radius <= 4.4445 && radius > 0.9, "radius {radius}");
        assert!(
            particle.velocity.dot(particle.position) < 0.0,
            "moves inward"
        );
        assert!(particle.age <= particle_lifespan(emitter, particle.seed));
    }
    // Density scales the emission rate.
    let mut sparse = EmitterSim::new(emitter, 42);
    for _ in 0..120 {
        sparse.update(emitter, 1.0 / 60.0, Mat4::IDENTITY, wow_to_godot(), 0.25);
    }
    let sparse_live = sparse.particles().len();
    assert!((12..=35).contains(&sparse_live), "sparse {sparse_live}");
}

/// A long gap is replayed in 0.1 s steps capped at the lifespan
/// (`particleEmitter.cpp:541-568`): one 10 s update equals a lifetime of history.
#[test]
fn long_updates_replay_at_most_one_lifetime() {
    let emitter = &portal()[1];
    let mut sim = EmitterSim::new(emitter, 7);
    sim.update(emitter, 10.0, Mat4::IDENTITY, wow_to_godot(), 1.0);
    let live = sim.particles().len();
    assert!((60..=110).contains(&live), "live {live}");
}

/// World-space emitters (no 0x10) transform spawns by the emitter matrix and fall
/// along world gravity converted from WoW axes (-Z WoW is -Y Godot).
#[test]
fn world_space_plane_emitter_falls_along_converted_gravity() {
    let emitter = ParticleEmitter {
        flags: 0x0002_0000,
        emitter_type: 1,
        blend_type: 2,
        emission_rate: 30.0,
        lifespan: 2.0,
        gravity: 1.0,
        gravity_vector: [0.0, 0.0, -1.0],
        tile_rows: 1,
        tile_cols: 1,
        ..ParticleEmitter::default()
    };
    let at = Mat4::from_translation(Vec3::new(10.0, 5.0, 0.0));
    let mut sim = EmitterSim::new(&emitter, 3);
    for _ in 0..30 {
        sim.update(&emitter, 1.0 / 30.0, at, wow_to_godot(), 1.0);
    }
    assert!(!sim.particles().is_empty());
    for particle in sim.particles() {
        assert_eq!((particle.position.x, particle.position.z), (10.0, 0.0));
        assert!(
            particle.position.y < 5.0 && particle.velocity.y < 0.0,
            "{particle:?}"
        );
        assert!(close(particle.velocity.x, 0.0) && close(particle.velocity.z, 0.0));
    }
}

fn camera() -> ViewBasis {
    ViewBasis {
        right: Vec3::X,
        up: Vec3::Y,
        back: Vec3::Z,
    }
}

fn one_quad(emitter: &ParticleEmitter, matrix: Mat4) -> Quad {
    let mut sim = EmitterSim::new(emitter, 11);
    sim.update(emitter, 0.05, matrix, wow_to_godot(), 1.0);
    let mut quads = Vec::new();
    sim.quads(emitter, &camera(), &mut quads);
    assert!(!quads.is_empty());
    quads[0]
}

fn still_emitter(flags: u32) -> ParticleEmitter {
    ParticleEmitter {
        flags: flags | 0x0002_0000,
        emitter_type: 1,
        blend_type: 4,
        emission_rate: 100.0,
        lifespan: 1.0,
        tile_rows: 2,
        tile_cols: 2,
        scale_keys: vec![(0.0, [0.5, 0.25]), (1.0, [0.5, 0.25])],
        color_keys: vec![(0.0, [255.0, 128.0, 0.0])],
        opacity_keys: vec![(0.0, 0.5)],
        twinkle_percent: 1.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        ..ParticleEmitter::default()
    }
}

/// Billboards span +-scale along the camera right/up axes (`BuildQuadT3` corners
/// m0 x +-1 + m1 x +-1) and carry the lifetime colour and opacity.
#[test]
fn billboard_quads_face_the_camera_with_their_lifetime_colour() {
    let quad = one_quad(&still_emitter(0), Mat4::IDENTITY);
    assert_eq!(quad.axis_x, Vec3::new(0.5, 0.0, 0.0));
    assert_eq!(quad.axis_y, Vec3::new(0.0, 0.25, 0.0));
    assert!(close(quad.color.x, 1.0) && close(quad.color.y, 128.0 / 255.0) && quad.color.z == 0.0);
    assert!(close(quad.color.w, 0.5));
    assert_eq!(quad.cell_offset, Vec2::ZERO);
}

/// Model-space (0x10) particles are drawn through the emitter matrix, and 0x1000
/// orients the quad in the emitter's XY plane instead of facing the camera.
#[test]
fn model_space_particles_follow_the_emitter_and_local_orientation_uses_its_axes() {
    let turned = Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0))
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let quad = one_quad(&still_emitter(0x10 | 0x1000), turned);
    assert!(
        quad.center.distance(Vec3::new(0.0, 3.0, 0.0)) < 1e-4,
        "{quad:?}"
    );
    assert!(quad.axis_x.distance(Vec3::new(0.5, 0.0, 0.0)) < 1e-5);
    assert!(
        quad.axis_y.distance(Vec3::new(0.0, 0.0, 0.25)) < 1e-5,
        "{quad:?}"
    );
}

#[test]
fn blend_modes_write_depth_and_alpha_test_as_authored() {
    let tests: Vec<(bool, f32)> = (0..8u8)
        .map(|blend| {
            (
                m2_particles::writes_depth(blend),
                m2_particles::alpha_test(blend),
            )
        })
        .collect();
    assert_eq!(tests[0], (true, -1.0));
    assert_eq!(tests[1], (true, 0.501_960_8));
    for &(depth, test) in &tests[2..] {
        assert_eq!((depth, test), (false, 0.003_921_569));
    }
}

/// Battle Shout's buff model (6194303) authors its bursts as keyframed tracks: in its
/// Stand (birth) sequence emitter 0 is disabled until 133 ms and emits 8/s from 33 to
/// 433 ms, nothing from 500 ms; its Hold sequence (158) emits a steady 8/s.
#[test]
fn keyframed_emission_follows_the_playing_sequence_time() {
    let emitter = &model(6194303).particle_emitters[0];
    assert_eq!(emitter.emission_rate, 0.0, "the static first key");
    let emitted = |sequence: usize, time_ms: u32| {
        let mut sim = EmitterSim::new(emitter, 3);
        sim.set_animation(
            emitter,
            &m2::AnimTime {
                sequence,
                time_ms,
                global_ms: 0.0,
                global_sequences: &[],
            },
        );
        for _ in 0..30 {
            sim.update(emitter, 1.0 / 60.0, Mat4::IDENTITY, wow_to_godot(), 1.0);
        }
        sim.particles().len()
    };
    // Disabled at 100 ms, emitting at 300 ms, silent at 700 ms of the birth.
    assert_eq!(emitted(0, 100), 0);
    assert!(emitted(0, 300) >= 2, "{}", emitted(0, 300));
    assert_eq!(emitted(0, 700), 0);
    assert!(emitted(1, 0) >= 2);
    // Without an animation time the static value (0/s) applies.
    let mut sim = EmitterSim::new(emitter, 3);
    for _ in 0..30 {
        sim.update(emitter, 1.0 / 60.0, Mat4::IDENTITY, wow_to_godot(), 1.0);
    }
    assert_eq!(sim.particles().len(), 0);
}

/// world/unk_exp11_6323400/6323400.m2 (one sequence, global sequences 50000/5333/14667 ms)
/// drives emitter 0's rate on global sequence 1 (5333 ms, timeline 0): 149.13/s from
/// 3167 to 3300 ms, 10/s from 833 to 1233 ms; particles live 0.3 s. The rate follows the
/// model's global clock whatever the sequence time (WebWowViewerCpp animationManager.cpp
/// `calcParticleEmitters` → `animateTrackWithBlend` with `globalSequenceTimes`).
#[test]
fn global_sequence_emission_rate_follows_the_global_clock() {
    let model = model(6323400);
    let emitter = &model.particle_emitters[0];
    let rate = emitter
        .tracks
        .emission_rate
        .as_ref()
        .expect("keyframed rate");
    assert_eq!(rate.global_sequence, 1);
    assert_eq!(model.global_sequences[1], 5333);
    let alive = |global_ms: f64| {
        let mut sim = EmitterSim::new(emitter, 3);
        sim.set_animation(
            emitter,
            &m2::AnimTime {
                sequence: 0,
                time_ms: 0,
                global_ms,
                global_sequences: &model.global_sequences,
            },
        );
        for _ in 0..30 {
            sim.update(emitter, 1.0 / 60.0, Mat4::IDENTITY, wow_to_godot(), 1.0);
        }
        sim.particles().len()
    };
    // About 149 × 0.3 alive in the burst, 10 × 0.3 in the lull, the burst again a
    // whole global loop later.
    assert!(alive(3200.0) >= 30, "burst: {}", alive(3200.0));
    assert!(alive(1000.0) <= 6, "lull: {}", alive(1000.0));
    assert!(alive(3200.0 + 3.0 * 5333.0) >= 30, "wrapped burst");
}
