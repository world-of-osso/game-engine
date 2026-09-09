use super::*;
use bevy::camera::RenderTargetInfo;

#[derive(Resource, Default, Debug, PartialEq)]
struct EffectChanges {
    bloom: usize,
    sharpening: usize,
    resolution: usize,
}

fn observe_changes(
    bloom: Query<(), Changed<Bloom>>,
    sharpening: Query<(), Changed<ContrastAdaptiveSharpening>>,
    resolution: Query<(), Changed<MainPassResolutionOverride>>,
    mut changes: ResMut<EffectChanges>,
) {
    *changes = EffectChanges {
        bloom: bloom.iter().count(),
        sharpening: sharpening.iter().count(),
        resolution: resolution.iter().count(),
    };
}

fn test_app() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(GraphicsOptions {
        bloom_enabled: true,
        bloom_intensity: 0.12,
        render_scale: 0.75,
        ..default()
    });
    app.init_resource::<EffectChanges>();
    app.add_systems(Update, sync_camera_graphics_post_process);
    app.add_systems(PostUpdate, observe_changes);
    let mut camera = Camera::default();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: UVec2::new(1600, 900),
        scale_factor: 1.0,
    });
    let entity = app.world_mut().spawn((Camera3d::default(), camera)).id();
    app.update();
    (app, entity)
}

fn assert_no_changes(app: &mut App) {
    app.update();
    assert_eq!(
        app.world().resource::<EffectChanges>(),
        &EffectChanges::default()
    );
}

#[test]
fn camera_unchanged_writes_keep_all_effect_ticks_stable() {
    let (mut app, entity) = test_app();
    assert_eq!(
        app.world().resource::<EffectChanges>(),
        &EffectChanges {
            bloom: 1,
            sharpening: 1,
            resolution: 1,
        }
    );
    assert_eq!(
        app.world()
            .get::<MainPassResolutionOverride>(entity)
            .unwrap()
            .0,
        UVec2::new(1200, 675)
    );
    assert_no_changes(&mut app);
    assert_no_changes(&mut app);
}

fn bloom_values(bloom: &Bloom) -> (f32, f32, f32, f32, f32, f32, BloomCompositeMode, u32, Vec2) {
    (
        bloom.intensity,
        bloom.low_frequency_boost,
        bloom.low_frequency_boost_curvature,
        bloom.high_pass_frequency,
        bloom.prefilter.threshold,
        bloom.prefilter.threshold_softness,
        bloom.composite_mode,
        bloom.max_mip_dimension,
        bloom.scale,
    )
}

#[test]
fn camera_unchanged_writes_restore_each_bloom_field() {
    let mutations: [fn(&mut Bloom); 10] = [
        |b| b.intensity = 0.9,
        |b| b.low_frequency_boost = 0.2,
        |b| b.low_frequency_boost_curvature = 0.1,
        |b| b.high_pass_frequency = 0.3,
        |b| b.prefilter.threshold = 0.4,
        |b| b.prefilter.threshold_softness = 0.5,
        |b| b.composite_mode = BloomCompositeMode::EnergyConserving,
        |b| b.max_mip_dimension = 128,
        |b| b.scale.x = 2.0,
        |b| b.scale.y = 3.0,
    ];
    for mutate in mutations {
        let (mut app, entity) = test_app();
        let expected = bloom_values(app.world().get::<Bloom>(entity).unwrap());
        // Model an external value without contributing our own Changed notification.
        mutate(
            app.world_mut()
                .get_mut::<Bloom>(entity)
                .unwrap()
                .bypass_change_detection(),
        );
        app.update();
        assert_eq!(
            bloom_values(app.world().get::<Bloom>(entity).unwrap()),
            expected
        );
        assert_eq!(app.world().resource::<EffectChanges>().bloom, 1);
        assert_no_changes(&mut app);
    }
}

#[test]
fn camera_unchanged_writes_restore_each_sharpening_field() {
    let mutations: [fn(&mut ContrastAdaptiveSharpening); 3] = [
        |cas| cas.enabled = false,
        |cas| cas.sharpening_strength = 0.2,
        |cas| cas.denoise = true,
    ];
    for mutate in mutations {
        let (mut app, entity) = test_app();
        mutate(
            app.world_mut()
                .get_mut::<ContrastAdaptiveSharpening>(entity)
                .unwrap()
                .bypass_change_detection(),
        );
        app.update();
        let cas = app
            .world()
            .get::<ContrastAdaptiveSharpening>(entity)
            .unwrap();
        assert!(cas.enabled);
        assert_eq!(cas.sharpening_strength, 0.6);
        assert!(!cas.denoise);
        assert_eq!(app.world().resource::<EffectChanges>().sharpening, 1);
        assert_no_changes(&mut app);
    }
}

#[test]
fn camera_unchanged_writes_apply_graphics_resize_and_removal() {
    let (mut app, entity) = test_app();
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .bloom_intensity = 0.3;
    app.update();
    assert_eq!(app.world().get::<Bloom>(entity).unwrap().intensity, 0.3);
    assert_eq!(app.world().resource::<EffectChanges>().bloom, 1);
    assert_no_changes(&mut app);

    app.world_mut()
        .get_mut::<Camera>(entity)
        .unwrap()
        .computed
        .target_info
        .as_mut()
        .unwrap()
        .physical_size = UVec2::new(1920, 1080);
    app.update();
    assert_eq!(
        app.world()
            .get::<MainPassResolutionOverride>(entity)
            .unwrap()
            .0,
        UVec2::new(1440, 810)
    );
    assert_eq!(app.world().resource::<EffectChanges>().resolution, 1);
    assert_no_changes(&mut app);

    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .render_scale = 0.5;
    app.update();
    assert_eq!(
        app.world()
            .get::<MainPassResolutionOverride>(entity)
            .unwrap()
            .0,
        UVec2::new(960, 540)
    );
    assert_no_changes(&mut app);

    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .bloom_enabled = false;
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .render_scale = 1.0;
    app.update();
    let camera = app.world().entity(entity);
    assert!(!camera.contains::<Bloom>());
    assert!(!camera.contains::<ContrastAdaptiveSharpening>());
    assert!(!camera.contains::<MainPassResolutionOverride>());
    assert!(camera.contains::<Camera3d>());
    assert_no_changes(&mut app);
}
