//! Read-only receipt of the actual server reply currently rendered by GuildRanksUI.
//! No fixture history injection: the private lifecycle acceptance uses real packets.
use super::RegistryUi;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use godot::prelude::*;

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn guild_event_log_observation(&self) -> GString {
        let Some(session) = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<GuildRanksSession>())
        else {
            return GString::new();
        };
        let json = serde_json::to_string(&serde_json::json!({
            "open": session.log_open,
            "waiting": session.log_waiting,
            "reply": session.event_log.as_ref(),
        }))
        .expect("serialize the rendered guild history receipt");
        GString::from(json.as_str())
    }
}
