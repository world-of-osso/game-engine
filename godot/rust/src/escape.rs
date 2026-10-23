//! Exact domain adapters for authored global Escape handlers; core owns stable dispatch.
use crate::{GameClient, frame_error::FrameError};
use game_engine_ui_model::panel_lifecycle::ESCAPE_BINDINGS;
use ui_toolkit::lifecycle::dispatch_escape;

/// Preserve account-failure authority and the menu's existing nonfatal logging path.
pub(super) enum EscapeError {
    Frame(&'static str, FrameError),
    GameMenu(String),
}

type EscapeCallback = fn(&mut GameClient) -> Result<bool, EscapeError>;

/// No frame-name inference: priorities and membership live only in authored bindings.
const ESCAPE_CALLBACKS: &[(&str, EscapeCallback)] = &[
    ("talent_loadout_dialog", |client| {
        Ok(client.account.talents.cancel_loadout_dialog())
    }),
    ("guild_invitation", |client| {
        client
            .dismiss_guild_invitation()
            .map_err(|error| EscapeError::Frame("Guild invitation dismissal", error))
    }),
    ("trading_slow_purchase", |client| {
        Ok(client.dismiss_trading_slow_purchase())
    }),
    ("ground_target", |client| Ok(client.cancel_ground_target())),
    ("pet_move_to", |client| Ok(client.cancel_pet_move_to())),
    ("game_menu", |client| {
        client
            .apply_game_menu_escape()
            .map_err(EscapeError::GameMenu)
    }),
    ("all_windows", |client| {
        client
            .close_all_windows()
            .map_err(|error| EscapeError::Frame("Close all windows", error))
    }),
    ("clear_target", |client| Ok(client.clear_target_on_escape())),
];

impl GameClient {
    pub(super) fn dispatch_template_escape(&mut self) -> Result<bool, EscapeError> {
        dispatch_escape(ESCAPE_BINDINGS, self, Self::invoke_escape_hook)
    }

    fn invoke_escape_hook(&mut self, hook: &str) -> Result<bool, EscapeError> {
        let (_, callback) = ESCAPE_CALLBACKS
            .iter()
            .find(|(key, _)| *key == hook)
            .ok_or_else(|| {
                EscapeError::Frame(
                    "Escape dispatch",
                    format!("Unknown authored Escape hook: {hook}").into(),
                )
            })?;
        callback(self)
    }
}
