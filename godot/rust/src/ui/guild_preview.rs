//! Offline guild ranks through the production settings screen.
use godot::prelude::*;

use super::{RegistryUi, party_preview};
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::GuildRankRequest;

#[derive(Clone)]
struct PendingGuildPreview(GuildRankRequest);

#[godot_api(secondary)]
impl RegistryUi {
    /// Offline authoritative guild snapshot through the production settings screen.
    #[func]
    pub fn show_guild_ranks_preview(&mut self) -> GString {
        self.show_guild_preview_skin(ui_toolkit::atlas::ActiveSkin::Modern)
    }

    #[func]
    pub fn show_forever_guild_ranks_preview(&mut self) -> GString {
        self.show_guild_preview_skin(ui_toolkit::atlas::ActiveSkin::Forever)
    }

    /// Physical pointer/Enter -> production input reader/reducer, without optimistic mutation.
    #[func]
    pub fn guild_control_preview_dispatch(&mut self) -> GString {
        let action = self.pop_action().to_string();
        if action.is_empty() {
            return GString::new();
        }
        let Some(model) = &self.model else {
            return GString::from("Guild preview not mounted");
        };
        let Some(mut state) = model.shared.get::<GuildRanksSession>().cloned() else {
            return GString::from("Guild preview state missing");
        };
        let texts = crate::guild_ranks::read_inputs_for_registry(&model.registry, &state);
        let request = state.click(&action, &texts);
        let result = request
            .as_ref()
            .map_or_else(String::new, |request| format!("{request:?}"));
        if let Err(error) = self.set_state(state) {
            return GString::from(error.as_str());
        };
        if let Some(request) = request {
            self.model
                .as_mut()
                .expect("guild preview")
                .shared
                .insert(PendingGuildPreview(request));
        }
        GString::from(result.as_str())
    }

    #[func]
    pub fn guild_control_preview_rights(&self) -> i64 {
        self.model
            .as_ref()
            .and_then(|model| model.shared.get::<GuildRanksSession>())
            .and_then(|session| session.selected())
            .map_or(0, |rank| i64::from(rank.rights))
    }

    #[func]
    pub fn guild_control_preview_waiting(&self) -> bool {
        self.model
            .as_ref()
            .and_then(|model| model.shared.get::<GuildRanksSession>())
            .is_some_and(|session| session.awaiting_state)
    }

    #[func]
    pub fn guild_control_preview_overflow(&mut self) -> GString {
        let Some(model) = self.model.as_ref() else {
            return GString::from("Guild preview not mounted");
        };
        let Some(mut session) = model.shared.get::<GuildRanksSession>().cloned() else {
            return GString::from("Guild preview state missing");
        };
        let Some(mut state) = session.state().cloned() else {
            return GString::from("No guild snapshot");
        };
        state.tab_names.extend(
            ["Weapons", "Armor", "Consumables", "Enchanting", "Overflow"].map(str::to_owned),
        );
        for rank in &mut state.ranks {
            rank.tabs.resize(
                8,
                shared::protocol::GuildRankTab {
                    view: true,
                    deposit: true,
                    withdrawals_per_day: 2,
                },
            );
        }
        session.apply(state);
        GString::from(self.set_state(session).err().unwrap_or_default().as_str())
    }

    /// Fixture reply boundary only; server authority is proven by persisted server tests.
    #[func]
    pub fn guild_control_preview_reply(&mut self) -> GString {
        GString::from(
            self.apply_preview_reply()
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn apply_preview_reply(&mut self) -> Result<(), String> {
        let model = self.model.as_ref().ok_or("Guild preview not mounted")?;
        let request = model
            .shared
            .get::<PendingGuildPreview>()
            .ok_or("No preview request awaiting reply")?
            .0
            .clone();
        let mut session = model
            .shared
            .get::<GuildRanksSession>()
            .ok_or("Guild preview state missing")?
            .clone();
        let mut reply = session.state().ok_or("No guild snapshot")?.clone();
        match request {
            GuildRankRequest::SetPermissions {
                rank,
                rights,
                gold_per_day,
            } => {
                let settings = reply
                    .ranks
                    .get_mut(usize::from(rank))
                    .ok_or("Preview rank missing")?;
                settings.rights = rights;
                settings.gold_per_day = gold_per_day;
            }
            GuildRankRequest::Rename { rank, name } => {
                reply
                    .ranks
                    .get_mut(usize::from(rank))
                    .ok_or("Preview rank missing")?
                    .name = name
            }
            GuildRankRequest::SetTab {
                rank,
                tab,
                view,
                deposit,
                withdrawals_per_day,
            } => {
                let settings = reply
                    .ranks
                    .get_mut(usize::from(rank))
                    .ok_or("Preview rank missing")?;
                let tab = settings
                    .tabs
                    .get_mut(usize::from(tab))
                    .ok_or("Preview tab missing")?;
                tab.view = view;
                tab.deposit = deposit;
                tab.withdrawals_per_day = withdrawals_per_day;
            }
            _ => return Err("Fixture reply does not implement this request".into()),
        }
        session.apply(reply);
        self.set_state(session)
    }

    fn show_guild_preview_skin(&mut self, skin: ui_toolkit::atlas::ActiveSkin) -> GString {
        if let Err(error) = party_preview::load_data_root() {
            return GString::from(error.as_str());
        }
        ui_toolkit::atlas::set_thread_skin(skin);
        GString::from(
            self.set_ui_scale(1.0)
                .and_then(|()| {
                    self.show_guild_ranks(game_engine_ui_model::guild_rank_frame::preview())
                })
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}
