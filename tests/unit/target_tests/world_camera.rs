use super::*;
use bevy::camera::CameraPlugin;
use bevy::ecs::system::RunSystemOnce;

fn picking_app() -> (App, Entity) {
    let mut app = App::new();
    register_picking_plugins(&mut app);
    configure_cursor_and_cameras(&mut app);
    let npc = spawn_pickable_npc(&mut app);
    initialize_picking_viewport(&mut app);
    (app, npc)
}

fn register_picking_plugins(app: &mut App) {
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::window::WindowPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        CameraPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.init_resource::<bevy::render::texture::ManualTextureViews>();
    app.add_systems(PostUpdate, bevy::render::camera::camera_system);
    app.init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<CurrentTarget>()
        .insert_resource(crate::client_options::UiDisabled);
}

fn configure_cursor_and_cameras(app: &mut App) {
    let mut window = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .unwrap();
    window.resolution.set(800.0, 600.0);
    window.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
    app.world_mut().spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    app.world_mut()
        .run_system_once(ui_toolkit::render::setup_ui_camera)
        .unwrap();
}

fn spawn_pickable_npc(app: &mut App) -> Entity {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::new(2.0, 2.0, 2.0));
    let npc = app
        .world_mut()
        .spawn((
            RemoteEntity,
            Npc { template_id: 42 },
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    app.world_mut().spawn((
        Mesh3d(mesh),
        Transform::default(),
        Visibility::Visible,
        ChildOf(npc),
    ));
    npc
}

fn initialize_picking_viewport(app: &mut App) {
    app.finish();
    app.cleanup();
    app.update();
    app.update();
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        2
    );
    let (camera, transform) = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
        .single(app.world())
        .unwrap();
    assert!(
        camera
            .viewport_to_world(transform, Vec2::new(400.0, 300.0))
            .is_ok()
    );
}

#[test]
fn world_camera_click_selects_npc_with_ui_camera_and_ui_disabled() {
    let (mut app, npc) = picking_app();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(click_to_target).unwrap();
    assert_eq!(app.world().resource::<CurrentTarget>().0, Some(npc));
}

#[test]
fn world_camera_right_click_ray_resolves_npc_with_ui_camera() {
    let (mut app, npc) = picking_app();
    app.init_resource::<GossipIntentQueue>()
        .init_resource::<MailIntentQueue>();
    app.world_mut().spawn((
        Player,
        GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    ));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    app.world_mut()
        .run_system_once(right_click_interact)
        .unwrap();
    assert_eq!(app.world().resource::<CurrentTarget>().0, Some(npc));
}
