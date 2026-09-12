use super::*;
use crate::asset::m2_light::M2Light;

#[derive(Resource, Default)]
struct LightChanges {
    lights: usize,
    visibility: usize,
}

fn observe_changes(
    lights: Query<(), Changed<PointLight>>,
    visibility: Query<(), (With<RuntimeM2PointLight>, Changed<Visibility>)>,
    mut changes: ResMut<LightChanges>,
) {
    changes.lights = lights.iter().count();
    changes.visibility = visibility.iter().count();
}

fn track<T>(values: Vec<T>) -> AnimTrack<T> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0, 100], values)],
    }
}

fn authored_light() -> M2Light {
    M2Light {
        light_type: 1,
        bone_index: -1,
        position: [0.0; 3],
        ambient_color: track(vec![[0.0; 3]; 2]),
        ambient_intensity: track(vec![0.0; 2]),
        diffuse_color: track(vec![[1.0, 0.5, 0.25]; 2]),
        diffuse_intensity: track(vec![1.0; 2]),
        attenuation_start: track(vec![2.0; 2]),
        attenuation_end: track(vec![12.0; 2]),
        visibility: track(vec![1; 2]),
    }
}

fn light_app(light: M2Light) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<LightChanges>();
    app.init_resource::<Time>();
    app.add_systems(Update, sync_model_lights);
    app.add_systems(PostUpdate, observe_changes);
    let owner = app
        .world_mut()
        .spawn(M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        })
        .id();
    let entity = app
        .world_mut()
        .spawn((
            RuntimeM2PointLight {
                light,
                animation: crate::m2_spawn::M2LightAnimation::for_model(&[], &[], Some(owner)),
            },
            PointLight {
                shadow_maps_enabled: true,
                shadow_depth_bias: 0.123,
                shadow_normal_bias: 0.456,
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();
    (app, owner, entity)
}

fn assert_changes(app: &App, lights: usize, visibility: usize) {
    let changes = app.world().resource::<LightChanges>();
    assert_eq!(changes.lights, lights, "PointLight change count");
    assert_eq!(changes.visibility, visibility, "Visibility change count");
}

fn assert_light(app: &App, entity: Entity, color: Color, intensity: f32, range: f32, radius: f32) {
    let light = app.world().get::<PointLight>(entity).unwrap();
    assert_eq!(light.color, color);
    assert_eq!(light.intensity, intensity);
    assert_eq!(light.range, range);
    assert_eq!(light.radius, radius);
    assert!(light.shadow_maps_enabled);
    assert_eq!(light.shadow_depth_bias, 0.123);
    assert_eq!(light.shadow_normal_bias, 0.456);
}

#[test]
fn model_lights_unchanged_tracks_do_not_mark_components_changed() {
    let (mut app, owner, entity) = light_app(authored_light());
    app.update();
    assert_light(
        &app,
        entity,
        Color::linear_rgb(1.0, 0.5, 0.25),
        2000.0,
        12.0,
        2.0,
    );
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Inherited)
    );
    for time in [0.0, 50.0, 100.0] {
        app.world_mut()
            .get_mut::<M2AnimPlayer>(owner)
            .unwrap()
            .time_ms = time;
        app.update();
        assert_changes(&app, 0, 0);
    }
}

#[test]
fn model_lights_animated_tracks_update_owned_fields_and_visibility_independently() {
    let mut light = authored_light();
    light.diffuse_color = track(vec![[1.0, 0.5, 0.25], [0.25, 0.5, 1.0]]);
    light.diffuse_intensity = track(vec![1.0, 2.0]);
    light.attenuation_start = track(vec![2.0, 3.0]);
    light.attenuation_end = track(vec![12.0, 18.0]);
    light.visibility = track(vec![1, 0]);
    let (mut app, owner, entity) = light_app(light);
    app.update();
    app.world_mut()
        .get_mut::<M2AnimPlayer>(owner)
        .unwrap()
        .time_ms = 100.0;
    app.update();
    assert_changes(&app, 1, 1);
    assert_light(
        &app,
        entity,
        Color::linear_rgb(0.5, 1.0, 2.0),
        4000.0,
        18.0,
        3.0,
    );
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Hidden)
    );
    app.update();
    assert_changes(&app, 0, 0);
    app.world_mut()
        .get_mut::<RuntimeM2PointLight>(entity)
        .unwrap()
        .light
        .visibility = track(vec![1, 1]);
    app.update();
    assert_changes(&app, 0, 1);
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Inherited)
    );
    app.world_mut()
        .get_mut::<M2AnimPlayer>(owner)
        .unwrap()
        .time_ms = 0.0;
    app.update();
    assert_changes(&app, 1, 0);
    assert_light(
        &app,
        entity,
        Color::linear_rgb(1.0, 0.5, 0.25),
        2000.0,
        12.0,
        2.0,
    );
}
