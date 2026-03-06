//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

use super::{Command, Request, Response, init};
use crate::auction_house::{AuctionHouseState, queue_ipc_request};
use crate::ui::plugin::UiState;

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
#[allow(clippy::type_complexity)]
fn poll_ipc(
    receiver: NonSend<mpsc::Receiver<Command>>,
    mut commands: Commands,
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    ui_state: Res<UiState>,
    mut auction_house: ResMut<AuctionHouseState>,
) {
    while let Ok(cmd) = receiver.try_recv() {
        dispatch(
            cmd,
            &mut commands,
            &tree_query,
            &parent_query,
            &ui_state,
            &mut auction_house,
        );
    }
}

#[allow(clippy::type_complexity)]
fn dispatch(
    cmd: Command,
    commands: &mut Commands,
    tree_query: &Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: &Query<&ChildOf>,
    ui_state: &UiState,
    auction_house: &mut AuctionHouseState,
) {
    if queue_ipc_request(auction_house, &cmd.request, cmd.respond.clone()) {
        return;
    }
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
        Request::DumpTree { filter } => {
            let tree = crate::dump::build_tree(tree_query, parent_query, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::DumpUiTree { filter } => {
            let tree = crate::dump::build_ui_tree(&ui_state.registry, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::AuctionOpen
        | Request::AuctionBrowse { .. }
        | Request::AuctionOwned
        | Request::AuctionBids
        | Request::AuctionInventory
        | Request::AuctionMailbox
        | Request::AuctionCreate { .. }
        | Request::AuctionBid { .. }
        | Request::AuctionBuyout { .. }
        | Request::AuctionCancel { .. }
        | Request::AuctionClaimMail { .. }
        | Request::AuctionStatus => {}
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
