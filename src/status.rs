use serde::{Deserialize, Serialize};
use shared::components::{CharacterAppearance, EquipmentAppearance};

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

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
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

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CharacterStatsSnapshot {
    pub character_id: Option<u64>,
    pub name: Option<String>,
    pub level: Option<u16>,
    pub race: Option<u8>,
    pub class: Option<u8>,
    pub appearance: Option<CharacterAppearance>,
    pub health_current: Option<f32>,
    pub health_max: Option<f32>,
    pub mana_current: Option<f32>,
    pub mana_max: Option<f32>,
    pub movement_speed: Option<f32>,
    pub zone_id: u32,
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

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EquipmentAppearanceStatusSnapshot {
    pub appearance: EquipmentAppearance,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestRepeatability {
    Normal,
    Daily,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestObjectiveEntry {
    pub text: String,
    pub current: u32,
    pub required: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestEntry {
    pub quest_id: u32,
    pub title: String,
    pub zone: String,
    pub completed: bool,
    pub repeatability: QuestRepeatability,
    pub objectives: Vec<QuestObjectiveEntry>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct QuestLogStatusSnapshot {
    pub entries: Vec<QuestEntry>,
    pub watched_quest_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupRole {
    Tank,
    Healer,
    Damage,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMemberEntry {
    pub name: String,
    pub role: GroupRole,
    pub is_leader: bool,
    pub online: bool,
    pub subgroup: u8,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GroupStatusSnapshot {
    pub is_raid: bool,
    pub members: Vec<GroupMemberEntry>,
    pub ready_count: u16,
    pub total_count: u16,
    pub last_server_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CombatLogEventKind {
    Damage,
    Heal,
    Interrupt,
    AuraApplied,
    Death,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CombatLogEntry {
    pub kind: CombatLogEventKind,
    pub source: String,
    pub target: String,
    pub spell: Option<String>,
    pub amount: Option<i32>,
    pub aura: Option<String>,
    pub text: String,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CombatLogStatusSnapshot {
    pub entries: Vec<CombatLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMountEntry {
    pub mount_id: u32,
    pub name: String,
    pub known: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionPetEntry {
    pub pet_id: u32,
    pub name: String,
    pub known: bool,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CollectionStatusSnapshot {
    pub mounts: Vec<CollectionMountEntry>,
    pub pets: Vec<CollectionPetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionRecipeEntry {
    pub spell_id: u32,
    pub profession: String,
    pub name: String,
    pub craftable: bool,
    pub cooldown: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProfessionStatusSnapshot {
    pub recipes: Vec<ProfessionRecipeEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Waypoint {
    pub x: f32,
    pub y: f32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapStatusSnapshot {
    pub zone_id: u32,
    pub player_x: f32,
    pub player_z: f32,
    pub waypoint: Option<Waypoint>,
}

impl Default for MapStatusSnapshot {
    fn default() -> Self {
        Self {
            zone_id: 0,
            player_x: 0.0,
            player_z: 0.0,
            waypoint: None,
        }
    }
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

    #[test]
    fn quest_log_snapshot_defaults_to_empty_entries() {
        let snapshot = QuestLogStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
        assert!(snapshot.watched_quest_ids.is_empty());
    }

    #[test]
    fn group_status_snapshot_defaults_to_empty_members() {
        let snapshot = GroupStatusSnapshot::default();

        assert!(snapshot.members.is_empty());
        assert_eq!(snapshot.ready_count, 0);
        assert_eq!(snapshot.total_count, 0);
    }

    #[test]
    fn combat_log_snapshot_defaults_to_no_entries() {
        let snapshot = CombatLogStatusSnapshot::default();

        assert!(snapshot.entries.is_empty());
    }

    #[test]
    fn collection_snapshot_defaults_to_empty_lists() {
        let snapshot = CollectionStatusSnapshot::default();

        assert!(snapshot.mounts.is_empty());
        assert!(snapshot.pets.is_empty());
    }

    #[test]
    fn profession_snapshot_defaults_to_empty_recipes() {
        let snapshot = ProfessionStatusSnapshot::default();

        assert!(snapshot.recipes.is_empty());
    }

    #[test]
    fn map_status_snapshot_defaults_to_origin() {
        let snapshot = MapStatusSnapshot::default();

        assert_eq!(snapshot.zone_id, 0);
        assert_eq!(snapshot.player_x, 0.0);
        assert_eq!(snapshot.player_z, 0.0);
        assert!(snapshot.waypoint.is_none());
    }
}
