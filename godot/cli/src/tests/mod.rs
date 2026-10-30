use std::path::PathBuf;

use game_engine_network::ipc_wire::{ItemInfoQuery, Request};
use serde_json::Value;

use crate::command_dispatch::{
    execute_text_request_output, format_text_response_output, serialize_json,
};
use crate::requests::*;
use crate::*;

mod camera;
mod export_character;
mod request_actions;
mod request_status_and_basic;
mod request_world_and_equipment;
mod transport;

#[test]
fn probe_ground_accepts_world_coordinates_and_direction() {
    let parsed = Cli::try_parse_from([
        "game-engine-cli",
        "probe-ground",
        "27.4",
        "-34",
        "154.36",
        "-0.8",
        "0.6",
    ])
    .unwrap();
    assert!(
        matches!(parsed.command, Cmd::ProbeGround { x, y, z, dir_x, dir_z } if x == 27.4 && y == -34.0 && z == 154.36 && dir_x == -0.8 && dir_z == 0.6)
    );
}
