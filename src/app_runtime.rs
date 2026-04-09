use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
};
use std::path::PathBuf;

use crate::{dump_systems, game_state, networking, scenes};

#[derive(Resource)]
pub(crate) struct ScreenshotRequest {
    pub(crate) output: PathBuf,
    pub(crate) frames_remaining: u32,
}

pub(crate) fn run_headless_ui_dump_app(initial_state: Option<game_state::GameState>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(game_engine::ui::plugin::UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(crate::DumpUiTreeFlag);
    if let Some(state) = initial_state {
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(game_state::InitialGameState(state));
        app.add_plugins(game_state::GameStatePlugin);
        if matches!(state, game_state::GameState::Eula) {
            app.add_plugins(scenes::eula::EulaScreenPlugin);
        } else if matches!(state, game_state::GameState::Login) {
            app.init_resource::<networking::AuthUiFeedback>();
            app.add_plugins(scenes::login::LoginScreenPlugin);
        } else if matches!(state, game_state::GameState::SelectionDebug) {
            app.add_plugins(scenes::selection_debug::SelectionDebugScreenPlugin);
        } else if matches!(state, game_state::GameState::InWorldSelectionDebug) {
            app.add_plugins(scenes::selection_debug::InWorldSelectionDebugScreenPlugin);
        }
    }
    app.add_systems(PostStartup, dump_systems::headless_dump_ui_tree_immediate);
    app.run();
}

pub(crate) fn take_screenshot(
    mut commands: Commands,
    req: Option<ResMut<ScreenshotRequest>>,
    automation_queue: Option<Res<game_engine::ui::automation::UiAutomationQueue>>,
    state: Res<State<crate::game_state::GameState>>,
) {
    let Some(mut req) = req else { return };
    if automation_queue.is_some_and(|q| !q.0.is_empty()) {
        return;
    }
    if matches!(
        *state.get(),
        crate::game_state::GameState::Login | crate::game_state::GameState::Connecting
    ) {
        return;
    }
    if req.frames_remaining > 0 {
        req.frames_remaining -= 1;
        return;
    }
    commands.remove_resource::<ScreenshotRequest>();
    let output = req.output.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            save_screenshot(&trigger.image, &output);
            exit.write(AppExit::Success);
        },
    );
}

fn save_screenshot(img: &bevy::image::Image, output: &PathBuf) {
    let webp_data = match game_engine::screenshot::encode_webp(img, 15.0) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
    std::fs::write(output, &webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

pub fn rgba_image(pixels: Vec<u8>, w: u32, h: u32) -> Image {
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
