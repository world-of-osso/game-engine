use std::time::Duration;

use bevy::{
    app::{App, Last, PluginGroup, ScheduleRunnerPlugin},
    asset::AssetPlugin,
    camera::{Camera, Camera2d, CameraPlugin, ClearColorConfig},
    core_pipeline::CorePipelinePlugin,
    ecs::{
        resource::Resource,
        system::{Commands, Res},
    },
    image::ImagePlugin,
    prelude::MinimalPlugins,
    render::{pipelined_rendering::PipelinedRenderingPlugin, RenderPlugin},
    time::{Real, Time},
    transform::TransformPlugin,
    window::{Window, WindowPlugin},
    winit::{WinitPlugin, WinitSettings},
};

use crate::game::state::client_options::GraphicsOptions;

const BLANK_CLEAR_COLOR: bevy::color::Color = bevy::color::Color::srgb(0.094, 0.094, 0.094);
const UPDATE_LOG_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) fn run(continuous: bool) -> Result<(), String> {
    build_app(continuous).run();
    Ok(())
}

fn build_app(continuous: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>())
        .add_plugins(TransformPlugin)
        .add_plugins(WindowPlugin {
            primary_window: Some(render_window()),
            ..default()
        })
        .add_plugins(AssetPlugin::default())
        .add_plugins(WinitPlugin::default())
        .add_plugins(RenderPlugin::default())
        .add_plugins(ImagePlugin::default())
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
}

fn log_update_rate(time: Res<Time<Real>>, mut report: ResMut<RendererUpdateReport>) {
    report.updates += 1;
    let elapsed = time.elapsed();
    if report.updates == 1 || elapsed.saturating_sub(report.last_log) >= UPDATE_LOG_INTERVAL {
        eprintln!(
            "blank renderer updates: {} elapsed={elapsed:.3?}",
            report.updates
        );
        report.last_log = elapsed;
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        app::{App, Startup},
        camera::{Camera, Camera2d, ClearColorConfig},
        ecs::query::With,
    };

    use super::{spawn_blank_camera, BLANK_CLEAR_COLOR};

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
