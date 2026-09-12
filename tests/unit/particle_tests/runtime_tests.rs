use super::*;

fn model_particle_test_app() -> App {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Mesh>>();
    app.world_mut().init_resource::<Assets<StandardMaterial>>();
    app.world_mut().init_resource::<Assets<M2EffectMaterial>>();
    app.world_mut().init_resource::<Assets<Image>>();
    app.world_mut()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.world_mut().insert_resource(CreatureDisplayMap);
    app.world_mut().insert_resource(Time::<()>::default());
    app
}

#[test]
fn graphics_config_particles_control_spawn_and_texture_loading() {
    for enabled in [false, true] {
        let mut app = model_particle_test_app();
        app.insert_resource(GraphicsOptions {
            particle_effects_enabled: enabled,
            ..Default::default()
        });
        let transform = Transform::from_xyz(2.0, 3.0, 4.0);
        let parent = app
            .world_mut()
            .spawn((transform, GlobalTransform::default()))
            .id();
        let mut emitter = sample_emitter();
        emitter.texture_fdid = Some(u32::MAX);
        app.world_mut()
            .run_system_once(move |mut commands: bevy::prelude::Commands| {
                spawn_emitters(&mut commands, &[emitter.clone()], &[], None, parent);
            })
            .expect("spawn system should run");
        app.world_mut().flush();
        let expected = usize::from(enabled);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), expected);
        assert_eq!(
            app.world_mut()
                .query::<&ParticleEmitterComp>()
                .iter(app.world())
                .count(),
            expected
        );
        assert_eq!(app.world().get::<Transform>(parent), Some(&transform));
        if enabled {
            let mut query = app
                .world_mut()
                .query::<(&ParticleEmitterComp, &bevy::prelude::ChildOf)>();
            let (emitter, child_of) = query.single(app.world()).expect("one emitter");
            assert_eq!(child_of.parent(), parent);
            assert_eq!(emitter.spawn_mode, ParticleSpawnMode::Continuous);
            assert!(!emitter.pending_textures.is_empty());
        }
    }
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
    emitter.flags = super::super::PARTICLE_FLAG_PROJECT_PARTICLE;
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
    emitter.flags = super::super::PARTICLE_FLAG_PROJECT_PARTICLE;
    let comp = ParticleEmitterComp {
        emitter,
        bone_entity: None,
        scale_source: entity,
        spawn_mode: ParticleSpawnMode::Continuous,
        spawn_source: ParticleSpawnSource::Standalone,
        child_emitters: Vec::new(),
        effect_parent: None,
        pending_textures: Vec::new(),
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
    assert_eq!(
        emitter_scale_source(&emitter, bone, parent),
        Entity::from_bits(22)
    );

    emitter.flags = PARTICLE_FLAG_WORLD_SPACE | PARTICLE_FLAG_BONE_SCALE;
    assert!(!emitter_uses_bone_scale(&emitter));
    assert_eq!(emitter_scale_source(&emitter, bone, parent), parent);
}

#[test]
fn follow_position_emitters_use_local_simulation_space() {
    let mut emitter = sample_emitter();
    emitter.flags = super::super::PARTICLE_FLAG_FOLLOW_POSITION;

    assert!(emitter_uses_follow_position(&emitter));
    assert_eq!(emitter_simulation_space(&emitter), SimulationSpace::Local);

    let effect = build_effect_asset(&emitter, 1.0, 1.0);
    assert_eq!(effect.simulation_space, SimulationSpace::Local);
}

#[test]
fn inherit_position_flag_is_detected_separately_from_follow_position() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_INHERIT_POSITION;

    assert!(emitter_uses_inherit_position(&emitter));
    assert!(!emitter_uses_follow_position(&emitter));

    let effect = build_effect_asset(&emitter, 1.0, 1.0);
    assert!(
        effect
            .properties()
            .iter()
            .any(|property| property.name() == "inherit_position_back_delta")
    );
}

#[test]
fn inherit_velocity_flag_is_detected_for_child_emitters() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_INHERIT_VELOCITY;

    assert!(emitter_uses_inherit_velocity(&emitter));
}

#[test]
fn particle_model_filename_marks_model_particle_emitters() {
    let mut emitter = sample_emitter();
    emitter.particle_model_filename = Some("spells/torch_model_particle.m2".to_string());

    assert!(emitter_uses_model_particles(&emitter));
}

#[test]
fn model_particle_emitters_skip_hanabi_quad_spawn_path() {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Image>>();
    let parent = app.world_mut().spawn_empty().id();
    let mut emitter = sample_emitter();
    emitter.particle_model_filename = Some("spells/torch_model_particle.m2".to_string());
    let emitters = vec![emitter];

    app.world_mut()
        .run_system_once(move |mut commands: bevy::prelude::Commands| {
            spawn_emitters(&mut commands, &emitters, &[], None, parent);
        })
        .expect("spawn system should run");
    app.world_mut().flush();

    assert_eq!(
        app.world_mut()
            .query::<&ParticleEmitterComp>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&ModelParticleEmitterComp>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn burst_emitters_spawn_with_burst_once_mode() {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Image>>();
    let parent = app.world_mut().spawn_empty().id();
    let emitters = vec![sample_emitter(), sample_emitter()];
    let expected_count = emitters.len();

    app.world_mut()
        .run_system_once(
            move |mut commands: bevy::prelude::Commands,
                  mut images: bevy::prelude::ResMut<Assets<Image>>| {
                spawn_emitters_with_mode(
                    &mut commands,
                    &mut images,
                    &emitters,
                    &[],
                    None,
                    parent,
                    ParticleSpawnMode::BurstOnce,
                );
            },
        )
        .expect("burst emitter spawn system should run");
    app.world_mut().flush();

    let spawn_modes: Vec<ParticleSpawnMode> = app
        .world_mut()
        .query::<&ParticleEmitterComp>()
        .iter(app.world())
        .map(|comp| comp.spawn_mode)
        .collect();
    assert_eq!(spawn_modes.len(), expected_count);
    assert!(
        spawn_modes
            .iter()
            .all(|mode| *mode == ParticleSpawnMode::BurstOnce)
    );
}

#[test]
fn child_model_particle_emitters_use_child_spawn_source() {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Image>>();
    let parent = app.world_mut().spawn_empty().id();
    let mut child_emitter = sample_emitter();
    child_emitter.particle_model_filename = Some("data/models/club_1h_torch_a_01.m2".to_string());
    let child_emitters = vec![child_emitter];

    app.world_mut()
        .run_system_once(
            move |mut commands: bevy::prelude::Commands,
                  mut images: bevy::prelude::ResMut<Assets<Image>>| {
                spawn_loaded_child_emitters(
                    &mut commands,
                    &mut images,
                    &child_emitters,
                    &[],
                    parent,
                    parent,
                );
            },
        )
        .expect("child emitter spawn system should run");
    app.world_mut().flush();

    let spawns: Vec<ParticleSpawnSource> = app
        .world_mut()
        .query::<&ModelParticleEmitterComp>()
        .iter(app.world())
        .map(|comp| comp.spawn_source)
        .collect();

    assert_eq!(spawns, vec![ParticleSpawnSource::ChildFromParentParticles]);
}

#[test]
fn continuous_model_particle_spawner_accumulates_fractional_rate() {
    let emitter = sample_emitter();
    let mut runtime = ModelParticleEmitterRuntime::default();

    let first = model_particle_spawn_count(
        &emitter,
        ParticleSpawnMode::Continuous,
        1.0,
        0.01,
        &mut runtime,
    );
    let second = model_particle_spawn_count(
        &emitter,
        ParticleSpawnMode::Continuous,
        1.0,
        0.05,
        &mut runtime,
    );

    assert_eq!(first, 0);
    assert_eq!(second, 1);
}

#[test]
fn burst_model_particle_spawner_fires_only_once() {
    let emitter = sample_emitter();
    let mut runtime = ModelParticleEmitterRuntime::default();

    let first = model_particle_spawn_count(
        &emitter,
        ParticleSpawnMode::BurstOnce,
        1.0,
        0.0,
        &mut runtime,
    );
    let second = model_particle_spawn_count(
        &emitter,
        ParticleSpawnMode::BurstOnce,
        1.0,
        0.0,
        &mut runtime,
    );

    assert_eq!(first, 20);
    assert_eq!(second, 0);
}

#[test]
fn model_particle_runtime_spawns_static_m2_instance() {
    let mut app = model_particle_test_app();
    let emitter = {
        let mut emitter = sample_emitter();
        emitter.particle_model_filename = Some("data/models/club_1h_torch_a_01.m2".to_string());
        emitter
    };
    let emitter_entity = app
        .world_mut()
        .spawn((
            GlobalTransform::IDENTITY,
            Transform::IDENTITY,
            ModelParticleEmitterComp {
                emitter,
                bone_entity: None,
                scale_source: Entity::PLACEHOLDER,
                spawn_mode: ParticleSpawnMode::BurstOnce,
                spawn_source: ParticleSpawnSource::Standalone,
                requested_model_path: "data/models/club_1h_torch_a_01.m2".to_string(),
                resolved_model_path: Some("data/models/club_1h_torch_a_01.m2".into()),
            },
            ModelParticleEmitterRuntime::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(emitter_entity)
        .get_mut::<ModelParticleEmitterComp>()
        .unwrap()
        .scale_source = emitter_entity;

    app.world_mut()
        .run_system_once(super::super::emitters::tick_model_particle_emitters)
        .expect("model particle tick should run");
    app.world_mut().flush();

    assert!(
        app.world_mut()
            .query::<&ModelParticleInstance>()
            .iter(app.world())
            .count()
            > 0
    );
}

#[test]
fn disabled_model_particle_emitter_is_not_ticked() {
    let mut app = model_particle_test_app();
    let emitter_entity = app
        .world_mut()
        .spawn((
            Disabled,
            GlobalTransform::IDENTITY,
            Transform::IDENTITY,
            ModelParticleEmitterComp {
                emitter: sample_emitter(),
                bone_entity: None,
                scale_source: Entity::PLACEHOLDER,
                spawn_mode: ParticleSpawnMode::BurstOnce,
                spawn_source: ParticleSpawnSource::Standalone,
                requested_model_path: "unused.m2".to_string(),
                resolved_model_path: Some("unused.m2".into()),
            },
            ModelParticleEmitterRuntime::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(emitter_entity)
        .get_mut::<ModelParticleEmitterComp>()
        .unwrap()
        .scale_source = emitter_entity;

    app.world_mut()
        .run_system_once(super::super::emitters::tick_model_particle_emitters)
        .expect("model particle tick should run");
    app.world_mut().flush();

    let runtime = app
        .world()
        .entity(emitter_entity)
        .get::<ModelParticleEmitterRuntime>()
        .expect("emitter runtime should remain present");
    assert!(!runtime.burst_fired);
    assert_eq!(runtime.spawn_serial, 0);
    assert_eq!(
        app.world_mut()
            .query::<&ModelParticleInstance>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn disabled_hanabi_emitter_is_not_registered() {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<EffectAsset>>();
    let emitter_entity = app
        .world_mut()
        .spawn((Disabled, Transform::IDENTITY, GlobalTransform::IDENTITY))
        .id();
    app.world_mut()
        .entity_mut(emitter_entity)
        .insert(ParticleEmitterComp {
            emitter: sample_emitter(),
            bone_entity: None,
            scale_source: emitter_entity,
            spawn_mode: ParticleSpawnMode::Continuous,
            spawn_source: ParticleSpawnSource::Standalone,
            child_emitters: Vec::new(),
            effect_parent: None,
            pending_textures: Vec::new(),
        });

    app.world_mut()
        .run_system_once(super::super::emitters::register_pending_particle_effects)
        .expect("particle registration should run");
    app.world_mut().flush();

    assert!(
        app.world()
            .entity(emitter_entity)
            .get::<ParticleEffect>()
            .is_none()
    );
    assert!(app.world().resource::<Assets<EffectAsset>>().is_empty());
}

#[test]
fn inherit_position_back_delta_maps_world_segment_into_local_space() {
    let global = GlobalTransform::from(
        Transform::from_translation(Vec3::new(10.0, 5.0, -3.0))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
    );
    let previous = Vec3::new(8.0, 5.0, -3.0);
    let current = Vec3::new(10.0, 5.0, -3.0);

    let back_delta = inherit_position_back_delta_local(previous, current, &global);

    assert!((back_delta - Vec3::new(0.0, 0.0, -2.0)).length() < 0.0001);
}

#[test]
fn dynamic_wind_flag_uses_dynamic_wind_path() {
    let mut emitter = sample_emitter();
    emitter.flags = PARTICLE_FLAG_WIND_DYNAMIC;

    assert!(emitter_uses_dynamic_wind(&emitter));
}

#[test]
fn sync_dynamic_wind_properties_updates_effect_property() {
    let mut app = App::new();
    app.insert_resource(DynamicParticleWind {
        effect_space_accel: Vec3::new(1.0, 2.0, 3.0),
    });
    let entity = app
        .world_mut()
        .spawn((
            ParticleEmitterComp {
                emitter: {
                    let mut emitter = sample_emitter();
                    emitter.flags = PARTICLE_FLAG_WIND_DYNAMIC;
                    emitter
                },
                bone_entity: None,
                scale_source: Entity::PLACEHOLDER,
                spawn_mode: ParticleSpawnMode::Continuous,
                spawn_source: ParticleSpawnSource::Standalone,
                child_emitters: Vec::new(),
                effect_parent: None,
                pending_textures: Vec::new(),
            },
            EffectProperties::default()
                .with_properties([(DYNAMIC_WIND_ACCEL_PROPERTY.to_string(), Vec3::ZERO.into())]),
        ))
        .id();

    let _ = app
        .world_mut()
        .run_system_once(sync_dynamic_wind_properties);

    let properties = app
        .world()
        .entity(entity)
        .get::<EffectProperties>()
        .unwrap();
    assert_eq!(
        properties.get_stored(DYNAMIC_WIND_ACCEL_PROPERTY),
        Some(Value::from(Vec3::new(1.0, 2.0, 3.0)))
    );
}
