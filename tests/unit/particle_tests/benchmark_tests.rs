use super::*;

#[test]
#[ignore = "benchmark-style integration test; run explicitly"]
fn bench_particle_heavy_scene_headless() {
    const PARTICLE_HEAVY_SCENE_P99_BUDGET_MS: f64 = 2.0;
    let Some(model) = benchmark_particle_model() else {
        println!("Skipping particle-heavy benchmark: no multi-emitter benchmark model found");
        return;
    };
    let emitters = model.particle_emitters;
    let bones = model.bones;
    assert!(
        emitters.len() >= 3,
        "expected multi-emitter benchmark model, got {} emitters",
        emitters.len()
    );

    let roots = 12_usize;
    let cycles = 120_usize;
    let total_emitters = roots * emitters.len();
    let mut app = game_engine::test_harness::headless_app_with(|app| {
        app.add_plugins(bevy::transform::TransformPlugin);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<M2EffectMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        app.init_resource::<CreatureDisplayMap>();
        app.init_resource::<Assets<bevy_hanabi::EffectAsset>>();
        app.insert_resource(DynamicParticleWind::default());
        app.add_systems(
            Update,
            (
                super::super::emitters::register_pending_particle_effects,
                super::super::emitters::sync_inherit_position_properties,
                super::super::emitters::sync_dynamic_wind_properties,
                super::super::emitters::trigger_pending_particle_bursts,
                super::super::emitters::tick_model_particle_emitters,
                super::super::emitters::simulate_model_particle_instances,
            ),
        );
    });

    for i in 0..roots {
        let parent = app
            .world_mut()
            .spawn((
                Transform::from_xyz(i as f32 * 2.0, 0.0, 0.0),
                GlobalTransform::from_translation(Vec3::new(i as f32 * 2.0, 0.0, 0.0)),
            ))
            .id();
        let emitters = emitters.clone();
        let bones = bones.clone();
        app.world_mut()
            .run_system_once(
                move |mut commands: bevy::prelude::Commands,
                      mut images: bevy::prelude::ResMut<Assets<Image>>| {
                    spawn_emitters(&mut commands, &mut images, &emitters, &bones, None, parent);
                },
            )
            .expect("spawn system should run");
        app.world_mut().flush();
    }

    let registered_before = app
        .world_mut()
        .query::<&ParticleEmitterComp>()
        .iter(app.world())
        .count();
    assert_eq!(registered_before, total_emitters);

    let mut frame_samples = Vec::with_capacity(cycles);
    for _ in 0..cycles {
        let start = Instant::now();
        app.update();
        frame_samples.push(start.elapsed());
    }
    let elapsed: std::time::Duration = frame_samples.iter().copied().sum();
    let p99 = game_engine::test_harness::p99_duration(&frame_samples).expect("frame samples");

    let active_effects = app
        .world_mut()
        .query::<&bevy_hanabi::ParticleEffect>()
        .iter(app.world())
        .count();
    assert_eq!(active_effects, total_emitters);

    println!(
        "particle_heavy_scene_headless roots={} emitters_per_root={} total_emitters={} cycles={} total_ms={:.2} avg_frame_ms={:.2} p99_ms={:.2}",
        roots,
        emitters.len(),
        total_emitters,
        cycles,
        elapsed.as_secs_f64() * 1000.0,
        (elapsed.as_secs_f64() * 1000.0) / cycles as f64,
        p99.as_secs_f64() * 1000.0,
    );
    assert!(
        p99.as_secs_f64() * 1000.0 <= PARTICLE_HEAVY_SCENE_P99_BUDGET_MS,
        "expected particle-heavy scene p99 <= {PARTICLE_HEAVY_SCENE_P99_BUDGET_MS:.2}ms, got {:.2}ms",
        p99.as_secs_f64() * 1000.0,
    );
}
