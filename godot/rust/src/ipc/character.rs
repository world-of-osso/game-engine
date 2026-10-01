//! `status character-stats` in the original response text, re-stated because the
//! original formats the Bevy `CharacterStatsSnapshot` (src/ipc/format_status.rs:503), from the selected roster entry, the local player's
//! replicated unit, the server's rest state and the current zone, as the original
//! `sync_character_stats_snapshot` and `receive_rest_state_update` fill it.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::{
    auction_house_data::Money,
    status::{SecondaryResourceEntry, SecondaryResourceKindEntry},
};
use shared::{
    components::{
        CombatStatus, Gold, Health, Mana, MovementSpeed, Player, PresenceStatus, UnitPowers,
    },
    protocol::RestAreaKindSnapshot,
};

impl crate::GameClient {
    pub(crate) fn character_request(&mut self, request: Request) -> Result<Response, Request> {
        match request {
            Request::CharacterStatsStatus => Ok(Response::Text(self.character_stats())),
            request => Err(request),
        }
    }

    fn character_stats(&self) -> String {
        let session = &self.account.session;
        let selected = session.selected_character_id.and_then(|id| {
            session
                .characters
                .iter()
                .find(|entry| entry.character_id == id)
        });
        let unit = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let name = selected.map(|entry| entry.name.clone()).or_else(|| {
            unit.and_then(|unit| unit.get::<Player>())
                .map(|player| player.name.clone())
        });
        let health = unit.and_then(|unit| unit.get::<Health>());
        let mana = unit.and_then(|unit| unit.get::<Mana>());
        let rest = self.rest.as_ref();
        format!(
            "name: {}\nlevel: {}\nrace: {}\nclass: {}\nhealth: {}/{}\nmana: {}/{}\nsecondary_resource: {}\nmovement_speed: {}\ngold: {}\npresence: {}\nin_combat: {}\nin_rest_area: {}\nrest_area_kind: {}\nrested_xp: {}\nrested_xp_max: {}\nzone_id: {}",
            name.as_deref().unwrap_or("-"),
            optional(selected.map(|entry| entry.level)),
            optional(selected.map(|entry| entry.race)),
            optional(selected.map(|entry| entry.class)),
            whole(health.map(|health| health.current)),
            whole(health.map(|health| health.max)),
            whole(mana.map(|mana| mana.current)),
            whole(mana.map(|mana| mana.max)),
            secondary_resource(
                unit.and_then(|unit| unit.get::<UnitPowers>())
                    .and_then(SecondaryResourceEntry::from_unit_powers)
            ),
            unit.and_then(|unit| unit.get::<MovementSpeed>())
                .map_or_else(|| "-".into(), |speed| format!("{:.2}", speed.0)),
            Money(
                unit.and_then(|unit| unit.get::<Gold>())
                    .map_or(0, |gold| gold.0)
            )
            .display(),
            presence(unit.and_then(|unit| unit.get::<PresenceStatus>())),
            unit.and_then(|unit| unit.get::<CombatStatus>())
                .is_some_and(|status| status.0),
            rest.is_some_and(|rest| rest.in_rest_area),
            match rest.and_then(|rest| rest.rest_area_kind.as_ref()) {
                Some(RestAreaKindSnapshot::City) => "city",
                Some(RestAreaKindSnapshot::Inn) => "inn",
                None => "-",
            },
            rest.map_or(0, |rest| rest.rested_xp),
            rest.map_or(0, |rest| rest.rested_xp_max),
            self.current_zone_id().unwrap_or(0),
        )
    }
}

fn optional(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(|| "-".into(), |value| value.to_string())
}

fn whole(value: Option<f32>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{value:.0}"))
}

fn presence(value: Option<&PresenceStatus>) -> &'static str {
    match value {
        Some(PresenceStatus::Online) => "online",
        Some(PresenceStatus::Afk) => "afk",
        Some(PresenceStatus::Dnd) => "dnd",
        Some(PresenceStatus::Offline) => "offline",
        None => "-",
    }
}

fn secondary_resource(value: Option<SecondaryResourceEntry>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    let kind = match value.kind {
        SecondaryResourceKindEntry::ComboPoints => "combo_points",
        SecondaryResourceKindEntry::HolyPower => "holy_power",
        SecondaryResourceKindEntry::Chi => "chi",
        SecondaryResourceKindEntry::Essence => "essence",
        SecondaryResourceKindEntry::SoulShards => "soul_shards",
        SecondaryResourceKindEntry::ArcaneCharges => "arcane_charges",
        SecondaryResourceKindEntry::Runes => "runes",
    };
    format!("{kind} {}/{}", value.current, value.max)
}
