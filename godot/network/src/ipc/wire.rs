//! Existing CLI wire definitions shared by legacy and native hosts.

use serde::{Deserialize, Serialize};
use shared::protocol::{
    AuctionSearchQuery, BuyoutAuction, CalendarSignupStatusSnapshot, CancelAuction, CreateAuction,
    EmoteKind, PlaceBid, PvpBracketSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemInfoQuery {
    pub item_id: u32,
    pub definition_source: shared::item_data::ItemDefinitionSource,
}

#[cfg(test)]
mod item_info_query_tests {
    use super::ItemInfoQuery;
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};

    #[test]
    fn item_info_query_roundtrips_each_explicit_source() {
        for source in [Retail, Forever70205] {
            let query = ItemInfoQuery {
                item_id: 2947,
                definition_source: source,
            };
            let encoded = serde_json::to_string(&query).unwrap();
            let decoded: ItemInfoQuery = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, query);
        }
    }

    #[test]
    fn item_info_query_rejects_ambiguous_missing_source() {
        let error = serde_json::from_str::<ItemInfoQuery>(r#"{"item_id":2947}"#).unwrap_err();
        assert!(error.to_string().contains("definition_source"), "{error}");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupRole {
    Tank,
    Healer,
    Damage,
    None,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum BarberOption {
    HairStyle,
    HairColor,
    FacialHair,
    SkinColor,
    Face,
}

/// IPC request from CLI to engine.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Request {
    Ping,
    Screenshot,
    Performance,
    DumpTree {
        filter: Option<String>,
    },
    DumpUiTree {
        filter: Option<String>,
    },
    AuctionOpen,
    AuctionBrowse {
        query: AuctionSearchQuery,
    },
    AuctionOwned,
    AuctionBids,
    AuctionInventory,
    AuctionCreate {
        create: CreateAuction,
    },
    AuctionBid {
        bid: PlaceBid,
    },
    AuctionBuyout {
        buyout: BuyoutAuction,
    },
    AuctionCancel {
        cancel: CancelAuction,
    },
    AuctionStatus,
    TradeInitiate {
        name: String,
    },
    TradeAccept,
    TradeDecline,
    TradeCancel,
    TradeSetItem {
        slot: u8,
        item_guid: u64,
        stack_count: u16,
    },
    TradeClearItem {
        slot: u8,
    },
    TradeSetMoney {
        copper: u64,
    },
    TradeConfirm,
    TradeCancelAccept,
    TradeStatus,
    InspectQuery,
    InspectStatus,
    DuelChallenge,
    DuelAccept,
    DuelDecline,
    DuelStatus,
    CalendarStatus,
    CalendarQuery,
    CalendarSchedule {
        title: String,
        starts_in_minutes: u32,
        max_signups: u8,
        is_raid: bool,
    },
    CalendarSignup {
        event_id: u64,
        status: CalendarSignupStatusSnapshot,
    },
    GuildStatus,
    GuildQuery,
    GuildSetMotd {
        text: String,
    },
    GuildSetInfo {
        text: String,
    },
    GuildSetOfficerNote {
        name: String,
        note: String,
    },
    FriendsStatus,
    WhoStatus,
    WhoQuery {
        query: String,
    },
    PresenceStatus,
    PresenceAfk,
    PresenceDnd,
    PresenceOnline,
    FriendAdd {
        name: String,
    },
    FriendRemove {
        name: String,
    },
    IgnoreStatus,
    IgnoreAdd {
        name: String,
    },
    IgnoreRemove {
        name: String,
    },
    BarberStatus,
    BarberSet {
        option: BarberOption,
        value: u8,
    },
    BarberReset,
    BarberApply,
    DeathStatus,
    DeathReleaseSpirit,
    DeathResurrectAtCorpse,
    DeathAcceptSpiritHealer,
    DeathStuckEscape,
    PvpStatus,
    EncounterJournalStatus,
    PvpQueueBattleground {
        battleground_id: u32,
    },
    PvpQueueRated {
        bracket: PvpBracketSnapshot,
    },
    PvpDequeue,
    LfgStatus,
    LfgQueue {
        role: GroupRole,
        dungeon_ids: Vec<u32>,
    },
    LfgDequeue,
    LfgAccept,
    LfgDecline,
    AchievementsStatus,
    NetworkStatus,
    TerrainStatus,
    SoundStatus,
    CurrenciesStatus,
    CurrencyEarn {
        currency_id: u32,
        amount: u32,
    },
    CurrencySpend {
        currency_id: u32,
        amount: u32,
    },
    ReputationsStatus,
    CharacterStatsStatus,
    BagsStatus,
    GuildVaultStatus,
    WarbankStatus,
    EquippedGearStatus,
    ItemInfo {
        query: ItemInfoQuery,
    },
    /// The open mailbox, its inbox and the pending-mail senders.
    MailStatus,
    /// `SendMail` at the open mailbox.
    MailSend {
        recipient: String,
        subject: String,
        body: String,
        attachments: Vec<u64>,
        money: u64,
        cod: u64,
    },
    /// One inbox action at the open mailbox.
    MailAct {
        mail_id: u64,
        action: shared::protocol::MailAction,
    },
    InventoryList,
    InventorySearch {
        text: String,
    },
    InventoryWhereis {
        item_id: u32,
    },
    QuestList,
    QuestWatch,
    /// Right-click the nearest NPC or game object of this name, else target the player.
    QuestInteract {
        npc: String,
    },
    /// Take every slot of the open loot window (`LootSlot` for each).
    LootTakeAll,
    QuestShow {
        quest_id: u32,
    },
    GroupRoster,
    GroupStatus,
    GroupInvite {
        name: String,
    },
    GroupUninvite {
        name: String,
    },
    Emote {
        emote: EmoteKind,
    },
    SpellCast {
        spell: String,
        target: Option<String>,
    },
    SpellStop,
    CombatLog {
        lines: u16,
    },
    CombatRecap {
        target: Option<String>,
    },
    ReputationList,
    CollectionMounts {
        missing: bool,
    },
    CollectionPets {
        missing: bool,
    },
    CollectionSummonMount {
        mount_id: u32,
    },
    CollectionDismissMount,
    CollectionSummonPet {
        pet_id: u32,
    },
    CollectionDismissPet,
    ProfessionRecipes {
        text: String,
    },
    ProfessionStatus,
    ProfessionCraft {
        recipe_id: u32,
        casts: u16,
    },
    EquipmentSet {
        slot: String,
        model_path: String,
    },
    EquipmentClear {
        slot: String,
    },
    ExportCharacter {
        output_path: String,
        character_name: Option<String>,
        character_id: Option<u64>,
    },
    ExportScene {
        output_path: String,
    },
    MapPosition,
    MapTarget,
    MapWaypointAdd {
        x: f32,
        y: f32,
    },
    MapWaypointClear,
    ScriptedMovementForward {
        duration_secs: f32,
        heading_degrees: Option<f32>,
    },
    ScriptedMovementStop,
    SetCameraDirection {
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    },
    /// Put the cursor at a window position (logical pixels, top left origin).
    HoverAt {
        x: f32,
        y: f32,
    },
    /// Put the cursor on the nearest on-screen NPC with this name.
    HoverNpc {
        name: String,
    },
    DumpScene {
        filter: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PerformanceSnapshot {
    pub fps: Option<f64>,
    pub frame_time_ms: Option<f64>,
    pub focused: bool,
}

/// IPC response from engine to CLI.
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong,
    Screenshot(Vec<u8>), // WebP bytes
    Performance(PerformanceSnapshot),
    Tree(String),
    Text(String),
    Error(String),
}
