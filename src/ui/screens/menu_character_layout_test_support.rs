use bevy::asset::AssetApp;
use bevy::prelude::*;
use ui_toolkit::plugin::UiState;
use ui_toolkit::registry::FrameRegistry;

/// Exercise native layout and registry readback without a Winit window or renderer.
pub(super) fn compute_layout(registry: &mut FrameRegistry) {
    let width = registry.screen_width;
    let height = registry.screen_height;
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
    app.world_mut().resource_mut::<UiState>().registry =
        std::mem::replace(registry, FrameRegistry::new(width, height));
    app.finish();
    app.cleanup();
    for _ in 0..3 {
        app.update();
    }
    *registry = app
        .world_mut()
        .remove_resource::<UiState>()
        .unwrap()
        .registry;
}
