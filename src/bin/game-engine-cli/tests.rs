use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use game_engine::ipc::{Request, Response};
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{DeleteMail, ListMailQuery, ReadMail, SendMail};
use peercred_ipc::Server;
use serde_json::Value;

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
