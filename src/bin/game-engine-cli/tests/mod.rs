use std::path::PathBuf;

use game_engine::character_export::{
    ExportCharacterPayload, build_export_character_payload, write_export_character_file,
};
use game_engine::ipc::{Request, Response};
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{DeleteMail, ListMailQuery, ReadMail, SendMail};
use game_engine::status::{
    CharacterStatsSnapshot, EquipmentAppearanceStatusSnapshot, EquippedGearEntry,
    EquippedGearStatusSnapshot,
};
use serde_json::Value;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

use crate::command_dispatch::{
    execute_text_request_output, format_text_response_output, serialize_json,
};
use crate::requests::*;
use crate::*;

mod export_character;
mod request_actions;
mod request_status_and_basic;
mod request_world_and_equipment;
mod transport;
