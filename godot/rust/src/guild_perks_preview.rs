//! Read-only receipt of real owner spell messages and current chat input; no fixture injection.
use crate::GameClient;
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    pub fn guild_perks_observation(&self) -> GString {
        let json = serde_json::to_string(&serde_json::json!({
            "known_spells": self.account.spells.known(),
            "guild_id": self.guild_ranks.session.profile.as_ref().map(|profile| profile.guild_id),
            "chat_open": self.chat.model.state.input_open,
            "draft_visible": self.chat.draft.visible(),
            "draft_wire": self.chat.draft.wire(),
        }))
        .expect("serialize actual perk knowledge/chat receipt");
        GString::from(json.as_str())
    }
}
