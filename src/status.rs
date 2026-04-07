use serde::{Deserialize, Serialize};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::CharacterListEntry;

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
    pub process_rss_kb: u64,
    pub process_anon_kb: u64,
    pub process_data_kb: u64,
    pub m2_model_cache_entries: usize,
    pub m2_model_cache_est_cpu_bytes: u64,
    pub composited_texture_cache_entries: usize,
    pub composited_texture_cache_est_cpu_bytes: u64,
    pub image_assets: usize,
    pub image_asset_cpu_bytes: u64,
    pub mesh_assets: usize,
    pub mesh_asset_est_cpu_bytes: u64,
    pub standard_material_assets: usize,
    pub terrain_material_assets: usize,
    pub water_material_assets: usize,
    pub m2_effect_material_assets: usize,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundStatusSnapshot {
    pub enabled: bool,
    pub muted: bool,
    pub master_volume: f32,
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

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CharacterRosterStatusSnapshot {
    pub entries: Vec<CharacterListEntry>,
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

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
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

    // --- Serialization round-trip tests ---

    fn round_trip<
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    >(
        val: &T,
    ) {
        let json = serde_json::to_string(val).expect("serialize");
        let deserialized: T = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*val, deserialized);
    }

    #[test]
    fn network_status_round_trip() {
        let snapshot = NetworkStatusSnapshot {
            server_addr: Some("127.0.0.1:5000".into()),
            game_state: "InWorld".into(),
            connected: true,
            connected_links: 2,
            local_client_id: Some(42),
            zone_id: 12,
            remote_entities: 15,
            local_players: 1,
            chat_messages: 5,
        };
        round_trip(&snapshot);
    }

    #[test]
    fn terrain_status_round_trip() {
        let snapshot = TerrainStatusSnapshot {
            map_name: "azeroth".into(),
            initial_tile: (32, 48),
            load_radius: 3,
            loaded_tiles: 12,
            pending_tiles: 2,
            failed_tiles: 1,
            ..Default::default()
        };
        round_trip(&snapshot);
    }

    #[test]
    fn currencies_status_round_trip() {
        let snapshot = CurrenciesStatusSnapshot {
            entries: vec![
                CurrencyEntry {
                    id: 1,
                    name: "Honor".into(),
                    amount: 15000,
                },
                CurrencyEntry {
                    id: 2,
                    name: "Conquest".into(),
                    amount: 1800,
                },
            ],
        };
        round_trip(&snapshot);
    }

    #[test]
    fn character_stats_round_trip() {
        let snapshot = CharacterStatsSnapshot {
            character_id: Some(99),
            name: Some("Tankadin".into()),
            level: Some(60),
            race: Some(1),
            class: Some(2),
            health_current: Some(5000.0),
            health_max: Some(5000.0),
            mana_current: Some(3000.0),
            mana_max: Some(4000.0),
            movement_speed: Some(7.0),
            zone_id: 12,
            ..Default::default()
        };
        round_trip(&snapshot);
    }

    #[test]
    fn quest_log_round_trip() {
        let snapshot = QuestLogStatusSnapshot {
            entries: vec![QuestEntry {
                quest_id: 100,
                title: "The Defias Brotherhood".into(),
                zone: "Westfall".into(),
                completed: false,
                repeatability: QuestRepeatability::Normal,
                objectives: vec![QuestObjectiveEntry {
                    text: "Kill 10 Defias".into(),
                    current: 5,
                    required: 10,
                    completed: false,
                }],
            }],
            watched_quest_ids: vec![100],
        };
        round_trip(&snapshot);
    }

    #[test]
    fn group_status_round_trip() {
        let snapshot = GroupStatusSnapshot {
            is_raid: false,
            members: vec![GroupMemberEntry {
                name: "Bob".into(),
                role: GroupRole::Damage,
                is_leader: false,
                online: true,
                subgroup: 1,
            }],
            ready_count: 1,
            total_count: 1,
            last_server_message: None,
        };
        round_trip(&snapshot);
    }

    #[test]
    fn default_snapshots_round_trip() {
        round_trip(&NetworkStatusSnapshot::default());
        round_trip(&TerrainStatusSnapshot::default());
        round_trip(&SoundStatusSnapshot::default());
        round_trip(&CurrenciesStatusSnapshot::default());
        round_trip(&ReputationsStatusSnapshot::default());
        round_trip(&CharacterStatsSnapshot::default());
        round_trip(&GuildVaultStatusSnapshot::default());
        round_trip(&WarbankStatusSnapshot::default());
        round_trip(&QuestLogStatusSnapshot::default());
        round_trip(&GroupStatusSnapshot::default());
        round_trip(&CombatLogStatusSnapshot::default());
    }

    #[test]
    fn network_status_preserves_none_fields() {
        let snapshot = NetworkStatusSnapshot {
            server_addr: None,
            local_client_id: None,
            ..Default::default()
        };
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let decoded: NetworkStatusSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert!(decoded.server_addr.is_none());
        assert!(decoded.local_client_id.is_none());
    }

    // --- Malformed packet/data handling ---

    #[test]
    fn malformed_empty_string_rejected() {
        let result = serde_json::from_str::<NetworkStatusSnapshot>("");
        assert!(result.is_err());
    }

    #[test]
    fn malformed_garbage_bytes_rejected() {
        let result = serde_json::from_str::<NetworkStatusSnapshot>("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn malformed_truncated_json_rejected() {
        let result = serde_json::from_str::<NetworkStatusSnapshot>(r#"{"connected": true"#);
        assert!(result.is_err());
    }

    #[test]
    fn malformed_wrong_type_rejected() {
        // connected should be bool, not string
        let result = serde_json::from_str::<NetworkStatusSnapshot>(
            r#"{"server_addr":null,"game_state":"X","connected":"yes","connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn malformed_missing_required_field_rejected() {
        // Missing "game_state" field
        let result = serde_json::from_str::<NetworkStatusSnapshot>(
            r#"{"server_addr":null,"connected":false,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn malformed_extra_fields_accepted() {
        // serde_json ignores unknown fields by default
        let json = r#"{"server_addr":null,"game_state":"X","connected":false,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0,"extra_field":42}"#;
        let result = serde_json::from_str::<NetworkStatusSnapshot>(json);
        assert!(result.is_ok());
    }

    #[test]
    fn malformed_null_for_non_option_rejected() {
        // "connected" is bool, not Option — null should fail
        let result = serde_json::from_str::<NetworkStatusSnapshot>(
            r#"{"server_addr":null,"game_state":"X","connected":null,"connected_links":0,"local_client_id":null,"zone_id":0,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn malformed_terrain_snapshot_wrong_tuple() {
        // initial_tile should be [u32, u32], not a string
        let result = serde_json::from_str::<TerrainStatusSnapshot>(
            r#"{"map_name":"","initial_tile":"wrong","load_radius":0,"loaded_tiles":0,"pending_tiles":0,"failed_tiles":0,"server_requested_tiles":0,"heightmap_tiles":0,"process_rss_kb":0,"process_anon_kb":0,"process_data_kb":0,"m2_model_cache_entries":0,"m2_model_cache_est_cpu_bytes":0,"composited_texture_cache_entries":0,"composited_texture_cache_est_cpu_bytes":0,"image_assets":0,"image_asset_cpu_bytes":0,"mesh_assets":0,"mesh_asset_est_cpu_bytes":0,"standard_material_assets":0,"terrain_material_assets":0,"water_material_assets":0,"m2_effect_material_assets":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn malformed_negative_unsigned_rejected() {
        // zone_id is u32, negative should fail
        let result = serde_json::from_str::<NetworkStatusSnapshot>(
            r#"{"server_addr":null,"game_state":"X","connected":false,"connected_links":0,"local_client_id":null,"zone_id":-1,"remote_entities":0,"local_players":0,"chat_messages":0}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn malformed_chat_message_wrong_channel_type() {
        let result = serde_json::from_str::<shared::protocol::ChatMessage>(
            r#"{"sender":"A","content":"hi","channel":"InvalidChannel"}"#,
        );
        assert!(result.is_err());
    }
}
