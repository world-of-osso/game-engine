//! `HoverAt` / `HoverNpc`: move the cursor so hover-driven UI (unit tooltips)
//! can be driven without a pointer device. The window keeps the set position
//! until a real pointer event moves it.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::components::Npc;

use game_engine::camera_control::WowCamera;

use super::Response;

/// Height above an NPC's origin (its feet) that the cursor aims at, in yards.
const NPC_AIM_HEIGHT: f32 = 1.0;

pub(super) enum HoverTarget {
    Point(Vec2),
    Npc(String),
}

/// The hover request waiting for the next frame, and who to answer.
#[derive(Resource, Default)]
pub(super) struct PendingHover(Option<(HoverTarget, mpsc::Sender<Response>)>);

impl PendingHover {
    pub(super) fn queue(&mut self, target: HoverTarget, respond: mpsc::Sender<Response>) {
        if let Some((_, previous)) = self.0.replace((target, respond)) {
            let _ = previous.send(Response::Error("replaced by a newer hover".into()));
        }
    }
}

pub(super) fn apply_pending_hover(
    mut pending: ResMut<PendingHover>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WowCamera>>,
    npcs: Query<(&Npc, &GlobalTransform)>,
) {
    let Some((target, respond)) = pending.0.take() else {
        return;
    };
    let point = match target {
        HoverTarget::Point(point) => Ok(point),
        HoverTarget::Npc(name) => npc_screen_point(&name, &cameras, &npcs),
    };
    let response = point.and_then(|point| {
        let mut window = windows.single_mut().map_err(|_| "no primary window")?;
        if point.x < 0.0 || point.y < 0.0 || point.x > window.width() || point.y > window.height() {
            return Err(format!(
                "({:.0}, {:.0}) is outside the window",
                point.x, point.y
            ));
        }
        window.set_cursor_position(Some(point));
        Ok(format!("cursor at ({:.0}, {:.0})", point.x, point.y))
    });
    let _ = respond.send(match response {
        Ok(text) => Response::Text(text),
        Err(error) => Response::Error(error),
    });
}

/// Window position of the nearest NPC named `name` in front of the camera.
fn npc_screen_point(
    name: &str,
    cameras: &Query<(&Camera, &GlobalTransform), With<WowCamera>>,
    npcs: &Query<(&Npc, &GlobalTransform)>,
) -> Result<Vec2, String> {
    let (camera, camera_pose) = cameras
        .iter()
        .find(|(camera, _)| camera.is_active)
        .ok_or("no active in-world camera")?;
    npcs.iter()
        .filter(|(npc, _)| npc.name.eq_ignore_ascii_case(name))
        .filter_map(|(_, pose)| {
            let aim = pose.translation() + Vec3::Y * NPC_AIM_HEIGHT;
            let point = camera.world_to_viewport(camera_pose, aim).ok()?;
            Some((aim.distance_squared(camera_pose.translation()), point))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| point)
        .ok_or_else(|| format!("no NPC named {name} in view"))
}
