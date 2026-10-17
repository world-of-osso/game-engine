use super::*;

fn emitter(lifespan: f32) -> ParticleEmitter {
    ParticleEmitter {
        flags: FLAG_HEAD,
        emitter_type: 1,
        emission_rate: 100.0,
        lifespan,
        twinkle_percent: 1.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        ..Default::default()
    }
}

fn particle(x: f32, age: f32) -> Particle {
    Particle {
        position: Vec3::X * x,
        previous_position: Vec3::X * x,
        velocity: Vec3::X * 2.0,
        orientation: Quat::IDENTITY,
        age,
        seed: 0,
        tex_pos: [Vec2::ZERO; 2],
        tex_vel: [Vec2::ZERO; 2],
    }
}

fn family(parents: usize) -> (ParticleEmitter, EmitterSim) {
    let parent = emitter(1.0);
    let mut root = EmitterSim::new(&parent, 17);
    root.props.emission_rate = 0.0;
    root.particles = (0..parents)
        .map(|i| particle(10.0 + i as f32 * 10.0, 0.2))
        .collect();
    let child = emitter(2.0);
    let mut child_sim = EmitterSim::new(&child, 19);
    child_sim.particles.push(particle(50.0, 0.5));
    root.children.push(RecursiveEmitter {
        emitter: child,
        sim: child_sim,
        interpolate_position: false,
    });
    (parent, root)
}

fn tick(emitter: &ParticleEmitter, sim: &mut EmitterSim, dt: f32) {
    sim.update(
        emitter,
        dt,
        Mat4::IDENTITY,
        Mat3::IDENTITY,
        1.0,
        &mut |_, _| None,
    );
}

#[test]
fn p5_one_and_two_parents_emit_at_moved_positions_child_ages_once() {
    for count in [1, 2] {
        let (emitter, mut root) = family(count);
        tick(&emitter, &mut root, 0.05);
        let children = root.children[0].sim.particles();
        assert!(
            (children[0].age - 0.55).abs() < 1e-6,
            "tagged child advances once with {count} parents"
        );
        assert!(
            children.len() > count,
            "surviving parents must supply births"
        );
        for i in 0..count {
            let x = 10.1 + i as f32 * 10.0;
            assert!(
                children
                    .iter()
                    .skip(1)
                    .any(|p| (p.position.x - x).abs() < 1e-5),
                "no child at moved parent {x}"
            );
        }
        assert!(
            children.iter().skip(1).all(|p| p.position.x > 9.0),
            "no births at root origin"
        );
    }
}

#[test]
fn p5_exact_parent_death_stops_births_but_not_existing_children() {
    let (mut emitter, mut root) = family(1);
    emitter.lifespan = 0.25;
    root.props.lifespan = 0.25;
    tick(&emitter, &mut root, 0.05);
    assert!(root.particles().is_empty());
    let children = root.children[0].sim.particles();
    assert_eq!(children.len(), 1, "dead parent may not supply births");
    assert!((children[0].age - 0.55).abs() < 1e-6);
}

#[test]
fn p5_descendants_drain_and_zero_duration_is_inert() {
    let (mut emitter, mut root) = family(1);
    emitter.emission_rate = 0.0;
    root.props.emission_rate = 0.0;
    root.children[0].emitter.emission_rate = 0.0;
    root.children[0].sim.props.emission_rate = 0.0;
    let grandchild = emitter.clone();
    let mut grandchild_sim = EmitterSim::new(&grandchild, 21);
    grandchild_sim.particles.push(particle(80.0, 0.1));
    root.children[0].sim.children.push(RecursiveEmitter {
        emitter: grandchild,
        sim: grandchild_sim,
        interpolate_position: false,
    });
    let before = root.particles().to_vec();
    let child_before = root.children[0].sim.particles().to_vec();
    tick(&emitter, &mut root, 0.0);
    assert_eq!(root.particles(), before);
    assert_eq!(root.children[0].sim.particles(), child_before);
    for _ in 0..30 {
        tick(&emitter, &mut root, 0.1);
    }
    assert!(root.particles().is_empty());
    assert!(
        root.children[0].sim.particles().is_empty(),
        "orphan child must drain"
    );
    assert!(
        root.children[0].sim.children[0].sim.particles().is_empty(),
        "orphan grandchild must drain"
    );
}

#[test]
fn p5_recursive_inherited_velocity_comes_from_parent_particle() {
    let (emitter, mut root) = family(1);
    root.children[0].emitter.flags |= 0x40;
    root.children[0].emitter.inherit_velocity_scale = 3.0;
    tick(&emitter, &mut root, 0.05);
    let children = root.children[0].sim.particles();
    assert!(children.len() > 1);
    for p in &children[1..] {
        assert!((p.velocity.x - 6.0).abs() < 1e-5);
    }
}

#[test]
fn p5_geometry_has_independent_lifetime_transform_color_and_opacity() {
    let mut e = emitter(1.0);
    e.particle_model_filename = Some("synthetic/arrow.m2".into());
    e.spin = 2.0;
    e.scale_keys = vec![(0.0, [1.0, 2.0]), (1.0, [3.0, 4.0])];
    e.color_keys = vec![(0.0, [255.0, 0.0, 0.0]), (1.0, [0.0, 0.0, 255.0])];
    e.opacity_keys = vec![(0.0, 1.0), (1.0, 0.0)];
    let mut sim = EmitterSim::new(&e, 17);
    sim.props.emission_rate = 0.0;
    sim.particles = vec![particle(10.0, 0.2), particle(20.0, 0.6)];
    tick(&e, &mut sim, 0.05);
    let mut instances = Vec::new();
    sim.model_particles(&e, &mut instances);
    for (i, age) in [0.25_f32, 0.65].into_iter().enumerate() {
        let (scale, orientation, position) = instances[i].transform.to_scale_rotation_translation();
        assert!((position.x - (10.1 + i as f32 * 10.0)).abs() < 1e-5);
        assert!(
            (scale - Vec3::new(1.0 + 2.0 * age, 2.0 + 2.0 * age, 1.0 + 2.0 * age)).length() < 1e-5
        );
        assert!(orientation.abs_diff_eq(Quat::from_rotation_z(2.0 * age), 1e-5));
        assert!((instances[i].color - Vec4::new(1.0 - age, 0.0, age, 1.0 - age)).length() < 1e-5);
    }
}

#[test]
fn p5_interpolated_children_receive_previous_parent_position() {
    let (e, mut root) = family(1);
    root.children[0].interpolate_position = true;
    tick(&e, &mut root, 0.05);
    let births = &root.children[0].sim.particles()[1..];
    assert!(!births.is_empty());
    assert!(
        births
            .iter()
            .all(|p| p.position.x >= 10.0 && p.position.x <= 10.1)
    );
    assert!(births.iter().any(|p| p.position.x < 10.099));
}

#[test]
fn p5_replay_is_per_substep_not_per_parent_or_frame() {
    let (e, mut combined) = family(2);
    let mut split = combined.clone();
    tick(&e, &mut combined, 0.25);
    tick(&e, &mut split, 0.1);
    tick(&e, &mut split, 0.1);
    tick(&e, &mut split, 0.05);
    assert_eq!(combined.particles().len(), split.particles().len());
    let actual = combined.children[0].sim.particles();
    let expected = split.children[0].sim.particles();
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.iter().zip(expected) {
        assert!((a.age - b.age).abs() < 1e-6);
        assert!((a.position - b.position).length() < 1e-5);
    }
    assert!((actual[0].age - 0.75).abs() < 1e-6);
}
