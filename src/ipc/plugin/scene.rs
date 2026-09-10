use std::path::Path;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

use super::{Command, Request, Response, SceneParams, ScreenshotReply};
use crate::ipc::build_performance_snapshot;

pub(super) fn dispatch_scene_request(cmd: &Command, scene: &mut SceneParams) -> bool {
    match &cmd.request {
        Request::Ping => reply_scene_ping(cmd),
        Request::Screenshot => queue_scene_screenshot(cmd, scene),
        Request::Performance => reply_scene_performance(cmd, scene),
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

fn reply_scene_performance(cmd: &Command, scene: &SceneParams) {
    let focused = scene
        .primary_window
        .single()
        .is_ok_and(|window| window.focused);
    let snapshot = build_performance_snapshot(&scene.diagnostics, focused);
    let _ = cmd.respond.send(Response::Performance(snapshot));
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
    let tree = crate::dump::build_ui_tree_with_native(
        &scene.ui_state.registry,
        &scene.native_ui_query,
        filter,
    );
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
    match crate::screenshot::encode_webp(img, crate::screenshot::DEFAULT_WEBP_QUALITY) {
        Ok(webp_data) => Response::Screenshot(webp_data),
        Err(err) => Response::Error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::native::NativeUiElement;
    use crate::ui::plugin::UiState;
    use bevy::ecs::system::SystemState;

    fn request_ui_dump(world: &mut World, filter: Option<&str>) -> String {
        let (respond, receive) = std::sync::mpsc::channel();
        let command = Command {
            request: Request::DumpUiTree {
                filter: filter.map(str::to_owned),
            },
            respond,
        };
        let mut state: SystemState<SceneParams> = SystemState::new(world);
        assert!(dispatch_scene_request(&command, &mut state.get_mut(world)));
        match receive.try_recv().expect("dump request returns a response") {
            Response::Tree(tree) => tree,
            response => panic!("expected UI tree, received {response:?}"),
        }
    }

    #[test]
    fn native_ui_dump_request_includes_live_entities_and_preserves_legacy_filter() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<bevy::diagnostic::DiagnosticsStore>();
        let mut registry = crate::ui::registry::FrameRegistry::new(1920.0, 1080.0);
        registry.create_frame("LegacyMenu", None);
        let legacy = crate::dump::build_ui_tree(&registry, Some("legacymenu"));
        world.insert_resource(UiState {
            registry,
            event_bus: crate::ui::event::EventBus::new(),
            focused_frame: None,
        });
        let root = world
            .spawn((NativeUiElement, Name::new("NativeLogin")))
            .id();
        world.spawn((
            NativeUiElement,
            Name::new("Status"),
            Text::new("Ready"),
            ChildOf(root),
        ));

        let tree = request_ui_dump(&mut world, None);
        assert!(tree.starts_with(&legacy));
        assert!(tree.contains("NativeLogin [Native]\n  Status [Native]"));
        assert!(tree.contains("text=\"Ready\""));
        assert_eq!(request_ui_dump(&mut world, Some("legacymenu")), legacy);
        assert!(!request_ui_dump(&mut world, Some("nativelogin")).contains("LegacyMenu"));
    }
}
