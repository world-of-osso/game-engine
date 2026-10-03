//! Server-driven windows: an inspect session opens its window, and
//! the window manager closing it (eviction, Escape) ends the session data.

use bevy::prelude::*;
use game_engine::status::InspectStatusSnapshot;

use super::{WindowId, WindowManager};

/// Reconciles one session with its window. `was_active` is the session state
/// seen on the previous run, so only a session start opens the window.
fn reconcile_session(
    manager: &mut ResMut<WindowManager>,
    id: WindowId,
    active: bool,
    was_active: &mut bool,
    end_session: impl FnOnce(),
) {
    let window_open = manager.is_open(id);
    if active && !*was_active {
        manager.open(id);
    } else if active && !window_open {
        end_session();
    } else if !active && window_open {
        manager.close(id);
    }
    *was_active = active;
}

pub fn sync_inspect_window(
    mut manager: ResMut<WindowManager>,
    snapshot: Option<ResMut<InspectStatusSnapshot>>,
    mut was_active: Local<bool>,
) {
    let Some(mut snapshot) = snapshot else { return };
    let active = snapshot.target_name.is_some();
    reconcile_session(
        &mut manager,
        WindowId::Inspect,
        active,
        &mut was_active,
        || *snapshot = InspectStatusSnapshot::default(),
    );
}
