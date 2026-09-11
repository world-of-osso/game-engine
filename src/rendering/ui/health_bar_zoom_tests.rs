use super::*;
use bevy::camera::CameraPlugin;
use bevy::mesh::VertexAttributeValues;
use bevy::window::{PrimaryWindow, WindowResolution};

pub(super) fn projection_app(dpi: f32, width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        WindowPlugin::default(),
        CameraPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.insert_resource(HudOptions {
        nameplate_health_thickness: NameplateBarThickness::Thin,
        ..default()
    });
    app.init_state::<GameState>();
    app.insert_resource(State::new(GameState::InWorld));
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<bevy::render::texture::ManualTextureViews>();
    app.add_plugins(HealthBarPlugin);
    app.add_systems(
        PostUpdate,
        bevy::render::camera::camera_system.in_set(CameraUpdateSystems),
    );
    app.world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .unwrap()
        .resolution = WindowResolution::new(width, height).with_scale_factor_override(dpi);
    app.finish();
    app.cleanup();
    app
}

fn zoom_scene(dpi: f32, width: u32, height: u32, fov: f32) -> (App, Entity, Entity, Entity) {
    let mut app = projection_app(dpi, width, height);
    let actor = app
        .world_mut()
        .spawn((
            Transform::from_xyz(102.0, 0.0, 0.0)
                .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.6, 0.0, 0.15))
                .with_scale(Vec3::new(0.7, 1.2, 0.9)),
            Visibility::Visible,
            Health {
                current: 75.0,
                max: 100.0,
            },
        ))
        .id();
    let bar = app.world().get::<Children>(actor).unwrap()[0];
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection { fov, ..default() }),
            Transform::from_xyz(100.0, 3.0, 6.0),
        ))
        .id();
    (app, actor, bar, camera)
}

fn projected_quad_size(
    app: &App,
    bar: Entity,
    camera: Entity,
    bar_global: GlobalTransform,
) -> Vec2 {
    let background = app.world().get::<Children>(bar).unwrap()[0];
    let mesh = &app.world().get::<Mesh3d>(background).unwrap().0;
    let mesh = app.world().resource::<Assets<Mesh>>().get(mesh).unwrap();
    let VertexAttributeValues::Float32x3(vertices) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        panic!("healthbar mesh must have positions");
    };
    let pose = bar_global.mul_transform(*app.world().get::<Transform>(background).unwrap());
    let camera_pose = app.world().get::<GlobalTransform>(camera).unwrap();
    let camera = app.world().get::<Camera>(camera).unwrap();
    let mut minimum = Vec2::splat(f32::INFINITY);
    let mut maximum = Vec2::splat(f32::NEG_INFINITY);
    for &vertex in vertices {
        let pixel = camera
            .world_to_viewport(camera_pose, pose.transform_point(Vec3::from(vertex)))
            .unwrap();
        minimum = minimum.min(pixel);
        maximum = maximum.max(pixel);
    }
    maximum - minimum
}

#[test]
fn thickness_changes_projected_height_without_changing_width() {
    let (mut app, _, bar, camera) = zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    for thickness in [NameplateBarThickness::Thin, NameplateBarThickness::Thick] {
        app.world_mut()
            .resource_mut::<HudOptions>()
            .nameplate_health_thickness = thickness;
        app.update();
        let global = *app.world().get::<GlobalTransform>(bar).unwrap();
        let size = projected_quad_size(&app, bar, camera, global);
        assert!(size.abs_diff_eq(health_bar_pixel_size(thickness), 0.05));
    }
}

#[test]
fn world_healthbar_quad_stays_one_ninety_by_ten_logical_pixels_across_zoom() {
    let target = Vec2::new(190.0, 10.0);
    let mut failures = Vec::new();
    for (dpi, width, height, fov_degrees) in [
        (1.0, 800, 600, 45.0_f32),
        (2.0, 800, 600, 45.0),
        (1.0, 1280, 720, 70.0),
        (2.0, 1280, 720, 70.0),
    ] {
        let (mut app, _, bar, camera) = zoom_scene(dpi, width, height, fov_degrees.to_radians());
        for distance in [6.0, 24.0] {
            app.world_mut()
                .get_mut::<Transform>(camera)
                .unwrap()
                .translation
                .z = distance;
            app.update();
            let global = *app.world().get::<GlobalTransform>(bar).unwrap();
            let size = projected_quad_size(&app, bar, camera, global);
            println!(
                "dpi={dpi} viewport={width}x{height} fov={fov_degrees} distance={distance}: actual={size:?}"
            );
            if !size.abs_diff_eq(target, 0.05) {
                failures.push((dpi, fov_degrees, distance, size));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "projected world-healthbar dimensions must remain190x10logicalpx: {failures:?}"
    );
}

#[test]
fn world_healthbar_responds_to_fov_viewport_and_dpi_changes_in_same_frame() {
    let (mut app, _, bar, camera) = zoom_scene(1.0, 800, 600, 45.0_f32.to_radians());
    app.update();
    app.world_mut()
        .entity_mut(camera)
        .insert(Projection::Perspective(PerspectiveProjection {
            fov: 70.0_f32.to_radians(),
            ..default()
        }));
    app.world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .unwrap()
        .resolution = WindowResolution::new(1280, 720).with_scale_factor_override(2.0);
    app.update();
    let size = projected_quad_size(
        &app,
        bar,
        camera,
        *app.world().get::<GlobalTransform>(bar).unwrap(),
    );
    assert!(
        size.abs_diff_eq(Vec2::new(190.0, 10.0), 0.05),
        "same-frame viewport/FOV size: {size:?}"
    );
    let tick = app
        .world()
        .entity(bar)
        .get_ref::<Transform>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(bar)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed(),
        tick
    );
}
