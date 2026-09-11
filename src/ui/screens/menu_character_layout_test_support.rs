use bevy::asset::AssetApp;
use bevy::prelude::*;
use ui_toolkit::plugin::UiState;
use ui_toolkit::registry::FrameRegistry;

/// Exercise native layout and registry readback without a Winit window or renderer.
pub(super) fn compute_layout(registry: &mut FrameRegistry) {
    let width = registry.screen_width;
    let height = registry.screen_height;
    let mut app = layout_app(width, height);
    app.world_mut().resource_mut::<UiState>().registry =
        std::mem::replace(registry, FrameRegistry::new(width, height));
    for _ in 0..3 {
        app.update();
    }
    *registry = app
        .world_mut()
        .remove_resource::<UiState>()
        .unwrap()
        .registry;
}

pub(super) fn layout_app(width: f32, height: f32) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::window::WindowPlugin {
            primary_window: Some(Window {
                resolution: (width as u32, height as u32).into(),
                ..default()
            }),
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::CameraPlugin,
        bevy::text::TextPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::ui::UiPlugin,
        ui_toolkit::plugin::UiPlugin,
    ));
    app.init_asset::<TextureAtlasLayout>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(16),
    ));
    app.add_systems(
        PostUpdate,
        update_camera_target
            .after(bevy::camera::CameraUpdateSystems)
            .before(bevy::ui::UiSystems::Prepare),
    );
    app.finish();
    app.cleanup();
    app
}

fn update_camera_target(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<ui_toolkit::render::UiCamera>>,
) {
    let window = windows.single().unwrap();
    for mut camera in &mut cameras {
        camera.computed = bevy::camera::ComputedCameraValues {
            target_info: Some(bevy::camera::RenderTargetInfo {
                physical_size: UVec2::new(window.physical_width(), window.physical_height()),
                scale_factor: window.scale_factor(),
            }),
            ..default()
        };
    }
}
