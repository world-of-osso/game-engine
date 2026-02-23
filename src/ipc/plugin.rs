//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

use super::{Command, Request, Response, init};

/// Channel sender to reply to an IPC caller waiting for a screenshot.
#[derive(Component)]
struct ScreenshotReply(mpsc::Sender<Response>);

pub struct IpcPlugin;

impl Plugin for IpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init();

        app.insert_non_send_resource(receiver)
            .insert_non_send_resource(guard)
            .add_systems(Update, poll_ipc);
    }
}

/// Poll IPC commands each frame and dispatch them.
fn poll_ipc(
    receiver: NonSend<mpsc::Receiver<Command>>,
    mut commands: Commands,
) {
    while let Ok(cmd) = receiver.try_recv() {
        handle_command(cmd, &mut commands);
    }
}

fn handle_command(cmd: Command, commands: &mut Commands) {
    match cmd.request {
        Request::Ping => {
            let _ = cmd.respond.send(Response::Pong);
        }
        Request::Screenshot => {
            commands
                .spawn(Screenshot::primary_window())
                .insert(ScreenshotReply(cmd.respond))
                .observe(on_screenshot_captured);
        }
    }
}

/// Per-entity observer triggered when this screenshot is captured.
fn on_screenshot_captured(
    trigger: On<ScreenshotCaptured>,
    query: Query<&ScreenshotReply>,
    mut commands: Commands,
) {
    let entity = trigger.event_target();
    let Ok(reply) = query.get(entity) else {
        return;
    };

    let response = encode_screenshot(&trigger.image);
    let _ = reply.0.send(response);

    commands.entity(entity).despawn();
}

fn encode_screenshot(img: &bevy::image::Image) -> Response {
    let Some(data) = img.data.as_ref() else {
        return Response::Error("screenshot has no pixel data".into());
    };
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(data, size.x, size.y);
    let webp_data = encoder.encode(15.0);
    Response::Screenshot(webp_data.to_vec())
}
