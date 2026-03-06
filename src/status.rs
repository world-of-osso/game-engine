use serde::{Deserialize, Serialize};

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkStatusSnapshot {
    pub server_addr: Option<String>,
    pub game_state: String,
    pub connected: bool,
    pub connected_links: usize,
    pub local_client_id: Option<u64>,
    pub zone_id: u32,
    pub remote_entities: usize,
    pub local_players: usize,
    pub chat_messages: usize,
}

impl Default for NetworkStatusSnapshot {
    fn default() -> Self {
        Self {
            server_addr: None,
            game_state: "Unavailable".into(),
            connected: false,
            connected_links: 0,
            local_client_id: None,
            zone_id: 0,
            remote_entities: 0,
            local_players: 0,
            chat_messages: 0,
        }
    }
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainStatusSnapshot {
    pub map_name: String,
    pub initial_tile: (u32, u32),
    pub load_radius: u32,
    pub loaded_tiles: usize,
    pub pending_tiles: usize,
    pub failed_tiles: usize,
    pub server_requested_tiles: usize,
    pub heightmap_tiles: usize,
}

impl Default for TerrainStatusSnapshot {
    fn default() -> Self {
        Self {
            map_name: String::new(),
            initial_tile: (0, 0),
            load_radius: 0,
            loaded_tiles: 0,
            pending_tiles: 0,
            failed_tiles: 0,
            server_requested_tiles: 0,
            heightmap_tiles: 0,
        }
    }
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundStatusSnapshot {
    pub enabled: bool,
    pub muted: bool,
    pub master_volume: f32,
    pub footstep_volume: f32,
    pub ambient_volume: f32,
    pub ambient_entities: usize,
    pub active_sinks: usize,
}

impl Default for SoundStatusSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            muted: false,
            master_volume: 1.0,
            footstep_volume: 0.5,
            ambient_volume: 0.3,
            ambient_entities: 0,
            active_sinks: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrencyEntry {
    pub id: u32,
    pub name: String,
    pub amount: u64,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CurrenciesStatusSnapshot {
    pub entries: Vec<CurrencyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReputationEntry {
    pub faction_id: u32,
    pub faction_name: String,
    pub standing: String,
    pub value: i32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ReputationsStatusSnapshot {
    pub entries: Vec<ReputationEntry>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterStatsSnapshot {
    pub name: Option<String>,
    pub level: Option<u16>,
    pub race: Option<u8>,
    pub class: Option<u8>,
    pub health_current: Option<f32>,
    pub health_max: Option<f32>,
    pub mana_current: Option<f32>,
    pub mana_max: Option<f32>,
    pub movement_speed: Option<f32>,
    pub zone_id: u32,
}

impl Default for CharacterStatsSnapshot {
    fn default() -> Self {
        Self {
            name: None,
            level: None,
            race: None,
            class: None,
            health_current: None,
            health_max: None,
            mana_current: None,
            mana_max: None,
            movement_speed: None,
            zone_id: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageItemEntry {
    pub slot: u32,
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub stack_count: u32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GuildVaultStatusSnapshot {
    pub entries: Vec<StorageItemEntry>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WarbankStatusSnapshot {
    pub entries: Vec<StorageItemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquippedGearEntry {
    pub slot: String,
    pub path: String,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EquippedGearStatusSnapshot {
    pub entries: Vec<EquippedGearEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryItemEntry {
    pub storage: String,
    pub slot: u32,
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub stack_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InventorySearchSnapshot {
    pub entries: Vec<InventoryItemEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_status_defaults_to_disconnected() {
        let snapshot = NetworkStatusSnapshot::default();

        assert!(!snapshot.connected);
        assert_eq!(snapshot.game_state, "Unavailable");
        assert_eq!(snapshot.zone_id, 0);
    }

    #[test]
    fn terrain_status_defaults_to_empty_streaming_state() {
        let snapshot = TerrainStatusSnapshot::default();

        assert!(snapshot.map_name.is_empty());
        assert_eq!(snapshot.loaded_tiles, 0);
        assert_eq!(snapshot.heightmap_tiles, 0);
    }

    #[test]
    fn sound_status_defaults_to_disabled() {
        let snapshot = SoundStatusSnapshot::default();

        assert!(!snapshot.enabled);
        assert!(!snapshot.muted);
        assert_eq!(snapshot.master_volume, 1.0);
        assert_eq!(snapshot.ambient_entities, 0);
    }

    #[test]
    fn currencies_status_defaults_to_empty_list() {
        let snapshot = CurrenciesStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn reputations_status_defaults_to_empty_list() {
        let snapshot = ReputationsStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn character_stats_defaults_to_unknown_character() {
        let snapshot = CharacterStatsSnapshot::default();

        assert!(snapshot.name.is_none());
        assert!(snapshot.level.is_none());
        assert_eq!(snapshot.zone_id, 0);
    }

    #[test]
    fn guild_vault_defaults_to_empty_list() {
        let snapshot = GuildVaultStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn warbank_defaults_to_empty_list() {
        let snapshot = WarbankStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn equipped_gear_defaults_to_empty_list() {
        let snapshot = EquippedGearStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn inventory_search_snapshot_defaults_to_empty_entries() {
        let snapshot = InventorySearchSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }
}
