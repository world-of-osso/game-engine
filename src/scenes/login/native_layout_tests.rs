use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::camera::{CameraPlugin, ComputedCameraValues, RenderTargetInfo};
use bevy::image::ImagePlugin;
use bevy::math::Affine2;
use bevy::text::TextPlugin;
use bevy::ui::UiPlugin;

fn layout_app(width: u32, height: u32) -> (App, Entity) {
    let mut app = App::new();
    initialize_fixture(app.world_mut());
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::window::WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        CameraPlugin,
        TextPlugin,
        bevy::picking::DefaultPickingPlugins,
        UiPlugin,
    ));
    app.init_asset::<TextureAtlasLayout>();
    let existing_camera = app
        .world_mut()
        .spawn((
            UiCamera,
            Camera {
                order: 7,
                ..default()
            },
        ))
        .id();
    setup_success_in_world(app.world_mut());
    let camera = app.world().resource::<LoginView>().camera;
    app.world_mut().get_mut::<Camera>(camera).unwrap().computed = ComputedCameraValues {
        target_info: Some(RenderTargetInfo {
            physical_size: UVec2::new(width, height),
            scale_factor: 1.0,
        }),
        ..default()
    };
    update(app.world_mut());
    app.finish();
    app.cleanup();
    // Run the real text/layout pipeline through its first settled frame.
    app.update();
    app.update();
    (app, existing_camera)
}

fn named_entity(world: &mut World, name: &str) -> Entity {
    let matches: Vec<_> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter_map(|(entity, candidate)| (candidate.as_str() == name).then_some(entity))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected one semantic element named {name}"
    );
    matches[0]
}

fn bounds(world: &mut World, name: &str) -> Rect {
    let entity = named_entity(world, name);
    let node = world.get::<ComputedNode>(entity).unwrap();
    let transform = world.get::<UiGlobalTransform>(entity).unwrap();
    Rect::from_center_size(Affine2::from(transform).translation, node.size)
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "expected {expected} within one layout pixel, got {actual}"
    );
}

#[test]
fn native_login_layout_preserves_centered_fields_and_action_order_at_two_viewports() {
    for (width, height) in [(1280, 720), (1600, 900)] {
        let (mut app, existing_camera) = layout_app(width, height);
        let world = app.world_mut();
        let username = bounds(world, "UsernameInput");
        let password = bounds(world, "PasswordInput");
        let connect = bounds(world, "ConnectButton");
        let status = bounds(world, "LoginStatusBounds");
        for field in [username, password] {
            near(field.center().x, width as f32 / 2.0);
            near(field.width(), 320.0);
            near(field.height(), 42.0);
        }
        near(username.min.y, height as f32 / 2.0 - 167.0);
        near(password.min.y - username.min.y, 72.0);
        assert!(username.max.y < password.min.y);
        assert!(password.max.y < connect.min.y);
        near(connect.center().x, width as f32 / 2.0);
        near(connect.center().y, height as f32 / 2.0);
        near(connect.width(), 250.0);
        near(connect.height(), 66.0);
        assert!(status.min.y > connect.max.y);

        let menu = bounds(world, "MenuButton");
        let quit = bounds(world, "ExitButton");
        near(menu.max.x, width as f32 - 24.0);
        near(quit.max.x, width as f32 - 24.0);
        near(quit.max.y, height as f32 - 56.0);
        near(quit.min.y - menu.max.y, 10.0);
        assert!(menu.center().x > width as f32 / 2.0);
        assert!(menu.center().y > height as f32 / 2.0);
        assert_eq!(world.get::<Camera>(existing_camera).unwrap().order, 2);
        cleanup(world);
        assert_eq!(world.get::<Camera>(existing_camera).unwrap().order, 7);
    }
}

#[test]
fn native_login_layout_fills_background_preserves_footer_and_excludes_hidden_controls() {
    let (mut app, _) = layout_app(1280, 720);
    let world = app.world_mut();
    for name in [
        "LoginRoot",
        "BlackLoginBackground",
        "LoginBackground",
        "LoginBackgroundShade",
    ] {
        let rect = bounds(world, name);
        near(rect.min.x, 0.0);
        near(rect.min.y, 0.0);
        near(rect.width(), 1280.0);
        near(rect.height(), 720.0);
    }
    for name in ["RealmButton", "CreateAccountButton"] {
        let entity = named_entity(world, name);
        assert_eq!(world.get::<ComputedNode>(entity).unwrap().size, Vec2::ZERO);
        assert_eq!(world.get::<Interaction>(entity), Some(&Interaction::None));
    }
    let logo = bounds(world, "LoginGameLogo");
    near(logo.width(), 384.0);
    near(logo.height(), 256.0);
    let version = bounds(world, "VersionTextBounds");
    near(version.min.x, 10.0);
    near(version.max.y, 712.0);
    let disclaimer = bounds(world, "DisclaimerTextBounds");
    near(disclaimer.center().x, 640.0);
    near(disclaimer.max.y, 712.0);
    let thanks = bounds(world, "BlizzardThanksBounds");
    let blizzard = bounds(world, "BlizzardLogo");
    near(blizzard.center().x, 640.0);
    near(blizzard.width(), 100.0);
    near(blizzard.height(), 100.0);
    assert!(thanks.center().y < blizzard.center().y);
    assert!(blizzard.max.y < disclaimer.min.y);
}
