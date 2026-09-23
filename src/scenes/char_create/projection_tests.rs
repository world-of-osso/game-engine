use super::*;
use bevy::state::app::StatesPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use game_engine::asset::m2_format::m2_camera::parse_camera_snapshot;

fn projected_fov(app: &mut App) -> f32 {
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera3d>>()
        .single(app.world())
        .expect("creation scene camera");
    let Projection::Perspective(projection) = app.world().get::<Projection>(camera).unwrap() else {
        panic!("creation camera projection should be perspective")
    };
    projection.fov
}

#[test]
fn creation_scene_projection_tracks_authored_vertical_fov_after_window_resize() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.insert_resource(CustomizationDb::load(Path::new("data")));
    app.insert_resource(game_engine::asset::char_texture::CharTextureData::load(
        Path::new("data"),
    ));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(AccumulatedMouseMotion::default());
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.add_plugins(CharCreateScenePlugin);
    app.insert_state(GameState::CharCreate);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(1600, 900),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.update();
    app.update();

    let path = asset::asset_cache::model(623712).expect("cached Alliance backdrop");
    let source = parse_camera_snapshot(&std::fs::read(path).unwrap()).unwrap();
    let widescreen_aspect: f32 = 1600.0 / 900.0;
    let widescreen_fov = source.fov / (1.0 + widescreen_aspect.powi(2)).sqrt();
    assert!(
        (projected_fov(&mut app) - widescreen_fov).abs() < 0.0001,
        "16:9 window should convert authored camera diagonal FOV to vertical FOV"
    );

    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set(900.0, 900.0);
    app.update();
    let square_fov = source.fov / 2.0_f32.sqrt();
    assert!(
        (projected_fov(&mut app) - square_fov).abs() < 0.0001,
        "resizing to 1:1 should update the live camera projection"
    );

    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution.set_scale_factor_override(Some(1.15));
        window.resolution.set(1600.0, 900.0);
    }
    app.update();
    assert!(
        (projected_fov(&mut app) - widescreen_fov).abs() < 0.0001,
        "DPI scaling must not change the viewport aspect used for authored FOV"
    );
}
