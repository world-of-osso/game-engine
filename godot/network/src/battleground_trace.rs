//! Private BG_NETWORK_TRACE diagnostics use Lightyear's built-in UDP/message traces.
//! No protocol fields, channel settings, deadlines or gameplay decisions change.
use shared::protocol::PvpStateUpdate;
use std::{
    any::Any,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

fn enabled() -> bool {
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
                .with_env_filter("lightyear_debug::message=trace,lightyear_udp=trace")
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .try_init()
                .map_err(|error| {
                    format!("Initialize private battleground transport trace: {error}")
                })
        })
        .clone()
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
    let unix_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_millis(),
        Err(error) => {
            eprintln!("Cannot timestamp battleground relay receipt: {error}");
            return;
        }
    };
    eprintln!(
        "BG_TRACE_RELAY unix_ms={unix_ms} match={} bg={} phase={:?} score_a={} score_b={}",
        bg.match_id, bg.battleground_id, bg.phase, bg.score_a, bg.score_b
    );
}
