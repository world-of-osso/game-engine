use bevy::light::EnvironmentMapLight;

use super::*;
use crate::camera::WowCamera;
use crate::game::inworld_scene_stage::InWorldSceneStage;

#[test]
fn inworld_camera_gets_current_generated_ibl_at_fixed_time() {
    let mut app = lighting_app();
    let camera = spawn_world_camera(&mut app, true);
    assert!(
        app.world()
            .get::<GeneratedEnvironmentMapLight>(camera)
            .is_none()
    );
    assert!(!app.world().contains_resource::<SkyEnvMapHandle>());

    app.update();

    assert_current_ibl(&app, camera);
    assert_eq!(app.world().resource::<GlobalAmbientLight>().brightness, 0.0);
    assert!(app.world().get::<DistanceFog>(camera).is_none());
}

#[test]
fn inworld_ibl_initializes_late_active_camera_only_once() {
    let mut app = lighting_app();
    app.update();
    let camera = spawn_world_camera(&mut app, false);
    app.update();
    assert!(
        app.world()
            .get::<GeneratedEnvironmentMapLight>(camera)
            .is_none()
    );

    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = true;
    app.update();
    assert_current_ibl(&app, camera);
    let handle = app.world().resource::<SkyEnvMapHandle>().0.clone();
    let image_count = app.world().resource::<Assets<Image>>().len();

    for _ in 0..3 {
        app.update();
    }

    assert_eq!(app.world().resource::<GameTime>().minutes, 1440.0);
    assert_eq!(app.world().resource::<SkyEnvMapHandle>().0, handle);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
    assert_eq!(
        app.world()
            .get::<GeneratedEnvironmentMapLight>(camera)
            .unwrap()
            .environment_map,
        handle,
    );
}

#[test]
fn disabled_skybox_visuals_preserve_inworld_ibl() {
    let mut app = lighting_app();
    app.insert_resource(SkyboxVisualsDisabled);
    app.add_systems(PostUpdate, remove_disabled_sky_domes);
    let camera = spawn_world_camera(&mut app, true);
    let dome = app.world_mut().spawn(SkyDome).id();

    app.update();

    assert_current_ibl(&app, camera);
    assert!(app.world().get_entity(dome).is_err());
    let mut domes = app.world_mut().query_filtered::<Entity, With<SkyDome>>();
    assert_eq!(domes.iter(app.world()).count(), 0);
}

#[test]
fn inworld_ibl_preserves_camera_environment_overrides() {
    let mut app = lighting_app();
    let custom_map = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(build_sky_cubemap(&default_sky_colors()));
    let generated_camera = spawn_world_camera(&mut app, true);
    let rotation = Quat::from_rotation_y(0.35);
    app.world_mut()
        .entity_mut(generated_camera)
        .insert(GeneratedEnvironmentMapLight {
            environment_map: custom_map.clone(),
            intensity: 19.0,
            rotation,
            affects_lightmapped_mesh_diffuse: false,
        });
    let baked_camera = spawn_world_camera(&mut app, true);
    app.world_mut()
        .entity_mut(baked_camera)
        .insert(EnvironmentMapLight {
            diffuse_map: custom_map.clone(),
            specular_map: custom_map.clone(),
            intensity: 23.0,
            rotation,
            affects_lightmapped_mesh_diffuse: false,
        });
    let unrelated_camera = app.world_mut().spawn(Camera3d::default()).id();
    let image_count = app.world().resource::<Assets<Image>>().len();

    app.update();

    let generated = app
        .world()
        .get::<GeneratedEnvironmentMapLight>(generated_camera)
        .unwrap();
    assert_eq!(generated.environment_map, custom_map);
    assert_eq!(generated.intensity, 19.0);
    assert_eq!(generated.rotation, rotation);
    assert!(!generated.affects_lightmapped_mesh_diffuse);
    let baked = app
        .world()
        .get::<EnvironmentMapLight>(baked_camera)
        .unwrap();
    assert_eq!(baked.diffuse_map, custom_map);
    assert_eq!(baked.specular_map, custom_map);
    assert_eq!(baked.intensity, 23.0);
    assert_eq!(baked.rotation, rotation);
    assert!(!baked.affects_lightmapped_mesh_diffuse);
    assert!(
        app.world()
            .get::<GeneratedEnvironmentMapLight>(baked_camera)
            .is_none()
    );
    assert!(
        app.world()
            .get::<GeneratedEnvironmentMapLight>(unrelated_camera)
            .is_none()
    );
    assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
    assert!(!app.world().contains_resource::<SkyEnvMapHandle>());
}

#[test]
fn inworld_ibl_respects_scene_state_and_lighting_stage() {
    for (state, stage) in [
        (GameState::CharSelect, InWorldSceneStage::Ui),
        (GameState::InWorld, InWorldSceneStage::Character),
    ] {
        let mut app = lighting_app();
        app.insert_resource(State::new(state));
        app.insert_resource(stage);
        let camera = spawn_world_camera(&mut app, true);
        app.update();
        assert!(
            app.world()
                .get::<GeneratedEnvironmentMapLight>(camera)
                .is_none()
        );
        assert!(!app.world().contains_resource::<SkyEnvMapHandle>());
    }
}

fn lighting_app() -> App {
    let mut app = App::new();
    app.insert_resource(State::new(GameState::InWorld));
    app.insert_resource(GameTime::default());
    app.insert_resource(LightKeyframes(vec![
        light_row(0.0, Color::linear_rgb(0.1, 0.2, 0.3)),
        light_row(2880.0, Color::linear_rgb(0.5, 0.6, 0.7)),
    ]));
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<SkyMaterial>::default());
    app.insert_resource(Assets::<crate::water_material::WaterMaterial>::default());
    app.insert_resource(GlobalAmbientLight {
        brightness: 0.0,
        ..default()
    });
    register_shared_sky_visual_systems(&mut app);
    app
}

fn spawn_world_camera(app: &mut App, active: bool) -> Entity {
    app.world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                is_active: active,
                ..default()
            },
            WowCamera::default(),
        ))
        .id()
}

fn assert_current_ibl(app: &App, camera: Entity) {
    let world = app.world();
    let light = world
        .get::<GeneratedEnvironmentMapLight>(camera)
        .expect("active InWorld camera must have generated environment lighting");
    assert_eq!(light.intensity, 300.0);
    assert_eq!(light.rotation, Quat::IDENTITY);
    assert!(light.affects_lightmapped_mesh_diffuse);
    assert_eq!(world.resource::<SkyEnvMapHandle>().0, light.environment_map);
    let image = world
        .resource::<Assets<Image>>()
        .get(&light.environment_map)
        .expect("generated lighting source cubemap must exist");
    assert_valid_ibl_cube(image);
    let colors = interpolate_colors(
        &world.resource::<LightKeyframes>().0,
        world.resource::<GameTime>().minutes,
    );
    let expected = build_sky_cubemap(&colors);
    assert_eq!(image.data, expected.data);
    assert_ne!(image.data, build_sky_cubemap(&default_sky_colors()).data);
}

fn assert_valid_ibl_cube(image: &Image) {
    assert_eq!(
        image.texture_descriptor.size,
        Extent3d {
            width: ENV_MAP_SIZE,
            height: ENV_MAP_SIZE,
            depth_or_array_layers: 6,
        }
    );
    assert_eq!(image.texture_descriptor.dimension, TextureDimension::D2);
    assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba16Float);
    assert_eq!(
        image
            .texture_view_descriptor
            .as_ref()
            .and_then(|view| view.dimension),
        Some(TextureViewDimension::Cube)
    );
}

fn light_row(time: f32, tint: Color) -> LightDataRow {
    LightDataRow {
        time,
        direct_color: tint,
        ambient_color: tint,
        sky_top: tint,
        sky_middle: tint,
        sky_band1: tint,
        sky_band2: tint,
        sky_smog: tint,
        fog_color: tint,
        sun_color: tint,
        sun_halo_color: tint,
        cloud_emissive_color: tint,
        cloud_layer1_ambient_color: tint,
        cloud_layer2_ambient_color: tint,
        ocean_close_color: tint,
        ocean_far_color: tint,
        river_close_color: tint,
        river_far_color: tint,
        horizon_ambient_color: tint,
        fog_end: 1200.0,
        fog_start: 300.0,
        glow: 1.0,
        cloud_density: 0.0,
        unk1: 0.0,
        unk2: 0.0,
    }
}
