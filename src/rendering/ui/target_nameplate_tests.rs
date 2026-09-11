use super::*;
use crate::rendering::nameplate_picking::NameplateHitTarget;
use bevy::camera::{CameraPlugin, primitives::Aabb};
use bevy::ecs::system::RunSystemOnce;
use bevy::math::Vec3A;
use bevy::window::WindowResolution;

struct ClickFixture {
    app: App,
    window: Entity,
    mesh_owner: Entity,
    plate_owner: Entity,
    plate: Entity,
    cursor: Vec2,
}

fn fixture() -> ClickFixture {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        WindowPlugin::default(),
        CameraPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.init_resource::<bevy::render::texture::ManualTextureViews>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<CurrentTarget>();
    app.add_systems(Update, click_to_target);
    app.add_systems(PostUpdate, bevy::render::camera::camera_system);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution = WindowResolution::new(640, 480);
    app.world_mut().spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    app.world_mut()
        .run_system_once(ui_toolkit::render::setup_ui_camera)
        .unwrap();
    app.finish();
    app.cleanup();
    for _ in 0..3 {
        app.update();
    }
    let cursor = Vec2::new(120.0, 140.0);
    let (camera, camera_pose) = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
        .single(app.world())
        .unwrap();
    let ray = camera.viewport_to_world(camera_pose, cursor).unwrap();
    let mesh_position = ray.origin + ray.direction * 5.0;
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_size(Vec3::ONE));
    let mesh_owner = app
        .world_mut()
        .spawn((
            RemoteEntity,
            Npc {
                template_id: 1,
                name: "Behind the plate".into(),
            },
            Mesh3d(mesh),
            Aabb {
                center: Vec3A::ZERO,
                half_extents: Vec3A::splat(0.5),
            },
            Transform::from_translation(mesh_position),
            Visibility::Visible,
        ))
        .id();
    let plate_owner = app
        .world_mut()
        .spawn((
            Name::new("Plate owner"),
            Transform::from_xyz(8.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();
    let (overlay, overlay_pose) = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<ui_toolkit::render::UiCamera>>()
        .single(app.world())
        .unwrap();
    let position = overlay.viewport_to_world_2d(overlay_pose, cursor).unwrap();
    let plate = app
        .world_mut()
        .spawn((
            NameplateHitTarget(plate_owner),
            Sprite::from_color(Color::WHITE, Vec2::new(100.0, 24.0)),
            Transform::from_translation(position.extend(1.0)),
            Visibility::Visible,
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    ClickFixture {
        app,
        window,
        mesh_owner,
        plate_owner,
        plate,
        cursor,
    }
}

fn click(fixture: &mut ClickFixture) {
    fixture
        .app
        .world_mut()
        .get_mut::<Window>(fixture.window)
        .unwrap()
        .set_cursor_position(Some(fixture.cursor));
    // The cube lies on the camera ray. This CPU fixture has no renderer.
    fixture
        .app
        .world_mut()
        .get_mut::<ViewVisibility>(fixture.mesh_owner)
        .unwrap()
        .set();
    {
        let mut mouse = fixture
            .app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
        mouse.clear();
        mouse.press(MouseButton::Left);
    }
    fixture.app.update();
    fixture
        .app
        .world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}

#[test]
fn nameplate_click_selects_owner_instead_of_mesh_behind_it() {
    let mut fixture = fixture();
    *fixture
        .app
        .world_mut()
        .get_mut::<Visibility>(fixture.plate)
        .unwrap() = Visibility::Hidden;
    fixture.app.update();
    click(&mut fixture);
    assert_eq!(
        fixture.app.world().resource::<CurrentTarget>().0,
        Some(fixture.mesh_owner)
    );
    *fixture
        .app
        .world_mut()
        .get_mut::<Visibility>(fixture.plate)
        .unwrap() = Visibility::Visible;
    fixture.app.update();
    click(&mut fixture);
    assert_eq!(
        fixture.app.world().resource::<CurrentTarget>().0,
        Some(fixture.plate_owner)
    );
}

#[test]
fn nameplate_click_without_model_hit_selects_the_plate_owner() {
    let mut fixture = fixture();
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.mesh_owner)
        .remove::<Mesh3d>();
    click(&mut fixture);
    assert_eq!(
        fixture.app.world().resource::<CurrentTarget>().0,
        Some(fixture.plate_owner)
    );
}

#[test]
fn nameplate_click_cannot_select_hidden_or_local_plates() {
    let mut fixture = fixture();
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.mesh_owner)
        .remove::<Mesh3d>();
    *fixture
        .app
        .world_mut()
        .get_mut::<Visibility>(fixture.plate)
        .unwrap() = Visibility::Hidden;
    fixture.app.update();
    click(&mut fixture);
    assert_eq!(fixture.app.world().resource::<CurrentTarget>().0, None);
    *fixture
        .app
        .world_mut()
        .get_mut::<Visibility>(fixture.plate)
        .unwrap() = Visibility::Visible;
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.plate_owner)
        .insert(crate::networking::LocalPlayer);
    fixture.app.update();
    click(&mut fixture);
    assert_eq!(fixture.app.world().resource::<CurrentTarget>().0, None);
}
