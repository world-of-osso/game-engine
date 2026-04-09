use serde::{Deserialize, Serialize};
use shared::components::{CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot};
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
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
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
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TalentSpecTabEntry {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TalentNodeEntry {
    pub talent_id: u32,
    pub name: String,
    pub points_spent: u8,
    pub max_points: u8,
    pub active: bool,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TalentStatusSnapshot {
    pub spec_tabs: Vec<TalentSpecTabEntry>,
    pub talents: Vec<TalentNodeEntry>,
    pub points_remaining: u16,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InspectStatusSnapshot {
    pub target_name: Option<String>,
    pub equipment_appearance: EquipmentAppearance,
    pub spec_tabs: Vec<TalentSpecTabEntry>,
    pub talents: Vec<TalentNodeEntry>,
    pub points_remaining: u16,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuelPhaseEntry {
    PendingOutgoing,
    PendingIncoming,
    Active,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuelBoundaryEntry {
    pub center_x: f32,
    pub center_z: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuelResultEntry {
    Won,
    Lost,
    Declined,
    Cancelled,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DuelStatusSnapshot {
    pub phase: Option<DuelPhaseEntry>,
    pub opponent_name: Option<String>,
    pub boundary: Option<DuelBoundaryEntry>,
    pub last_result: Option<DuelResultEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RestAreaKindEntry {
    City,
    Inn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PresenceStateEntry {
    Online,
    Afk,
    Dnd,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecondaryResourceKindEntry {
    ComboPoints,
    HolyPower,
    Chi,
    Essence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecondaryResourceEntry {
    pub kind: SecondaryResourceKindEntry,
    pub current: u8,
    pub max: u8,
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
    pub secondary_resource: Option<SecondaryResourceEntry>,
    pub movement_speed: Option<f32>,
    pub gold: u32,
    pub presence: Option<PresenceStateEntry>,
    pub in_combat: bool,
    pub in_rest_area: bool,
    pub rest_area_kind: Option<RestAreaKindEntry>,
    pub rested_xp: u32,
    pub rested_xp_max: u32,
    pub zone_id: u32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CharacterRosterStatusSnapshot {
    pub entries: Vec<CharacterListEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FriendEntry {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub area: String,
    pub online: bool,
    pub presence: PresenceStateEntry,
    pub note: String,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FriendsStatusSnapshot {
    pub entries: Vec<FriendEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildMemberEntry {
    pub character_name: String,
    pub level: u16,
    pub class_name: String,
    pub rank_name: String,
    pub online: bool,
    pub officer_note: String,
    pub last_online: String,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GuildStatusSnapshot {
    pub guild_id: Option<u32>,
    pub guild_name: String,
    pub motd: String,
    pub info_text: String,
    pub entries: Vec<GuildMemberEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhoEntry {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub area: String,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WhoStatusSnapshot {
    pub query: String,
    pub entries: Vec<WhoEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CalendarSignupStateEntry {
    Confirmed,
    Tentative,
    Declined,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarSignupEntry {
    pub character_name: String,
    pub status: CalendarSignupStateEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarEventEntry {
    pub event_id: u64,
    pub title: String,
    pub organizer_name: String,
    pub starts_at_unix_secs: u64,
    pub max_signups: u8,
    pub is_raid: bool,
    pub signups: Vec<CalendarSignupEntry>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CalendarStatusSnapshot {
    pub events: Vec<CalendarEventEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IgnoreListStatusSnapshot {
    pub names: Vec<String>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LfgRoleCheckEntry {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRole,
    pub accepted_count: u8,
    pub total_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LfgMatchMemberEntry {
    pub name: String,
    pub role: GroupRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LfgMatchFoundEntry {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRole,
    pub members: Vec<LfgMatchMemberEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PvpBracketEntry {
    pub bracket: String,
    pub rating: u32,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PvpStatusSnapshot {
    pub honor: u32,
    pub honor_max: u32,
    pub conquest: u32,
    pub conquest_max: u32,
    pub queue: Option<String>,
    pub brackets: Vec<PvpBracketEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncounterJournalBossEntry {
    pub entry: u32,
    pub name: String,
    pub min_level: u16,
    pub max_level: u16,
    pub rank: u8,
    pub ability_count: usize,
    pub loot_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncounterJournalInstanceEntry {
    pub instance_id: u32,
    pub name: String,
    pub instance_type: String,
    pub tier: String,
    pub source: String,
    pub bosses: Vec<EncounterJournalBossEntry>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EncounterJournalStatusSnapshot {
    pub instances: Vec<EncounterJournalInstanceEntry>,
    pub last_error: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LfgStatusSnapshot {
    pub queued: bool,
    pub selected_role: Option<GroupRole>,
    pub dungeon_ids: Vec<u32>,
    pub queue_size: u16,
    pub average_wait_secs: u32,
    pub in_demand_roles: Vec<GroupRole>,
    pub role_check: Option<LfgRoleCheckEntry>,
    pub match_found: Option<LfgMatchFoundEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BarberShopStatusSnapshot {
    pub current_appearance: CharacterAppearance,
    pub pending_appearance: CharacterAppearance,
    pub gold: u32,
    pub pending_cost: u32,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
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
    pub durability_current: Option<u32>,
    pub durability_max: Option<u32>,
    pub repair_cost: u32,
    pub broken: bool,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EquippedGearStatusSnapshot {
    pub entries: Vec<EquippedGearEntry>,
    pub total_repair_cost: u32,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurabilityEntry {
    pub slot: EquipmentVisualSlot,
    pub current: u32,
    pub max: u32,
    pub repair_cost: u32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DurabilityStatusSnapshot {
    pub entries: Vec<DurabilityEntry>,
    pub total_repair_cost: u32,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
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
pub struct AchievementProgressEntry {
    pub achievement_id: u32,
    pub current: u32,
    pub required: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AchievementCompletionEntry {
    pub achievement_id: u32,
    pub name: String,
    pub points: u32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AchievementsStatusSnapshot {
    pub earned_ids: Vec<u32>,
    pub progress: Vec<AchievementProgressEntry>,
    pub last_completed: Option<AchievementCompletionEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionMountEntry {
    pub mount_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionPetEntry {
    pub pet_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CollectionStatusSnapshot {
    pub mounts: Vec<CollectionMountEntry>,
    pub pets: Vec<CollectionPetEntry>,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionSkillEntry {
    pub profession: String,
    pub current: u16,
    pub max: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionRecipeEntry {
    pub spell_id: u32,
    pub profession: String,
    pub name: String,
    pub craftable: bool,
    pub cooldown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionSkillUpEntry {
    pub profession: String,
    pub current: u16,
    pub max: u16,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProfessionStatusSnapshot {
    pub skills: Vec<ProfessionSkillEntry>,
    pub recipes: Vec<ProfessionRecipeEntry>,
    pub last_server_message: Option<String>,
    pub last_skill_up: Option<ProfessionSkillUpEntry>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Waypoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeathStateEntry {
    Alive,
    Dead,
    Ghost,
    Resurrecting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeathPositionEntry {
    pub map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DeathStatusSnapshot {
    pub state: Option<DeathStateEntry>,
    pub corpse: Option<DeathPositionEntry>,
    pub graveyard: Option<DeathPositionEntry>,
    pub can_resurrect_at_corpse: bool,
    pub spirit_healer_available: bool,
    pub last_server_message: Option<String>,
    pub last_error: Option<String>,
}

#[derive(bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapStatusSnapshot {
    pub zone_id: u32,
    pub player_x: f32,
    pub player_z: f32,
    pub waypoint: Option<Waypoint>,
    pub graveyard_marker: Option<Waypoint>,
}

impl Default for MapStatusSnapshot {
    fn default() -> Self {
        Self {
            zone_id: 0,
            player_x: 0.0,
            player_z: 0.0,
            waypoint: None,
            graveyard_marker: None,
        }
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod status_tests;
