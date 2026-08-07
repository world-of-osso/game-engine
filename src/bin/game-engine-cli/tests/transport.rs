use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::diagnostic::{
    Diagnostic, DiagnosticMeasurement, DiagnosticsStore, FrameTimeDiagnosticsPlugin,
};
use bevy::platform::time::Instant;
use game_engine::ipc::{PerformanceSnapshot, Request, Response, build_performance_snapshot};

use super::*;
use crate::command_dispatch::{
    execute_performance_request_output, format_performance_response_output,
};
use peercred_ipc::Server;

#[test]
fn performance_command_parses_as_a_top_level_cli_command() {
    let cli = crate::Cli::try_parse_from(["game-engine-cli", "performance"])
        .expect("performance command should parse");

    assert!(matches!(cli.command, crate::Cmd::Performance));
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
fn performance_request_roundtrips_over_ipc_and_formats_text() {
    let socket = unique_test_socket("performance-roundtrip");
    let response = Response::Performance(PerformanceSnapshot {
        fps: Some(59.5),
        frame_time_ms: Some(16.8),
        focused: true,
    });
    let server = spawn_mock_server(socket.clone(), Request::Performance, response);

    let output = execute_performance_request_output(&socket, Request::Performance, false)
        .expect("performance output");

    assert_eq!(output, "fps=59.50 frame_time_ms=16.80 focused=true");
    server.join().expect("mock server thread");
}

#[test]
fn performance_response_formats_text_and_json() {
    let response = Response::Performance(PerformanceSnapshot {
        fps: Some(59.5),
        frame_time_ms: Some(16.755),
        focused: false,
    });

    let text = format_performance_response_output(response, false).expect("text output");
    assert_eq!(text, "fps=59.50 frame_time_ms=16.76 focused=false");

    let json = format_performance_response_output(
        Response::Performance(PerformanceSnapshot {
            fps: None,
            frame_time_ms: Some(33.333),
            focused: false,
        }),
        true,
    )
    .expect("json output");
    let parsed: Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(parsed["Performance"]["fps"], Value::Null);
    assert_eq!(parsed["Performance"]["frame_time_ms"], 33.333);
    assert_eq!(parsed["Performance"]["focused"], false);
}

#[test]
fn performance_snapshot_extracts_smoothed_diagnostics_and_focus() {
    let start = Instant::now();
    let mut fps = Diagnostic::new(FrameTimeDiagnosticsPlugin::FPS)
        .with_max_history_length(8)
        .with_smoothing_factor(1.0);
    fps.add_measurement(DiagnosticMeasurement {
        time: start,
        value: 60.0,
    });
    fps.add_measurement(DiagnosticMeasurement {
        time: start + Duration::from_millis(500),
        value: 30.0,
    });

    let mut frame_time = Diagnostic::new(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .with_max_history_length(8)
        .with_smoothing_factor(1.0);
    frame_time.add_measurement(DiagnosticMeasurement {
        time: start,
        value: 16.0,
    });
    frame_time.add_measurement(DiagnosticMeasurement {
        time: start + Duration::from_millis(500),
        value: 20.0,
    });

    let mut diagnostics = DiagnosticsStore::default();
    diagnostics.add(fps);
    diagnostics.add(frame_time);

    let snapshot = build_performance_snapshot(&diagnostics, true);

    assert_eq!(snapshot.fps, Some(45.0));
    assert_eq!(snapshot.frame_time_ms, Some(18.0));
    assert!(snapshot.focused);
}

#[test]
fn performance_snapshot_reports_unavailable_diagnostics() {
    let snapshot = build_performance_snapshot(&DiagnosticsStore::default(), false);

    assert_eq!(snapshot.fps, None);
    assert_eq!(snapshot.frame_time_ms, None);
    assert!(!snapshot.focused);
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
