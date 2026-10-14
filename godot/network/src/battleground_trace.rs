//! Private BG_NETWORK_TRACE diagnostics use Lightyear's built-in UDP/message traces.
//! No protocol fields, channel settings, deadlines or gameplay decisions change.
use shared::protocol::PvpStateUpdate;
use std::{
    any::Any,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("BG_NETWORK_TRACE").is_some())
}

pub(crate) fn configure() -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    static SUBSCRIBER: OnceLock<Result<(), String>> = OnceLock::new();
    SUBSCRIBER
        .get_or_init(|| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    "lightyear_debug::message=trace,lightyear_udp=trace,lightyear_netcode=debug,lightyear_connection=debug",
                )
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .try_init()
                .map_err(|error| {
                    format!("Initialize private battleground transport trace: {error}")
                })
        })
        .clone()
}

fn read_unix_millis() -> Option<u128> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => Some(elapsed.as_millis()),
        Err(error) => {
            eprintln!("Cannot timestamp battleground transport diagnostic: {error}");
            None
        }
    }
}

/// Sampling immediately after UDP buffer-to-link, before Netcode decrypt/reassembly.
/// A batch timestamp bounds socket receipt; typed-message traces identify delivery.
pub(crate) fn record_socket_receive(
    links: bevy::prelude::Query<
        &lightyear::prelude::Link,
        bevy::prelude::With<lightyear::prelude::UdpIo>,
    >,
) {
    for link in &links {
        let packets = link.recv.len();
        if packets == 0 {
            continue;
        }
        if let Some(unix_ms) = read_unix_millis() {
            eprintln!("BG_TRACE_SOCKET_BATCH unix_ms={unix_ms} packets={packets}");
        }
    }
}

/// Record the exact reason delivered to the host before it resets the world.
pub(crate) fn record_disconnect(reason: Option<&str>, handshake_timed_out: bool) {
    if !enabled() {
        return;
    }
    if let Some(unix_ms) = read_unix_millis() {
        eprintln!(
            "BG_TRACE_DISCONNECTED unix_ms={unix_ms} handshake_timed_out={handshake_timed_out} reason={reason:?}"
        );
    }
}

pub(crate) fn record_relay(message: &dyn Any) {
    if !enabled() {
        return;
    }
    let Some(update) = message.downcast_ref::<PvpStateUpdate>() else {
        return;
    };
    let Some(bg) = update
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.battleground.as_ref())
    else {
        return;
    };
    let Some(unix_ms) = read_unix_millis() else {
        return;
    };
    eprintln!(
        "BG_TRACE_RELAY unix_ms={unix_ms} match={} bg={} phase={:?} score_a={} score_b={}",
        bg.match_id, bg.battleground_id, bg.phase, bg.score_a, bg.score_b
    );
}
