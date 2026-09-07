use std::time::Duration;

use bevy::{
    a11y::AccessibilityPlugin,
    app::{App, AppExit, Last, PluginGroup, ScheduleRunnerPlugin},
    asset::AssetPlugin,
    camera::{Camera, Camera2d, CameraPlugin, ClearColorConfig},
    core_pipeline::CorePipelinePlugin,
    ecs::{
        resource::Resource,
        system::{Commands, Res, ResMut},
    },
    image::ImagePlugin,
    input::InputPlugin,
    log::LogPlugin,
    mesh::MeshPlugin,
    prelude::{MinimalPlugins, default},
    render::{RenderPlugin, pipelined_rendering::PipelinedRenderingPlugin},
    time::{Real, Time},
    transform::TransformPlugin,
    window::{Window, WindowPlugin, WindowResolution},
    winit::{WinitPlugin, WinitSettings},
};

use crate::game::client_options::GraphicsOptions;

const BLANK_CLEAR_COLOR: bevy::color::Color = bevy::color::Color::srgb(0.094, 0.094, 0.094);
const UPDATE_LOG_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) fn run(continuous: bool, arguments: &[String]) -> Result<(), String> {
    let mut app = build_app(continuous);
    crate::system_isolation::configure(&mut app, arguments);
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("blank renderer exited with status {code}")),
    }
}

/// Installs only core Bevy, logging, window, asset, Winit, render, image, camera,
/// pipelined-rendering and core-pipeline plugins. It intentionally excludes game services.
fn build_app(continuous: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>())
        .add_plugins(LogPlugin {
            #[cfg(feature = "cpu-system-profile")]
            custom_layer: game_engine::cpu_system_profile::layer,
            ..default()
        })
        .add_plugins(TransformPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(AccessibilityPlugin)
        .add_plugins(WindowPlugin {
            primary_window: Some(render_window()),
            ..default()
        })
        .add_plugins(AssetPlugin::default())
        .add_plugins(WinitPlugin::default())
        .add_plugins(RenderPlugin::default())
        .add_plugins(ImagePlugin::default())
        .add_plugins(MeshPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(PipelinedRenderingPlugin)
        .add_plugins(CorePipelinePlugin)
        .insert_resource(winit_settings(continuous))
        .init_resource::<RendererUpdateReport>()
        .add_systems(bevy::app::Startup, spawn_blank_camera)
        .add_systems(Last, log_update_rate);
    app
}

fn render_window() -> Window {
    Window {
        title: "game-engine — blank Bevy renderer".into(),
        name: Some("com.worldofosso.game-engine.renderer".into()),
        present_mode: GraphicsOptions::default().present_mode(),
        resolution: WindowResolution::new(1280, 720),
        ..default()
    }
}

fn winit_settings(continuous: bool) -> WinitSettings {
    if continuous {
        WinitSettings::game()
    } else {
        WinitSettings::desktop_app()
    }
}

fn spawn_blank_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BLANK_CLEAR_COLOR),
            ..default()
        },
    ));
}

#[derive(Resource, Default)]
struct RendererUpdateReport {
    updates: u64,
    last_log: Duration,
    updates_at_last_log: u64,
}

fn log_update_rate(time: Res<Time<Real>>, mut report: ResMut<RendererUpdateReport>) {
    report.updates += 1;
    let elapsed = time.elapsed();
    let interval = elapsed.saturating_sub(report.last_log);
    if report.updates == 1 || interval >= UPDATE_LOG_INTERVAL {
        let interval_updates = report.updates - report.updates_at_last_log;
        let updates_per_second = if interval.is_zero() {
            0.0
        } else {
            interval_updates as f64 / interval.as_secs_f64()
        };
        eprintln!(
            "blank renderer update_rate elapsed_s={:.3} updates={} updates_per_s={updates_per_second:.3}",
            elapsed.as_secs_f64(),
            report.updates,
        );
        report.last_log = elapsed;
        report.updates_at_last_log = report.updates;
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        app::{App, Startup},
        camera::{Camera, Camera2d, ClearColorConfig},
        ecs::query::With,
    };

    use super::{BLANK_CLEAR_COLOR, spawn_blank_camera};

    #[test]
    fn blank_renderer_starts_one_active_2d_camera_with_its_clear_color() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_blank_camera);
        app.update();

        let cameras = app
            .world_mut()
            .query_filtered::<(&Camera, &Camera2d), With<Camera2d>>()
            .iter(app.world())
            .collect::<Vec<_>>();

        assert_eq!(cameras.len(), 1);
        assert!(cameras[0].0.is_active);
        let ClearColorConfig::Custom(color) = cameras[0].0.clear_color else {
            panic!("blank camera must have a custom clear color");
        };
        assert_eq!(color, BLANK_CLEAR_COLOR);
    }
}
