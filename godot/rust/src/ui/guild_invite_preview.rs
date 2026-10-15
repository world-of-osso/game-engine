//! Offline invitation chrome/input fixture; never claims server authority.
use super::RegistryUi;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use godot::prelude::*;
use shared::protocol::GuildInvitation;

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_guild_invitation_preview(&mut self, forever: bool) -> GString {
        let error = if forever {
            self.show_forever_guild_ranks_preview()
        } else {
            self.show_guild_ranks_preview()
        };
        if !error.is_empty() {
            return error;
        }
        let mut session = GuildRanksSession::default();
        session.invitation = Some(GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
        });
        GString::from(self.set_state(session).err().unwrap_or_default().as_str())
    }
}
