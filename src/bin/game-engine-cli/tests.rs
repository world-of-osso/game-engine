use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
use peercred_ipc::Server;
use serde_json::Value;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

use crate::requests::*;
use crate::*;

#[test]
fn mail_send_command_maps_to_send_request() {
    let request = mail_request(MailCmd::Send {
        to: "Thrall".into(),
        from: "Jaina".into(),
        subject: "Supplies".into(),
        body: "Three crates are ready.".into(),
        money: 1250,
    })
    .expect("valid send command");

    assert_eq!(
        request,
        Request::MailSend {
            mail: SendMail {
                to: "Thrall".into(),
                from: "Jaina".into(),
                subject: "Supplies".into(),
                body: "Three crates are ready.".into(),
                money: 1250,
            },
        }
    );
}

#[test]
fn mail_list_command_maps_to_list_request() {
    let request = mail_request(MailCmd::List {
        character: Some("Thrall".into()),
        include_deleted: true,
    })
    .expect("valid list command");

    assert_eq!(
        request,
        Request::MailList {
            query: ListMailQuery {
                character: Some("Thrall".into()),
                include_deleted: true
            },
        }
    );
}

#[test]
fn mail_read_command_maps_to_read_request() {
    let request = mail_request(MailCmd::Read { mail_id: 42 }).expect("valid read command");
    assert_eq!(
        request,
        Request::MailRead {
            read: ReadMail { mail_id: 42 }
        }
    );
}

#[test]
fn mail_delete_command_maps_to_delete_request() {
    let request = mail_request(MailCmd::Delete { mail_id: 42 }).expect("valid delete command");
    assert_eq!(
        request,
        Request::MailDelete {
            delete: DeleteMail { mail_id: 42 }
        }
    );
}

#[test]
fn network_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Network).unwrap(),
        Request::NetworkStatus
    );
}

#[test]
fn terrain_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Terrain).unwrap(),
        Request::TerrainStatus
    );
}

#[test]
fn sound_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Sound).unwrap(),
        Request::SoundStatus
    );
}

#[test]
fn currencies_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Currencies).unwrap(),
        Request::CurrenciesStatus
    );
}

#[test]
fn reputations_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Reputations).unwrap(),
        Request::ReputationsStatus
    );
}

#[test]
fn character_stats_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::CharacterStats).unwrap(),
        Request::CharacterStatsStatus
    );
}

#[test]
fn bags_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Bags).unwrap(),
        Request::BagsStatus
    );
}

#[test]
fn guild_vault_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::GuildVault).unwrap(),
        Request::GuildVaultStatus
    );
}

#[test]
fn warbank_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::Warbank).unwrap(),
        Request::WarbankStatus
    );
}

#[test]
fn equipped_gear_status_command_maps_to_request() {
    assert_eq!(
        status_request(StatusCmd::EquippedGear).unwrap(),
        Request::EquippedGearStatus
    );
}

#[test]
fn item_info_command_maps_to_request() {
    let request = item_request(ItemCmd::Info { item_id: 2589 }).expect("valid item command");
    assert_eq!(
        request,
        Request::ItemInfo {
            query: ItemInfoQuery { item_id: 2589 }
        }
    );
}

#[test]
fn spell_cast_command_maps_to_ipc_request() {
    let request = spell_request(SpellCmd::Cast {
        spell: "133".into(),
        target: Some("current".into()),
    })
    .expect("valid spell cast command");
    assert!(matches!(request, Request::SpellCast { .. }));
}

#[test]
fn inventory_search_command_maps_to_request() {
    let request = inventory_request(InventoryCmd::Search {
        text: "torch".into(),
    })
    .expect("valid inventory search command");
    assert_eq!(
        request,
        Request::InventorySearch {
            text: "torch".into()
        }
    );
}

#[test]
fn quest_list_command_maps_to_request() {
    assert_eq!(quest_request(QuestCmd::List).unwrap(), Request::QuestList);
}

#[test]
fn group_roster_command_maps_to_request() {
    assert_eq!(
        group_request(GroupCmd::Roster).unwrap(),
        Request::GroupRoster
    );
}

#[test]
fn map_waypoint_add_command_maps_to_request() {
    let request = map_request(MapCmd::Waypoint {
        command: WaypointCmd::Add { x: 42.1, y: 65.7 },
    })
    .expect("valid waypoint command");
    assert_eq!(request, Request::MapWaypointAdd { x: 42.1, y: 65.7 });
}

#[test]
fn combat_log_command_maps_to_request() {
    assert_eq!(
        combat_request(CombatCmd::Log { lines: 10 }).unwrap(),
        Request::CombatLog { lines: 10 }
    );
}

#[test]
fn reputation_list_command_maps_to_request() {
    assert_eq!(
        reputation_request(ReputationCmd::List).unwrap(),
        Request::ReputationList
    );
}

#[test]
fn collection_mounts_missing_command_maps_to_request() {
    let request = collection_request(CollectionCmd::Mounts { missing: true })
        .expect("valid collection mounts command");
    assert_eq!(request, Request::CollectionMounts { missing: true });
}

#[test]
fn profession_recipes_command_maps_to_request() {
    let request = profession_request(ProfessionCmd::Recipes {
        text: "potion".into(),
    })
    .expect("valid profession recipes command");
    assert_eq!(
        request,
        Request::ProfessionRecipes {
            text: "potion".into()
        }
    );
}

#[test]
fn map_target_command_maps_to_request() {
    assert_eq!(map_request(MapCmd::Target).unwrap(), Request::MapTarget);
}

#[test]
fn equipment_set_command_maps_to_request() {
    let request = equipment_request(EquipmentCmd::Set {
        slot: "mainhand".into(),
        model: PathBuf::from("data/models/club_1h_torch_a_01.m2"),
    })
    .expect("valid equipment set command");
    assert_eq!(
        request,
        Request::EquipmentSet {
            slot: "mainhand".into(),
            model_path: "data/models/club_1h_torch_a_01.m2".into(),
        }
    );
}

#[test]
fn equipment_clear_command_maps_to_request() {
    let request = equipment_request(EquipmentCmd::Clear {
        slot: "offhand".into(),
    })
    .expect("valid equipment clear command");
    assert_eq!(
        request,
        Request::EquipmentClear {
            slot: "offhand".into()
        }
    );
}

#[test]
fn export_character_command_maps_to_request() {
    let request = export_character_request(
        PathBuf::from("data/exports/thrall.json"),
        Some("Thrall".into()),
        Some(7),
    );
    assert_eq!(
        request,
        Request::ExportCharacter {
            output_path: "data/exports/thrall.json".into(),
            character_name: Some("Thrall".into()),
            character_id: Some(7),
        }
    );
}

#[test]
fn export_character_cli_command_parses_output_path() {
    let cli = crate::Cli::try_parse_from([
        "game-engine-cli",
        "export-character",
        "--name",
        "Thrall",
        "--character-id",
        "7",
        "data/exports/thrall.json",
    ])
    .expect("cli args should parse");

    assert!(matches!(
        cli.command,
        crate::Cmd::ExportCharacter {
            output,
            name,
            character_id,
        }
        if output == std::path::Path::new("data/exports/thrall.json")
            && name == Some("Thrall".into())
            && character_id == Some(7)
    ));
}

#[test]
fn export_character_payload_includes_stats_appearance_and_equipment() {
    let payload = build_export_character_payload(
        &CharacterStatsSnapshot {
            character_id: Some(7),
            name: Some("Thrall".into()),
            level: Some(60),
            race: Some(2),
            class: Some(7),
            appearance: Some(CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            }),
            health_current: Some(950.0),
            health_max: Some(1000.0),
            mana_current: Some(400.0),
            mana_max: Some(500.0),
            movement_speed: Some(7.0),
            zone_id: 12,
        },
        &EquippedGearStatusSnapshot {
            entries: vec![EquippedGearEntry {
                slot: "MainHand".into(),
                path: "data/models/club_1h_torch_a_01.m2".into(),
            }],
        },
        &EquipmentAppearanceStatusSnapshot {
            appearance: EquipmentAppearance {
                entries: vec![EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(6123),
                    display_info_id: Some(777),
                    inventory_type: 5,
                    hidden: false,
                }],
            },
        },
        &[],
        None,
        None,
    )
    .expect("payload should build");

    assert_eq!(
        payload,
        ExportCharacterPayload {
            character_id: 7,
            name: "Thrall".into(),
            level: 60,
            race: 2,
            class: 7,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            },
            zone_id: 12,
            health_current: Some(950.0),
            health_max: Some(1000.0),
            mana_current: Some(400.0),
            mana_max: Some(500.0),
            movement_speed: Some(7.0),
            equipped_gear: vec![EquippedGearEntry {
                slot: "MainHand".into(),
                path: "data/models/club_1h_torch_a_01.m2".into(),
            }],
            equipment_appearance: EquipmentAppearance {
                entries: vec![EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(6123),
                    display_info_id: Some(777),
                    inventory_type: 5,
                    hidden: false,
                }],
            },
        }
    );
}

#[test]
fn export_character_payload_requires_selected_character_identity() {
    let err = build_export_character_payload(
        &CharacterStatsSnapshot::default(),
        &EquippedGearStatusSnapshot::default(),
        &EquipmentAppearanceStatusSnapshot::default(),
        &[],
        None,
        None,
    )
    .expect_err("payload should reject missing character");

    assert!(err.contains("no selected character"));
}

#[test]
fn export_character_payload_resolves_from_character_list_by_name() {
    let payload = build_export_character_payload(
        &CharacterStatsSnapshot {
            zone_id: 12,
            ..Default::default()
        },
        &EquippedGearStatusSnapshot::default(),
        &EquipmentAppearanceStatusSnapshot::default(),
        &[shared::protocol::CharacterListEntry {
            character_id: 7,
            name: "Thrall".into(),
            level: 60,
            race: 2,
            class: 7,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 3,
                face: 4,
                eye_color: 0,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
            },
            equipment_appearance: EquipmentAppearance::default(),
        }],
        Some("Thrall"),
        None,
    )
    .expect("payload should build from character list");

    assert_eq!(payload.character_id, 7);
    assert_eq!(payload.name, "Thrall");
    assert_eq!(payload.level, 60);
}

#[test]
fn write_export_character_file_persists_pretty_json() {
    let output = unique_export_path("write-character-export");
    let payload = ExportCharacterPayload {
        character_id: 99,
        name: "Jaina".into(),
        level: 42,
        race: 1,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 1,
            face: 2,
            eye_color: 0,
            hair_style: 3,
            hair_color: 4,
            facial_style: 5,
        },
        zone_id: 1519,
        health_current: Some(123.0),
        health_max: Some(456.0),
        mana_current: Some(789.0),
        mana_max: Some(999.0),
        movement_speed: Some(7.0),
        equipped_gear: vec![],
        equipment_appearance: EquipmentAppearance::default(),
    };

    write_export_character_file(&output, &payload).expect("write should succeed");

    let written = std::fs::read_to_string(&output).expect("export file should exist");
    let parsed: ExportCharacterPayload =
        serde_json::from_str(&written).expect("written export should be valid json");
    assert_eq!(parsed, payload);
    assert!(written.contains("\n  \"name\": \"Jaina\""));

    let _ = std::fs::remove_file(&output);
    let _ = output.parent().map(std::fs::remove_dir_all);
}

#[test]
fn export_scene_request_uses_output_path() {
    let request = export_scene_request(PathBuf::from("data/debug/scene.json"));
    assert_eq!(
        request,
        Request::ExportScene {
            output_path: "data/debug/scene.json".into(),
        }
    );
}

#[test]
fn json_flag_parses_for_new_command_families() {
    let cli = crate::Cli::try_parse_from([
        "game-engine-cli",
        "--json",
        "inventory",
        "search",
        "--text",
        "torch",
    ])
    .expect("cli args should parse");

    assert!(cli.json);
    assert!(matches!(
        cli.command,
        crate::Cmd::Inventory {
            command: InventoryCmd::Search { .. }
        }
    ));
}

#[test]
fn text_response_serializes_in_json_mode() {
    let serialized =
        format_text_response_output(Response::Text("ok".into()), true).expect("json output");
    let parsed: Value = serde_json::from_str(&serialized).expect("valid json");
    assert_eq!(parsed["Text"], Value::String("ok".into()));
    assert!(parsed.get("ok").is_none());
    assert!(parsed.get("data").is_none());
}

#[test]
fn pong_response_serializes_in_json_mode_with_enum_shape() {
    let serialized = serialize_json(&Response::Pong).expect("json output");
    let parsed: Value = serde_json::from_str(&serialized).expect("valid json");
    assert_eq!(parsed["Pong"], Value::Null);
    assert!(parsed.get("ok").is_none());
    assert!(parsed.get("data").is_none());
}

#[test]
fn text_response_formatter_errors_for_non_text_in_text_mode() {
    let err = format_text_response_output(Response::Pong, false).expect_err("should fail");
    assert!(err.contains("unexpected response"));
}

#[test]
fn json_mode_roundtrips_ipc_and_keeps_enum_shape() {
    let socket = unique_test_socket("json-roundtrip");
    let server_expected = Request::InventorySearch {
        text: "torch".into(),
    };
    let client_request = Request::InventorySearch {
        text: "torch".into(),
    };
    let server = spawn_mock_server(
        socket.clone(),
        server_expected,
        Response::Text("inventory ok".into()),
    );

    let output = execute_text_request_output(&socket, client_request, true).expect("json output");
    let parsed: Value = serde_json::from_str(&output).expect("valid json");
    assert_eq!(parsed["Text"], Value::String("inventory ok".into()));

    server.join().expect("mock server thread");
}

#[test]
fn text_mode_roundtrips_ipc_and_returns_plain_text() {
    let socket = unique_test_socket("text-roundtrip");
    let server = spawn_mock_server(
        socket.clone(),
        Request::QuestList,
        Response::Text("quest list output".into()),
    );

    let output =
        execute_text_request_output(&socket, Request::QuestList, false).expect("text output");
    assert_eq!(output, "quest list output");

    server.join().expect("mock server thread");
}

fn unique_test_socket(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "game-engine-cli-{label}-{}-{nanos}.sock",
        std::process::id()
    ))
}

fn unique_export_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "game-engine-export-{label}-{}-{nanos}/character.json",
        std::process::id()
    ))
}

fn spawn_mock_server(
    socket: PathBuf,
    expected_request: Request,
    response: Response,
) -> std::thread::JoinHandle<()> {
    let (ready_tx, ready_rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let socket_for_runtime = socket.clone();
        runtime.block_on(async move {
            let server = Server::bind(&socket_for_runtime).expect("bind mock socket");
            ready_tx.send(()).expect("notify ready");
            let (mut conn, _) = server.accept().await.expect("accept connection");
            let got_request: Request = conn.read().await.expect("read request");
            assert_eq!(got_request, expected_request);
            conn.write(&response).await.expect("write response");
        });
        let _ = std::fs::remove_file(&socket);
    });

    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("mock server ready");
    handle
}
