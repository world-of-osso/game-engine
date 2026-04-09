use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;
use peercred_ipc::Server;

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
