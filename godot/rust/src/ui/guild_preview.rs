//! Offline guild ranks through the production settings screen.
use godot::prelude::*;

use super::{RegistryUi, party_preview};
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use shared::protocol::GuildRankRequest;

#[derive(Clone)]
struct PendingGuildPreview(GuildRankRequest);

#[cfg(debug_assertions)]
impl RegistryUi {
    pub(crate) fn update_guild_communities_design(
        &mut self,
        design: shared::guild_creation::GuildTabardDesign,
    ) -> Result<(), String> {
        let mut session = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<GuildRanksSession>())
            .cloned()
            .ok_or("Missing Communities fixture")?;
        session
            .profile
            .as_mut()
            .ok_or("Missing guild snapshot")?
            .tabard = Some(design);
        self.set_state(session)
    }
}

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

    /// Overflowing roster fixture through the production Communities screen and input reducer.
    #[func]
    pub fn show_guild_communities_preview(&mut self, forever: bool) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let error = self.show_guild_preview_skin(skin);
        if !error.is_empty() {
            return error;
        }
        let mut session = game_engine_ui_model::guild_rank_frame::preview();
        session.settings_open = false;
        let mut state = session.state().expect("guild fixture").clone();
        state.members = (0..32)
            .map(|index| shared::protocol::GuildRankMember {
                character_name: format!("Member{index:02}"),
                rank: if index == 0 { 0 } else { 2 },
            })
            .collect();
        session.profile = Some(shared::protocol::GuildSnapshot {
            tabard: None,
            guild_id: 7,
            guild_name: "Adventurers' Rest".into(),
            created_at: None,
            motd: "Meet at the guild hall tonight.".into(),
            info_text: "Tuesday raid\nThursday dungeon".into(),
            members: state
                .members
                .iter()
                .enumerate()
                .map(|(index, member)| shared::protocol::GuildMemberSnapshot {
                    character_name: member.character_name.clone(),
                    level: 80,
                    class_name: ["Paladin", "Mage", "Druid", "Warrior"][index % 4].into(),
                    rank_name: "Raider".into(),
                    is_online: index % 3 != 1,
                    officer_note: "Officer-only fixture note".into(),
                    public_note: String::new(),
                    is_self: false,
                    last_online: "Yesterday".into(),
                })
                .collect(),
        });
        session.apply(state);
        GString::from(self.set_state(session).err().unwrap_or_default().as_str())
    }

    #[func]
    pub fn guild_communities_preview_receipt(&self) -> GString {
        let Some(session) = self
            .model
            .as_ref()
            .and_then(|m| m.shared.get::<GuildRanksSession>())
        else {
            return GString::from("Guild preview state missing");
        };
        GString::from(
            format!(
                "member={:?};commands={:?};pointer_enabled={}",
                session.member_menu,
                session.commands,
                crate::guild_ranks::guild_pointer_enabled(session, false)
            )
            .as_str(),
        )
    }

    /// Physical pointer/Enter -> production input reader/reducer, without optimistic mutation.
    #[func]
    pub fn guild_control_preview_dispatch(&mut self) -> GString {
        let Some(model) = &self.model else {
            return GString::from("Guild preview not mounted");
        };
        let Some(mut state) = model.shared.get::<GuildRanksSession>().cloned() else {
            return GString::from("Guild preview state missing");
        };
        let mut request = None;
        loop {
            let action = self.pop_action().to_string();
            if action.is_empty() {
                break;
            }
            let model = self.model.as_ref().expect("guild preview");
            let texts = crate::guild_ranks::read_inputs_for_registry(&model.registry, &state);
            if let Some(next) = state.click(&action, &texts) {
                request = Some(next);
            }
        }
        // Mirror GameClient::update_guild_ranks: right releases use the alt-click queue.
        while let Some((action, right, _)) = self.pop_alt_click() {
            if right && action.starts_with("guild:member:") {
                state.click(
                    &action.replacen("guild:member:", "guild:member-menu:", 1),
                    &Default::default(),
                );
            }
        }
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
