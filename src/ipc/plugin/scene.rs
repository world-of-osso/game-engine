use std::path::Path;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

use super::{Command, Request, Response, SceneParams, ScreenshotReply};

pub(super) fn dispatch_scene_request(cmd: &Command, scene: &mut SceneParams) -> bool {
    match &cmd.request {
        Request::Ping => reply_scene_ping(cmd),
        Request::Screenshot => queue_scene_screenshot(cmd, scene),
        Request::DumpTree { filter } => reply_scene_tree_dump(cmd, scene, filter.as_deref()),
        Request::DumpUiTree { filter } => reply_scene_ui_tree_dump(cmd, scene, filter.as_deref()),
        Request::DumpScene { filter: _ } => reply_scene_dump(cmd, scene),
        Request::ExportScene { output_path } => reply_scene_export(cmd, scene, output_path),
        _ => return false,
    }
    true
}

fn reply_scene_ping(cmd: &Command) {
    let _ = cmd.respond.send(Response::Pong);
}

fn queue_scene_screenshot(cmd: &Command, scene: &mut SceneParams) {
    scene
        .commands
        .spawn(Screenshot::primary_window())
        .insert(ScreenshotReply(cmd.respond.clone()))
        .observe(on_screenshot_captured);
}

fn reply_scene_tree_dump(cmd: &Command, scene: &SceneParams, filter: Option<&str>) {
    let tree = crate::dump::build_tree(&scene.tree_query, &scene.parent_query, filter);
    let _ = cmd.respond.send(Response::Tree(tree));
}

fn reply_scene_ui_tree_dump(cmd: &Command, scene: &SceneParams, filter: Option<&str>) {
    let tree = crate::dump::build_ui_tree(&scene.ui_state.registry, filter);
    let _ = cmd.respond.send(Response::Tree(tree));
}

fn reply_scene_dump(cmd: &Command, scene: &mut SceneParams) {
    let text = build_scene_dump_text(scene);
    let _ = cmd.respond.send(Response::Tree(text));
}

fn build_scene_dump_text(scene: &mut SceneParams) -> String {
    match &scene.scene_tree {
        Some(tree) => crate::dump::build_scene_tree(
            tree,
            &scene.transform_query,
            &scene.global_transform_query,
            &scene.parent_query,
            &scene.aabb_query,
            &scene.camera_query,
            &mut scene.ray_cast,
        ),
        None => "(no scene tree)".into(),
    }
}

fn reply_scene_export(cmd: &Command, scene: &SceneParams, output_path: &str) {
    let response = match export_scene(scene, output_path) {
        Ok(message) => Response::Text(message),
        Err(error) => Response::Error(error),
    };
    let _ = cmd.respond.send(response);
}

fn export_scene(scene: &SceneParams, output_path: &str) -> Result<String, String> {
    let tree = scene
        .scene_tree
        .as_ref()
        .ok_or_else(|| "no scene tree available to export".to_string())?;
    let snapshot = crate::scene_tree::snapshot_scene_tree(tree, &scene.transform_query);
    crate::scene_tree::write_scene_snapshot_file(Path::new(output_path), &snapshot)
        .map(|_| format!("scene exported to {output_path}"))
}

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
    match crate::screenshot::encode_webp(img, 15.0) {
        Ok(webp_data) => Response::Screenshot(webp_data),
        Err(err) => Response::Error(err),
    }
}
