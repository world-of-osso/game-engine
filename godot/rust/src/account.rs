use std::{
    fs,
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::frame_error::SessionError;
use crate::mirror_timers::MirrorTimerMessage;
use game_engine_network::{
    Event, HANDSHAKE_TIMEOUT_REASON, NetworkBridge, ProtocolMessage,
    replica::{ReplicationBatch, Schema},
};
use game_engine_session::{
    AuthRequest, ReconnectPhase, Session, SessionEffect, SessionOptions, SessionScreen,
    normalize_auth_token, token_path,
};
use game_engine_ui_model::bank_data::{BankRequest, GuildBankRequest};
use game_engine_ui_model::trade::TradeRequest;
use shared::components::StandState;
use shared::protocol::{
    AcceptTrade, CancelTrade, CancelTradeAccept, ClearTradeItem, ConfirmTrade, DeclineTrade,
    InitiateTrade, SetTradeMoney, TradeChannel, TradeStateUpdate,
};
use shared::protocol::{
    ActionBarSnapshot, AttackStart, AttackStop, AttackStopped, AttackSwing, AuthChannel,
    CastFailed, CharacterListUpdate, ChatChannel, ChatMessage, CombatChannel, CombatEvent,
    CombatLogEvent, CreateCharacter, CreateCharacterResponse, DamageMeterSnapshot, DeleteCharacter,
    DeleteCharacterResponse, DungeonDifficultySet, EmoteEvent, EmoteIntent, EnterWorldResponse,
    ForcedDisconnect, InputChannel, InstanceChannel, InstanceInfo, InstanceLockInfo,
    KnownSpellsSnapshot, LoadTerrain, LoginResponse, MirrorTimerPause, MirrorTimerStart,
    MirrorTimerStop, NewWorld, PlayerInput, QuestChannel, QuestFailed, QuestGiverStatusMultiple,
    QuestGiverStatusQuery, QuestLogSnapshot, QuestLogUpdate, RegisterResponse, RequestRaidInfo,
    RestStateUpdate, SetDungeonDifficulty, SetSpecialization, SetTarget, SpecializationChanged,
    SpellCastIntent, SpellCooldownUpdate, SpellFailure, SpellGo, SpellsLearned, SpellsUnlearned,
    StandStateIntent, StopSpellCast, TalentChannel, TransferAborted, TransferChannel, WitnessRay,
    WorldPortAck,
};
use shared::protocol::{
    AppearanceCollectionUpdate, CreatureTooltip, CreatureTooltipQuery, TooltipChannel,
};
use shared::protocol::{
    BankAutoDeposit, BankChannel, BankContents, BankDeposit, BankFailed, BankMoneyTransfer,
    BankPurchaseTab, BankUpdateTabSettings, BankWithdraw, GuildBankBuyTab, GuildBankChannel,
    GuildBankContents, GuildBankDeposit, GuildBankFailed, GuildBankLog, GuildBankMoneyTransfer,
    GuildBankQueryLog, GuildBankSetTabInfo, GuildBankSetTabText, GuildBankWithdraw,
};
use shared::protocol::{
    BuyItem, BuybackItemRequest, BuybackList, CloseInteraction, DurabilityStateUpdate,
    EquipmentSnapshot, InteractNpc, InteractionChannel, InteractionClosed, InteractionFailed,
    InteractionOpened, InventoryDelta, InventoryError, InventorySnapshot, MerchantChannel,
    MerchantFailed, RepairItem, SellAllJunkItems, SellItem, VendorInventory,
};
use shared::protocol::{
    MailChannel, MailFailed, MailRequest, MailSent, MailboxContents, PendingMail, SendMail,
    UseGameObject,
};

use game_engine_ui_model::group_state::{GroupCommand, GroupState};
use game_engine_ui_model::merchant_data::MerchantRequest;
use game_engine_ui_model::quest_runtime::{NpcInteractionRequest, QuestRuntime};
use shared::protocol::{
    AbandonQuest, QuestGiverAcceptQuest, QuestGiverChooseReward, QuestGiverCompleteQuest,
    QuestGiverHello, QuestGiverOfferReward, QuestGiverQueryQuest, QuestGiverQuestComplete,
    QuestGiverQuestDetails, QuestGiverQuestList, QuestGiverRequestItems, SetQuestWatched,
};
use shared::protocol::{
    ConvertGroupToParty, ConvertGroupToRaid, GroupChannel, GroupCommandResponse,
    GroupInviteCancelled, GroupInviteIntent, GroupInvitePrompt, GroupMemberStates,
    GroupRosterSnapshot, GroupUninviteIntent, LeaveGroup, PromoteGroupLeader, RaidTargetIcons,
    ReadyCheckUpdate, RespondGroupInvite, RespondReadyCheck, SetGroupRole, SetRaidTarget,
    StartReadyCheck,
};

use shared::protocol::{
    CorpseLootable, LootChannel, LootClosed, LootFailed, LootRelease, LootResponse,
    LootSlotRemoved, LootSlotRequest, LootUnit,
};

use crate::player_spells::PlayerSpells;
use game_engine_ui_model::auction::{AuctionReply, AuctionRequest};
use shared::protocol::{
    AuctionBrowseResults, AuctionChannel, AuctionHouseOpened, AuctionInventorySnapshot,
    AuctionOperationResponse, AuctionSearchResults, BidAuctionListResponse, OpenAuctionHouse,
    OwnedAuctionListResponse, QueryAuctionBrowse, QueryAuctionInventory, QueryAuctions,
    QueryBidAuctions, QueryOwnedAuctions, SelectGossipOption,
};

/// Combat log lines kept for automation and the cast result readout.
const COMBAT_LOG_KEEP: usize = 64;

#[derive(Default)]
pub(crate) struct StartupLoginOptions {
    pub preselected_name: Option<String>,
    pub auto_enter_world: bool,
    pub startup_screen: Option<SessionScreen>,
}

/// Godot host's account state. Only NetworkBridge owns the transport ECS world.
pub struct Account {
    pub session: Session,
    pub reply_received: bool,
    pub(crate) startup_options: StartupLoginOptions,
    bridge: Option<NetworkBridge>,
    /// The running bridge's endpoint and Netcode client, for `status network`.
    pub link: Option<NetworkLink>,
    data_root: PathBuf,
    hostname: String,
    /// Server quest log, watch list, quest giver markers and the open quest dialog.
    pub quests: QuestRuntime,
    /// `GetDungeonDifficultyID`, from `DungeonDifficultySet` (login and every change).
    pub dungeon_difficulty: Option<u32>,
    /// Saved instances of the last `InstanceInfo`.
    pub instance_locks: Vec<InstanceLockInfo>,
    /// Known spells, action bar and cooldowns from the server.
    pub spells: PlayerSpells,
    /// Newest `CombatLogEvent`s, oldest first.
    pub combat_log: std::collections::VecDeque<CombatLogEvent>,
    /// Count of `CombatLogEvent`s received this connection.
    pub combat_log_seq: u64,
    /// The server's newest damage meter sessions.
    pub damage_meter: Option<DamageMeterSnapshot>,
    /// The newest `PlayerXpUpdate`: XP into the level and the level's requirement.
    pub xp: Option<shared::protocol::PlayerXpUpdate>,
    /// Party/raid roster, live member states, the ready check and the pending invite.
    pub group: GroupState,
    /// The group's raid target icons, or the solo player's own.
    pub raid_targets: RaidTargetIcons,
}

pub struct NetworkLink {
    pub server: std::net::SocketAddr,
    pub client_id: u64,
    /// Whether the server's protocol fingerprint matched (`Event::Connected`).
    pub connected: bool,
}

pub enum AccountEvent {
    Screen(SessionScreen),
    /// The session feedback changed without a screen change.
    Feedback,
    WorldReset,
    RestState(RestStateUpdate),
    /// The realm's game time (`SMSG_LOGIN_SET_TIME_SPEED`).
    GameTime(shared::protocol::LoginSetTimeSpeed),
    LoadTerrain(LoadTerrain),
    NewWorld(NewWorld),
    TransferError(String),
    /// A connection started replicating; its entities replace the previous connection's.
    ReplicationStarted(std::sync::Arc<Schema>),
    Replication(ReplicationBatch),
    /// The connection ended: every replicated entity is gone.
    ReplicationEnded,
    /// The character roster changed through a server update or response.
    RosterChanged,
    /// A server breath, fatigue or feign-death bar change.
    MirrorTimer(crate::mirror_timers::MirrorTimerMessage),
    /// Server answer to the pending character-creation request.
    CharacterCreated {
        success: bool,
        error: Option<String>,
    },
    /// The server rejected a cast request.
    CastFailed(CastFailed),
    /// A melee swing outcome or resolved cast of a replicated unit.
    Combat(CombatMessage),
    /// NPC interaction, vendor, bag and durability traffic.
    Npc(NpcMessage),
    Auction(AuctionReply),
    Mail(MailMessage),
    /// `TradeStateUpdate`: the trade snapshot, its refusal and message.
    Trade(TradeStateUpdate),
    /// Bank and guild bank contents, logs and refusals.
    Bank(BankMessage),
    Loot(LootMessage),
    /// A chat line: players, creatures, the MOTD and server errors (`ChatChannel`).
    Chat(ChatMessage),
    /// A player's social emote, played on its model.
    Emote(EmoteEvent),
    /// A group result or notice (`ERR_*`, `READY_CHECK_*`), shown as a system chat line.
    GroupNotice(String),
    /// Quest giver dialog traffic and quest results.
    Quest(QuestMessage),
    /// A quest system line (`ERR_QUEST_ACCEPTED_S`, ...), shown as a system chat line.
    QuestNotice(String),
    /// The server's tooltip data for one creature entry.
    CreatureTooltip(CreatureTooltip),
    /// The account's learned appearances.
    Appearances(AppearanceCollectionUpdate),
}

/// Quest giver dialog pages, turn-in results and rejections (`QuestChannel`).
pub(crate) enum QuestMessage {
    List(QuestGiverQuestList),
    Details(QuestGiverQuestDetails),
    Progress(QuestGiverRequestItems),
    Reward(QuestGiverOfferReward),
    Complete(QuestGiverQuestComplete),
    Failed(QuestFailed),
}

pub(crate) enum BankMessage {
    Contents(BankContents),
    Failed(BankFailed),
    GuildContents(GuildBankContents),
    GuildLog(GuildBankLog),
    GuildFailed(GuildBankFailed),
}

pub(crate) enum MailMessage {
    Contents(MailboxContents),
    Failed(MailFailed),
    Sent(MailSent),
    Pending(PendingMail),
}

/// Authoritative loot traffic, retained in `LootChannel` send order.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LootMessage {
    Lootable(CorpseLootable),
    Opened(LootResponse),
    Removed(LootSlotRemoved),
    Closed(LootClosed),
    Failed(LootFailed),
}

/// Combat traffic that animates units and spawns spell visuals.
pub enum CombatMessage {
    /// `CombatEvent`: melee swing results (attacker swing, victim reaction), deaths.
    Event(CombatEvent),
    /// `SpellGo`: a cast resolved.
    SpellGo(SpellGo),
    /// `SpellFailure` (`SMSG_SPELL_FAILURE`): a cast or channel was interrupted or failed.
    SpellFailure(SpellFailure),
    /// `AttackStart` (`SMSG_ATTACK_START`): a unit started auto-attacking.
    AttackStart(AttackStart),
    /// `AttackStopped` (`SMSG_ATTACK_STOP`): a unit stopped auto-attacking.
    AttackStopped(AttackStopped),
}

/// Server messages for the NPC interaction and merchant host.
pub enum NpcMessage {
    Opened(InteractionOpened),
    /// `InteractionClosed`: the server ended the interaction with this NPC.
    Closed(u64),
    Vendor(VendorInventory),
    Buyback(BuybackList),
    Inventory(InventorySnapshot),
    Equipment(EquipmentSnapshot),
    InventoryChanged(InventoryDelta),
    /// `DurabilityStateUpdate.total_repair_cost` (`GetRepairAllCost`).
    RepairCost(u32),
    InteractionError(InteractionFailed),
    /// Retail `UIErrorsFrame` text of a refused vendor or bag request.
    Error(String),
}

impl Account {
    pub(crate) fn login_reply_pending(&self) -> bool {
        self.session.screen == SessionScreen::Login && self.bridge.is_some() && !self.reply_received
    }

    pub fn new(data_root: PathBuf) -> Self {
        Self {
            session: Session::default(),
            reply_received: false,
            startup_options: StartupLoginOptions::default(),
            bridge: None,
            link: None,
            data_root,
            hostname: String::new(),
            quests: QuestRuntime::default(),
            dungeon_difficulty: None,
            instance_locks: Vec::new(),
            spells: PlayerSpells::default(),
            combat_log: std::collections::VecDeque::new(),
            combat_log_seq: 0,
            damage_meter: None,
            xp: None,
            group: GroupState::default(),
            raid_targets: RaidTargetIcons::default(),
        }
    }

    pub fn connect(
        &mut self,
        hostname: &str,
        username: &str,
        password: &str,
        register: bool,
    ) -> Result<(), SessionError> {
        self.prepare_connection(hostname).map_err(SessionError)?;
        let reconnect = !register && username.trim().is_empty() && password.trim().is_empty();
        if reconnect && self.session.token.is_none() {
            return Err(SessionError("No saved session to reconnect".into()));
        }
        self.start_transport(username, password, register)
            .map_err(SessionError)
    }

    pub fn connect_startup(
        &mut self,
        hostname: &str,
        username: &str,
        password: &str,
    ) -> Result<(), SessionError> {
        self.prepare_connection(hostname).map_err(SessionError)?;
        if self.session.token.is_some() {
            self.start_transport("", "", false)
        } else {
            self.start_transport(username, password, false)
        }
        .map_err(SessionError)
    }

    fn prepare_connection(&mut self, hostname: &str) -> Result<(), String> {
        self.stop_bridge()?;
        self.reply_received = false;
        self.hostname = hostname.to_owned();
        self.dungeon_difficulty = None;
        self.instance_locks.clear();
        self.spells.clear();
        self.combat_log.clear();
        self.damage_meter = None;
        self.xp = None;
        self.group = GroupState::default();
        self.raid_targets = RaidTargetIcons::default();
        self.quests = QuestRuntime::default();
        self.session.token = self.read_token()?;
        Ok(())
    }

    fn start_transport(
        &mut self,
        username: &str,
        password: &str,
        register: bool,
    ) -> Result<(), String> {
        let address = self
            .hostname
            .to_socket_addrs()
            .map_err(|error| format!("Resolve realm {}: {error}", self.hostname))?
            .find(|address| address.is_ipv4())
            .ok_or_else(|| format!("Realm {} has no IPv4 address", self.hostname))?;
        self.reply_received = false;
        let client_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Connection clock: {error}"))?
            .as_nanos() as u64;
        let bridge = NetworkBridge::connect(address, client_id)?;
        match self.session.auth_request(username, password, register) {
            AuthRequest::Login(request) => bridge.send::<_, AuthChannel>(request)?,
            AuthRequest::Register(request) => bridge.send::<_, AuthChannel>(request)?,
        }
        self.bridge = Some(bridge);
        self.link = Some(NetworkLink {
            server: address,
            client_id,
            connected: false,
        });
        Ok(())
    }

    pub fn send_enter_world(&self) -> Result<(), SessionError> {
        let Some(request) = self.session.select_character() else {
            return Ok(());
        };
        self.bridge()?
            .send::<_, AuthChannel>(request)
            .map_err(SessionError)
    }

    pub fn send_create_character(&self, request: CreateCharacter) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, AuthChannel>(request)
            .map_err(SessionError)
    }

    pub fn send_delete_character(&self, character_id: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, AuthChannel>(DeleteCharacter { character_id })
            .map_err(SessionError)
    }

    pub fn send_player_input(&self, input: PlayerInput) -> Result<(), SessionError> {
        if self.session.screen != SessionScreen::InWorld || !self.session.gameplay_input_allowed() {
            return Ok(());
        }
        self.bridge()?
            .send::<_, InputChannel>(input)
            .map_err(SessionError)
    }

    /// Bevy `send_target_to_server`: the selected unit's server entity bits, or None.
    pub fn send_set_target(&self, target: Option<u64>) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(SetTarget {
                target_entity: target,
            })
            .map_err(SessionError)
    }

    /// `CMSG_ATTACK_SWING`: start auto-attacking `target` (server entity bits).
    pub fn send_attack_swing(&self, target: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(AttackSwing { target })
            .map_err(SessionError)
    }

    /// `CMSG_ATTACK_STOP`.
    pub fn send_attack_stop(&self) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(AttackStop)
            .map_err(SessionError)
    }

    /// `CastSpellByID`: the server validates the cast against `target` (server entity
    /// bits; `None` lets it use the replicated target) and its line of sight `witness`,
    /// and answers with `CastState`, cooldowns and combat log, or `CastFailed`.
    pub fn send_cast(
        &self,
        spell_id: u32,
        name: &str,
        target: Option<u64>,
        witness: Option<WitnessRay>,
    ) -> Result<(), SessionError> {
        if self.session.screen != SessionScreen::InWorld || !self.session.gameplay_input_allowed() {
            return Ok(());
        }
        self.bridge()?
            .send::<_, CombatChannel>(SpellCastIntent {
                spell_id: Some(spell_id),
                spell: name.to_owned(),
                target_entity: target,
                witness,
            })
            .map_err(SessionError)
    }

    /// IPC `spell cast`: the intent as given, by spell ID or name token.
    pub fn send_spell_intent(&self, intent: SpellCastIntent) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(intent)
            .map_err(SessionError)
    }

    /// `SpellStopCasting`: cancel the current cast.
    pub fn send_stop_cast(&self) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(StopSpellCast)
            .map_err(SessionError)
    }

    /// `SetDungeonDifficultyID`; the server validates it and answers `DungeonDifficultySet`.
    pub fn send_set_dungeon_difficulty(&self, difficulty_id: u32) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InstanceChannel>(SetDungeonDifficulty { difficulty_id })
            .map_err(SessionError)
    }

    /// `SetSpecialization(spec_id)`; the server answers `SpecializationChanged` and the
    /// spec's spells.
    pub fn send_set_specialization(&self, spec_id: u32) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, TalentChannel>(SetSpecialization { spec_id })
            .map_err(SessionError)
    }

    /// `CreatureTooltipQuery`; the server answers `CreatureTooltip` for the entry.
    pub fn send_creature_tooltip_query(&self, entry: u32) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, TooltipChannel>(CreatureTooltipQuery { entry })
            .map_err(SessionError)
    }

    /// `RequestRaidInfo()`; the server answers `InstanceInfo`.
    pub fn send_request_raid_info(&self) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InstanceChannel>(RequestRaidInfo)
            .map_err(SessionError)
    }

    /// Right-click on an NPC (`CMSG_GOSSIP_HELLO` and the role hellos).
    pub fn send_interact(&self, npc: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InteractionChannel>(InteractNpc { npc })
            .map_err(SessionError)
    }

    pub fn send_use_game_object(&self, object: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InteractionChannel>(UseGameObject { object })
            .map_err(SessionError)
    }

    /// A BankFrame request to the open banker `npc`.
    pub fn send_bank_request(&self, npc: u64, request: &BankRequest) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match request.clone() {
            BankRequest::Deposit {
                bank,
                tab,
                item_guid,
            } => bridge.send::<_, BankChannel>(BankDeposit {
                npc,
                bank,
                tab,
                item_guid,
            }),
            BankRequest::Withdraw { bank, tab, slot } => {
                bridge.send::<_, BankChannel>(BankWithdraw {
                    npc,
                    bank,
                    tab,
                    slot,
                })
            }
            BankRequest::PurchaseTab { bank } => {
                bridge.send::<_, BankChannel>(BankPurchaseTab { npc, bank })
            }
            BankRequest::Money {
                bank,
                copper,
                deposit,
            } => bridge.send::<_, BankChannel>(BankMoneyTransfer {
                npc,
                bank,
                copper,
                deposit,
            }),
            BankRequest::AutoDeposit {
                bank,
                include_reagents,
            } => bridge.send::<_, BankChannel>(BankAutoDeposit {
                npc,
                bank,
                include_reagents,
            }),
            BankRequest::UpdateTab {
                bank,
                tab,
                name,
                icon,
                deposit_flags,
            } => bridge.send::<_, BankChannel>(BankUpdateTabSettings {
                npc,
                bank,
                tab,
                name,
                icon,
                deposit_flags,
            }),
        }
        .map_err(SessionError)
    }

    /// A GuildBankFrame request to the open Guild Vault `object`.
    pub fn send_guild_bank_request(
        &self,
        object: u64,
        request: &GuildBankRequest,
    ) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match request.clone() {
            GuildBankRequest::Deposit { tab, item_guid } => {
                bridge.send::<_, GuildBankChannel>(GuildBankDeposit {
                    object,
                    tab,
                    item_guid,
                })
            }
            GuildBankRequest::Withdraw { tab, slot } => {
                bridge.send::<_, GuildBankChannel>(GuildBankWithdraw { object, tab, slot })
            }
            GuildBankRequest::Money { copper, deposit } => {
                bridge.send::<_, GuildBankChannel>(GuildBankMoneyTransfer {
                    object,
                    copper,
                    deposit,
                })
            }
            GuildBankRequest::BuyTab => {
                bridge.send::<_, GuildBankChannel>(GuildBankBuyTab { object })
            }
            GuildBankRequest::SetTabInfo { tab, name, icon } => {
                bridge.send::<_, GuildBankChannel>(GuildBankSetTabInfo {
                    object,
                    tab,
                    name,
                    icon,
                })
            }
            GuildBankRequest::SetTabText { tab, text } => {
                bridge.send::<_, GuildBankChannel>(GuildBankSetTabText { object, tab, text })
            }
            GuildBankRequest::QueryLog { tab } => {
                bridge.send::<_, GuildBankChannel>(GuildBankQueryLog { object, tab })
            }
        }
        .map_err(SessionError)
    }

    pub fn send_mail_request(&self, request: MailRequest) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, MailChannel>(request)
            .map_err(SessionError)
    }

    pub fn send_trade(&self, request: TradeRequest) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match request {
            TradeRequest::Initiate(target_name) => {
                bridge.send::<_, TradeChannel>(InitiateTrade { target_name })
            }
            TradeRequest::Accept => bridge.send::<_, TradeChannel>(AcceptTrade),
            TradeRequest::Decline => bridge.send::<_, TradeChannel>(DeclineTrade),
            TradeRequest::Cancel => bridge.send::<_, TradeChannel>(CancelTrade),
            TradeRequest::SetItem(item) => bridge.send::<_, TradeChannel>(item),
            TradeRequest::ClearItem(slot) => {
                bridge.send::<_, TradeChannel>(ClearTradeItem { slot })
            }
            TradeRequest::SetMoney(copper) => {
                bridge.send::<_, TradeChannel>(SetTradeMoney { copper })
            }
            TradeRequest::Confirm => bridge.send::<_, TradeChannel>(ConfirmTrade),
            TradeRequest::CancelAccept => bridge.send::<_, TradeChannel>(CancelTradeAccept),
        }
        .map_err(SessionError)
    }

    pub fn send_mail(&self, mail: SendMail) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, MailChannel>(mail)
            .map_err(SessionError)
    }

    /// Original cursor requests; only server InventoryDelta changes local contents.
    pub fn send_inventory_request(
        &self,
        request: &game_engine_ui_model::bag_data::InventoryRequest,
    ) -> Result<(), SessionError> {
        use game_engine_ui_model::bag_data::InventoryRequest;
        use shared::protocol::InventoryChannel;
        let bridge = self.bridge()?;
        match request {
            InventoryRequest::Swap(request) => bridge.send::<_, InventoryChannel>(request.clone()),
            InventoryRequest::Equip(request) => bridge.send::<_, InventoryChannel>(request.clone()),
            InventoryRequest::Split(request) => bridge.send::<_, InventoryChannel>(request.clone()),
            InventoryRequest::Destroy(request) => {
                bridge.send::<_, InventoryChannel>(request.clone())
            }
            InventoryRequest::Use(request) => bridge.send::<_, InventoryChannel>(request.clone()),
        }
        .map_err(SessionError)
    }

    pub fn send_loot_unit(&self, corpse: u64, auto: bool) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, LootChannel>(LootUnit { corpse, auto })
            .map_err(SessionError)
    }

    pub fn send_loot_slot(&self, corpse: u64, slot: u8) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, LootChannel>(LootSlotRequest { corpse, slot })
            .map_err(SessionError)
    }

    pub fn send_loot_release(&self, corpse: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, LootChannel>(LootRelease { corpse })
            .map_err(SessionError)
    }

    /// Ask for the quest markers of NPCs the client sees (`CMSG_QUEST_GIVER_STATUS_MULTIPLE_QUERY`).
    pub fn send_quest_giver_status_query(&self, npcs: Vec<u64>) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, QuestChannel>(QuestGiverStatusQuery { npcs })
            .map_err(SessionError)
    }

    /// A quest giver frame, quest log or gossip request (Bevy `networking/quests.rs`
    /// `send_request`).
    pub fn send_quest_request(&self, request: NpcInteractionRequest) -> Result<(), SessionError> {
        use NpcInteractionRequest as R;
        let bridge = self.bridge()?;
        match request {
            R::Hello { npc } => bridge.send::<_, QuestChannel>(QuestGiverHello { npc }),
            R::SelectGossip { npc, option_id } => {
                bridge.send::<_, InteractionChannel>(SelectGossipOption { npc, option_id })
            }
            R::QueryQuest { npc, quest_id } => {
                bridge.send::<_, QuestChannel>(QuestGiverQueryQuest { npc, quest_id })
            }
            R::Accept { npc, quest_id } => {
                bridge.send::<_, QuestChannel>(QuestGiverAcceptQuest { npc, quest_id })
            }
            R::Complete { npc, quest_id } => {
                bridge.send::<_, QuestChannel>(QuestGiverCompleteQuest { npc, quest_id })
            }
            R::ChooseReward {
                npc,
                quest_id,
                choice,
            } => bridge.send::<_, QuestChannel>(QuestGiverChooseReward {
                npc,
                quest_id,
                choice_index: choice,
            }),
            R::Abandon { quest_id } => bridge.send::<_, QuestChannel>(AbandonQuest { quest_id }),
            R::SetWatched { quest_id, watched } => {
                bridge.send::<_, QuestChannel>(SetQuestWatched { quest_id, watched })
            }
            R::Close { npc } => bridge.send::<_, InteractionChannel>(CloseInteraction { npc }),
        }
        .map_err(SessionError)
    }

    /// The player closed the NPC's frame (`CMSG_CLOSE_INTERACTION`).
    pub fn send_close_interaction(&self, npc: u64) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InteractionChannel>(CloseInteraction { npc })
            .map_err(SessionError)
    }

    pub fn send_gossip_option(&self, npc: u64, option_id: u32) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, InteractionChannel>(SelectGossipOption { npc, option_id })
            .map_err(SessionError)
    }
    pub fn send_auction_request(&self, request: AuctionRequest) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match request {
            AuctionRequest::Open => bridge.send::<_, AuctionChannel>(OpenAuctionHouse),
            AuctionRequest::Browse(query) => {
                bridge.send::<_, AuctionChannel>(QueryAuctionBrowse { query })
            }
            AuctionRequest::Listings(query) => {
                bridge.send::<_, AuctionChannel>(QueryAuctions { query })
            }
            AuctionRequest::Owned => bridge.send::<_, AuctionChannel>(QueryOwnedAuctions),
            AuctionRequest::Bids => bridge.send::<_, AuctionChannel>(QueryBidAuctions),
            AuctionRequest::Inventory => bridge.send::<_, AuctionChannel>(QueryAuctionInventory),
            AuctionRequest::Create(request) => bridge.send::<_, AuctionChannel>(request),
            AuctionRequest::Bid(request) => bridge.send::<_, AuctionChannel>(request),
            AuctionRequest::Buyout(request) => bridge.send::<_, AuctionChannel>(request),
            AuctionRequest::Cancel(request) => bridge.send::<_, AuctionChannel>(request),
        }
        .map_err(SessionError)
    }

    /// A MerchantFrame request to the open vendor `npc` (Bevy `send_merchant_requests`).
    pub fn send_merchant_request(
        &self,
        npc: u64,
        request: &MerchantRequest,
    ) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match *request {
            MerchantRequest::Buy {
                slot,
                item_id,
                count,
                destination,
            } => bridge.send::<_, MerchantChannel>(BuyItem {
                npc,
                slot,
                item_id,
                count,
                destination,
            }),
            MerchantRequest::SellAllJunk => {
                bridge.send::<_, MerchantChannel>(SellAllJunkItems { npc })
            }
            MerchantRequest::Sell { item_guid, count } => {
                bridge.send::<_, MerchantChannel>(SellItem {
                    npc,
                    item_guid,
                    count,
                })
            }
            MerchantRequest::Buyback { slot } => {
                bridge.send::<_, MerchantChannel>(BuybackItemRequest { npc, slot })
            }
            MerchantRequest::Repair { item_guid } => {
                bridge.send::<_, MerchantChannel>(RepairItem {
                    npc,
                    item_guid,
                    guild_bank: false,
                })
            }
            MerchantRequest::GuildRepairAll => bridge.send::<_, MerchantChannel>(RepairItem {
                npc,
                item_guid: None,
                guild_bank: true,
            }),
        }
        .map_err(SessionError)
    }

    /// A chat edit box line (Bevy `send_chat_message`); the server echoes it to its hearers.
    pub fn send_chat(&self, message: ChatMessage) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, ChatChannel>(message)
            .map_err(SessionError)
    }

    /// `SetRaidTarget(unit, icon)`: icon 1–8 on the unit's server entity bits, 0 clears.
    pub fn send_raid_target(&self, target: u64, icon: u8) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, GroupChannel>(SetRaidTarget { target, icon })
            .map_err(SessionError)
    }

    /// A group request from chat or the invite popup, on `GroupChannel` as the root
    /// client's `send_group_command` sends it.
    pub fn send_group(&self, command: GroupCommand) -> Result<(), SessionError> {
        let bridge = self.bridge()?;
        match command {
            GroupCommand::Invite(name) => {
                bridge.send::<_, GroupChannel>(GroupInviteIntent { name })
            }
            GroupCommand::Uninvite(name) => {
                bridge.send::<_, GroupChannel>(GroupUninviteIntent { name })
            }
            GroupCommand::Promote(name) => {
                bridge.send::<_, GroupChannel>(PromoteGroupLeader { name })
            }
            GroupCommand::Leave => bridge.send::<_, GroupChannel>(LeaveGroup),
            GroupCommand::ConvertToRaid => bridge.send::<_, GroupChannel>(ConvertGroupToRaid),
            GroupCommand::ConvertToParty => bridge.send::<_, GroupChannel>(ConvertGroupToParty),
            GroupCommand::SetRole { name, role } => {
                bridge.send::<_, GroupChannel>(SetGroupRole { name, role })
            }
            GroupCommand::StartReadyCheck => bridge.send::<_, GroupChannel>(StartReadyCheck),
            GroupCommand::RespondReadyCheck(ready) => {
                bridge.send::<_, GroupChannel>(RespondReadyCheck { ready })
            }
            GroupCommand::RespondInvite(accept) => {
                bridge.send::<_, GroupChannel>(RespondGroupInvite { accept })
            }
        }
        .map_err(SessionError)
    }

    /// `/dance`, `/wave`, ...: the server plays the emote and sends its chat line.
    pub fn send_emote(&self, intent: EmoteIntent) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, ChatChannel>(intent)
            .map_err(SessionError)
    }

    /// `CMSG_STAND_STATE_CHANGE`: ask the server to stand, sit, sleep or kneel.
    pub fn send_stand_state(&self, state: StandState) -> Result<(), SessionError> {
        self.bridge()?
            .send::<_, CombatChannel>(StandStateIntent { state })
            .map_err(SessionError)
    }

    /// Called by the host only after the destination is ready for world entry.
    pub fn finish_world_port(&mut self) -> Result<(), SessionError> {
        self.session.finish_world_port();
        if self.session.loaded_world_port().is_some() {
            self.bridge()?
                .send::<_, TransferChannel>(WorldPortAck)
                .map_err(SessionError)?;
            self.session.take_world_port_ack();
        }
        Ok(())
    }

    fn read_token(&self) -> Result<Option<String>, String> {
        let path = token_path(&self.data_root, Some(&self.hostname));
        match fs::read_to_string(&path) {
            Ok(token) => Ok(normalize_auth_token(&token)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("Read saved session {}: {error}", path.display())),
        }
    }

    /// Drain transport events into account events. Every failure here (transport,
    /// protocol decode, auth, token persistence) is a session failure.
    pub fn poll(&mut self) -> Result<Vec<AccountEvent>, SessionError> {
        self.poll_events().map_err(SessionError)
    }

    fn poll_events(&mut self) -> Result<Vec<AccountEvent>, String> {
        let events = match self.bridge.as_mut() {
            Some(bridge) => bridge.drain_events()?,
            None => return Ok(Vec::new()),
        };
        let mut output = Vec::new();
        for event in events {
            match event {
                Event::Connected => {
                    self.set_link_connected(true);
                    self.session.receive_connected();
                }
                Event::ProtocolRejected(reason) => self.session.receive_protocol_rejected(reason),
                Event::Disconnected(reason) => {
                    self.set_link_connected(false);
                    output.push(AccountEvent::ReplicationEnded);
                    let effects = match reason.as_deref() {
                        Some(reason @ HANDSHAKE_TIMEOUT_REASON) => {
                            self.session.receive_connect_failed(reason)
                        }
                        reason => self.session.receive_disconnected_with_reason(reason),
                    };
                    self.apply_effects(effects, &mut output)?;
                }
                Event::Message(message) => self.dispatch_message(message, &mut output)?,
                Event::ReplicationStarted(schema) => {
                    output.push(AccountEvent::ReplicationStarted(schema))
                }
                Event::Replication(batch) => output.push(AccountEvent::Replication(batch)),
            }
            if self.bridge.is_none() {
                break;
            }
        }
        // Finish the old event batch before creating a replacement worker. Reset effects stop
        // and join the old worker, and the loop above discards its remaining queued events.
        if self.bridge.is_none() && self.session.reconnect_phase == ReconnectPhase::PendingConnect {
            self.start_transport("", "", false)?;
        }
        Ok(output)
    }

    fn dispatch_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if Self::is_account_state_message(&message) {
            return self.dispatch_account_state_message(message, output);
        }
        let message = match quest_message(message)? {
            Ok(quest) => {
                output.push(AccountEvent::Quest(quest));
                return Ok(());
            }
            Err(message) => message,
        };
        if Self::is_mirror_timer_message(&message) {
            return Self::dispatch_mirror_timer_message(message, output);
        }
        if message.is::<MailboxContents>() {
            output.push(AccountEvent::Mail(MailMessage::Contents(decode(message)?)));
            return Ok(());
        }
        if message.is::<MailFailed>() {
            output.push(AccountEvent::Mail(MailMessage::Failed(decode(message)?)));
            return Ok(());
        }
        if message.is::<MailSent>() {
            output.push(AccountEvent::Mail(MailMessage::Sent(decode(message)?)));
            return Ok(());
        }
        if message.is::<PendingMail>() {
            output.push(AccountEvent::Mail(MailMessage::Pending(decode(message)?)));
            return Ok(());
        }
        if message.is::<TradeStateUpdate>() {
            output.push(AccountEvent::Trade(decode(message)?));
            return Ok(());
        }
        if is_loot_message(&message) {
            output.push(AccountEvent::Loot(receive_loot_message(message)?));
            return Ok(());
        }
        if message.is::<CombatEvent>() {
            output.push(AccountEvent::Combat(CombatMessage::Event(decode(message)?)));
            return Ok(());
        }
        if self.receive_spell_message(&message) {
            return self.dispatch_spell_message(message, output);
        }
        if message.is::<RestStateUpdate>() {
            output.push(AccountEvent::RestState(decode(message)?));
            return Ok(());
        }
        if message.is::<shared::protocol::LoginSetTimeSpeed>() {
            output.push(AccountEvent::GameTime(decode(message)?));
            return Ok(());
        }
        if message.is::<ChatMessage>() {
            output.push(AccountEvent::Chat(decode(message)?));
            return Ok(());
        }
        if message.is::<EmoteEvent>() {
            output.push(AccountEvent::Emote(decode(message)?));
            return Ok(());
        }
        if message.is::<CreatureTooltip>() {
            output.push(AccountEvent::CreatureTooltip(decode(message)?));
            return Ok(());
        }
        if message.is::<AppearanceCollectionUpdate>() {
            output.push(AccountEvent::Appearances(decode(message)?));
            return Ok(());
        }
        if Self::is_group_message(&message) {
            return self.dispatch_group_message(message, output);
        }
        self.dispatch_world_message(message, output)
    }

    fn is_account_state_message(message: &ProtocolMessage) -> bool {
        message.is::<QuestLogSnapshot>()
            || message.is::<QuestLogUpdate>()
            || message.is::<QuestGiverStatusMultiple>()
            || message.is::<DungeonDifficultySet>()
            || message.is::<InstanceInfo>()
            || message.is::<DamageMeterSnapshot>()
            || message.is::<shared::protocol::PlayerXpUpdate>()
    }

    fn dispatch_account_state_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<QuestLogSnapshot>() {
            self.quests.apply_snapshot(decode(message)?);
            return Ok(());
        }
        if message.is::<QuestLogUpdate>() {
            let notices = self.quests.apply_update(decode(message)?);
            output.extend(notices.into_iter().map(AccountEvent::QuestNotice));
            return Ok(());
        }
        if message.is::<QuestGiverStatusMultiple>() {
            let statuses: QuestGiverStatusMultiple = decode(message)?;
            self.quests.apply_statuses(
                statuses
                    .statuses
                    .into_iter()
                    .map(|entry| (entry.npc, entry.status)),
            );
            return Ok(());
        }
        if message.is::<shared::protocol::PlayerXpUpdate>() {
            self.xp = Some(decode(message)?);
            return Ok(());
        }
        if message.is::<DamageMeterSnapshot>() {
            self.damage_meter = Some(decode(message)?);
            return Ok(());
        }
        if message.is::<DungeonDifficultySet>() {
            let set: DungeonDifficultySet = decode(message)?;
            self.dungeon_difficulty = Some(set.difficulty_id);
            return Ok(());
        }
        let info: InstanceInfo = decode(message)?;
        self.instance_locks = info.locks;
        Ok(())
    }

    fn is_group_message(message: &ProtocolMessage) -> bool {
        message.is::<GroupRosterSnapshot>()
            || message.is::<GroupMemberStates>()
            || message.is::<GroupInvitePrompt>()
            || message.is::<GroupInviteCancelled>()
            || message.is::<ReadyCheckUpdate>()
            || message.is::<GroupCommandResponse>()
            || message.is::<RaidTargetIcons>()
    }

    /// Fill [`GroupState`] as the root client's `receive_group` does; results go to chat.
    fn dispatch_group_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<GroupRosterSnapshot>() {
            self.group.apply_roster(decode(message)?);
        } else if message.is::<GroupMemberStates>() {
            let states: GroupMemberStates = decode(message)?;
            self.group.apply_member_states(states.members);
        } else if message.is::<GroupInvitePrompt>() {
            let prompt: GroupInvitePrompt = decode(message)?;
            self.group.pending_invite = Some(prompt.inviter_name);
        } else if message.is::<GroupInviteCancelled>() {
            let cancelled: GroupInviteCancelled = decode(message)?;
            if self.group.pending_invite.as_deref() == Some(cancelled.inviter_name.as_str()) {
                self.group.pending_invite = None;
            }
        } else if message.is::<ReadyCheckUpdate>() {
            self.group.apply_ready_check(decode(message)?);
        } else if message.is::<RaidTargetIcons>() {
            self.raid_targets = decode(message)?;
        } else {
            let response: GroupCommandResponse = decode(message)?;
            self.group.last_server_message = Some(response.message.clone());
            output.push(AccountEvent::GroupNotice(response.message));
        }
        Ok(())
    }

    fn is_mirror_timer_message(message: &ProtocolMessage) -> bool {
        message.is::<MirrorTimerStart>()
            || message.is::<MirrorTimerPause>()
            || message.is::<MirrorTimerStop>()
    }

    fn dispatch_mirror_timer_message(
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<MirrorTimerStart>() {
            output.push(AccountEvent::MirrorTimer(MirrorTimerMessage::Start(
                decode(message)?,
            )));
            return Ok(());
        }
        if message.is::<MirrorTimerPause>() {
            output.push(AccountEvent::MirrorTimer(MirrorTimerMessage::Pause(
                decode(message)?,
            )));
            return Ok(());
        }
        output.push(AccountEvent::MirrorTimer(MirrorTimerMessage::Stop(decode(
            message,
        )?)));
        Ok(())
    }

    fn dispatch_world_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if Self::is_world_transition_message(&message) {
            return self.dispatch_world_transition_message(message, output);
        }
        if Self::is_roster_message(&message) {
            return self.dispatch_roster_message(message, output);
        }
        let message = match auction_message(message)? {
            Ok(reply) => {
                output.push(AccountEvent::Auction(reply));
                return Ok(());
            }
            Err(message) => message,
        };
        let message = match bank_message(message)? {
            Ok(bank) => {
                output.push(AccountEvent::Bank(bank));
                return Ok(());
            }
            Err(message) => message,
        };
        let message = match npc_message(message)? {
            Ok(npc) => {
                output.push(AccountEvent::Npc(npc));
                return Ok(());
            }
            Err(message) => message,
        };
        let effects = self.receive_message(message)?;
        self.apply_effects(effects, output)
    }

    fn is_world_transition_message(message: &ProtocolMessage) -> bool {
        message.is::<LoadTerrain>() || message.is::<NewWorld>() || message.is::<TransferAborted>()
    }

    fn dispatch_world_transition_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<LoadTerrain>() {
            let request = decode(message)?;
            self.session.receive_terrain_refresh();
            output.push(AccountEvent::LoadTerrain(request));
            return Ok(());
        }
        if message.is::<NewWorld>() {
            let new_world: NewWorld = decode(message)?;
            self.session.begin_world_port(new_world.map_id);
            output.push(AccountEvent::NewWorld(new_world));
            output.push(AccountEvent::Screen(SessionScreen::Loading));
            return Ok(());
        }
        let aborted: TransferAborted = decode(message)?;
        output.push(AccountEvent::TransferError(
            self.format_transfer_error(aborted)?,
        ));
        Ok(())
    }

    fn is_roster_message(message: &ProtocolMessage) -> bool {
        message.is::<CharacterListUpdate>()
            || message.is::<DeleteCharacterResponse>()
            || message.is::<CreateCharacterResponse>()
    }

    fn dispatch_roster_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<CharacterListUpdate>() {
            self.session.receive_character_update(decode(message)?);
            output.push(AccountEvent::RosterChanged);
            return Ok(());
        }
        if message.is::<DeleteCharacterResponse>() {
            self.session.receive_character_deleted(decode(message)?);
            output.push(AccountEvent::RosterChanged);
            return Ok(());
        }
        let result = self.session.receive_character_created(decode(message)?);
        output.push(AccountEvent::RosterChanged);
        output.push(AccountEvent::CharacterCreated {
            success: result.success,
            error: result.error,
        });
        Ok(())
    }

    fn receive_spell_message(&self, message: &ProtocolMessage) -> bool {
        message.is::<KnownSpellsSnapshot>()
            || message.is::<SpellsLearned>()
            || message.is::<SpellsUnlearned>()
            || message.is::<SpecializationChanged>()
            || message.is::<ActionBarSnapshot>()
            || message.is::<SpellCooldownUpdate>()
            || message.is::<CastFailed>()
            || message.is::<CombatLogEvent>()
            || message.is::<SpellGo>()
            || message.is::<SpellFailure>()
            || message.is::<AttackStart>()
            || message.is::<AttackStopped>()
    }

    /// Spell state lives on the account (like the quest log); rejections reach the host.
    fn dispatch_spell_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        let spells = &mut self.spells;
        if message.is::<KnownSpellsSnapshot>() {
            spells.set_known(decode::<KnownSpellsSnapshot>(message)?.spells);
        } else if message.is::<SpellsLearned>() {
            spells.learn(&decode::<SpellsLearned>(message)?.spells);
        } else if message.is::<SpellsUnlearned>() {
            spells.unlearn(&decode::<SpellsUnlearned>(message)?.spells);
        } else if message.is::<SpecializationChanged>() {
            spells.set_spec(decode::<SpecializationChanged>(message)?.spec_id);
        } else if message.is::<ActionBarSnapshot>() {
            spells.set_bar(&decode::<ActionBarSnapshot>(message)?.slots);
        } else if message.is::<SpellCooldownUpdate>() {
            spells.apply_cooldown(&decode(message)?);
        } else if message.is::<CastFailed>() {
            output.push(AccountEvent::CastFailed(decode(message)?));
        } else if message.is::<SpellGo>() {
            output.push(AccountEvent::Combat(CombatMessage::SpellGo(decode(
                message,
            )?)));
        } else if message.is::<SpellFailure>() {
            output.push(AccountEvent::Combat(CombatMessage::SpellFailure(decode(
                message,
            )?)));
        } else if message.is::<AttackStart>() {
            output.push(AccountEvent::Combat(CombatMessage::AttackStart(decode(
                message,
            )?)));
        } else if message.is::<AttackStopped>() {
            output.push(AccountEvent::Combat(CombatMessage::AttackStopped(decode(
                message,
            )?)));
        } else {
            if self.combat_log.len() == COMBAT_LOG_KEEP {
                self.combat_log.pop_front();
            }
            self.combat_log.push_back(decode(message)?);
            self.combat_log_seq += 1;
        }
        Ok(())
    }

    fn receive_message(&mut self, message: ProtocolMessage) -> Result<Vec<SessionEffect>, String> {
        if message.is::<LoginResponse>() {
            let response = decode(message)?;
            self.reply_received = true;
            let startup = std::mem::take(&mut self.startup_options);
            if self.session.reconnect_phase == ReconnectPhase::Inactive {
                validate_startup_name(&response, startup.preselected_name.as_deref())?;
            }
            let options = SessionOptions {
                preselected_name: startup.preselected_name.as_deref(),
                auto_enter_world: startup.auto_enter_world,
                startup_screen: startup.startup_screen,
            };
            return Ok(self.session.receive_login(response, options));
        }
        if message.is::<RegisterResponse>() {
            self.reply_received = true;
            return Ok(self.session.receive_registration(decode(message)?));
        }
        if message.is::<EnterWorldResponse>() {
            return Ok(self.session.receive_enter_world(decode(message)?));
        }
        if message.is::<ForcedDisconnect>() {
            return Ok(self.session.receive_forced_disconnect(decode(message)?));
        }
        Err("Unhandled account protocol message".into())
    }

    fn apply_effects(
        &mut self,
        effects: Vec<SessionEffect>,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        for effect in effects {
            match effect {
                SessionEffect::PersistToken(token) => {
                    let path = token_path(&self.data_root, Some(&self.hostname));
                    fs::write(&path, token)
                        .map_err(|error| format!("Save session {}: {error}", path.display()))?;
                }
                SessionEffect::SelectCharacter(request) => {
                    self.connected_bridge()?.send::<_, AuthChannel>(request)?
                }
                SessionEffect::RequestDisconnect => self.connected_bridge()?.disconnect()?,
                SessionEffect::ResetNetworkWorld => {
                    self.stop_bridge()?;
                    output.push(AccountEvent::WorldReset);
                }
                SessionEffect::Transition(screen) => output.push(AccountEvent::Screen(screen)),
                SessionEffect::ShowFeedback => output.push(AccountEvent::Feedback),
            }
        }
        Ok(())
    }

    fn format_transfer_error(&self, aborted: TransferAborted) -> Result<String, String> {
        use shared::protocol::TransferAbortReason;

        let map_name = match aborted.reason {
            TransferAbortReason::Difficulty(_) | TransferAbortReason::LockedToDifferentInstance => {
                read_transfer_map_name(&self.data_root, aborted.map_id)?
            }
            _ => String::new(),
        };
        Ok(self.session.receive_transfer_aborted(aborted, &map_name))
    }

    pub fn is_connected(&self) -> bool {
        self.bridge.is_some()
    }

    fn bridge(&self) -> Result<&NetworkBridge, SessionError> {
        self.connected_bridge().map_err(SessionError)
    }

    fn connected_bridge(&self) -> Result<&NetworkBridge, String> {
        self.bridge
            .as_ref()
            .ok_or_else(|| "No active account connection".into())
    }

    pub fn stop(&mut self) -> Result<(), SessionError> {
        self.stop_bridge().map_err(SessionError)
    }

    fn set_link_connected(&mut self, connected: bool) {
        if let Some(link) = &mut self.link {
            link.connected = connected;
        }
    }

    fn stop_bridge(&mut self) -> Result<(), String> {
        self.session.reset_world_port();
        self.link = None;
        if let Some(mut bridge) = self.bridge.take() {
            bridge.stop()?;
        }
        Ok(())
    }
}

/// The NPC interaction, vendor, bag or durability message, or the message back.
fn bank_message(message: ProtocolMessage) -> Result<Result<BankMessage, ProtocolMessage>, String> {
    let bank = if message.is::<BankContents>() {
        BankMessage::Contents(decode(message)?)
    } else if message.is::<BankFailed>() {
        BankMessage::Failed(decode(message)?)
    } else if message.is::<GuildBankContents>() {
        BankMessage::GuildContents(decode(message)?)
    } else if message.is::<GuildBankLog>() {
        BankMessage::GuildLog(decode(message)?)
    } else if message.is::<GuildBankFailed>() {
        BankMessage::GuildFailed(decode(message)?)
    } else {
        return Ok(Err(message));
    };
    Ok(Ok(bank))
}

fn npc_message(message: ProtocolMessage) -> Result<Result<NpcMessage, ProtocolMessage>, String> {
    let npc = if message.is::<InteractionOpened>() {
        NpcMessage::Opened(decode(message)?)
    } else if message.is::<InteractionClosed>() {
        NpcMessage::Closed(decode::<InteractionClosed>(message)?.npc)
    } else if message.is::<InteractionFailed>() {
        NpcMessage::InteractionError(decode(message)?)
    } else if message.is::<VendorInventory>() {
        NpcMessage::Vendor(decode(message)?)
    } else if message.is::<BuybackList>() {
        NpcMessage::Buyback(decode(message)?)
    } else if message.is::<MerchantFailed>() {
        NpcMessage::Error(decode::<MerchantFailed>(message)?.error.message().into())
    } else if message.is::<InventorySnapshot>() {
        NpcMessage::Inventory(decode(message)?)
    } else if message.is::<EquipmentSnapshot>() {
        NpcMessage::Equipment(decode(message)?)
    } else if message.is::<InventoryDelta>() {
        NpcMessage::InventoryChanged(decode(message)?)
    } else if message.is::<InventoryError>() {
        NpcMessage::Error(decode::<InventoryError>(message)?.reason.message())
    } else if message.is::<DurabilityStateUpdate>() {
        let update: DurabilityStateUpdate = decode(message)?;
        match update.snapshot {
            Some(snapshot) => NpcMessage::RepairCost(snapshot.total_repair_cost),
            None => NpcMessage::Error(
                update
                    .error
                    .unwrap_or_else(|| "Durability update without a snapshot".into()),
            ),
        }
    } else {
        return Ok(Err(message));
    };
    Ok(Ok(npc))
}

fn validate_startup_name(response: &LoginResponse, name: Option<&str>) -> Result<(), String> {
    if !response.success {
        return Ok(());
    }
    let Some(name) = name else {
        return Ok(());
    };
    if response
        .characters
        .iter()
        .any(|character| character.name.eq_ignore_ascii_case(name))
    {
        Ok(())
    } else {
        Err(format!(
            "Character '{name}' not found in authenticated roster"
        ))
    }
}

fn read_transfer_map_name(data_root: &Path, map_id: u32) -> Result<String, String> {
    find_map_field(data_root, "ID", &map_id.to_string(), "MapName_lang")
        .map_err(|error| format!("{error} (map {map_id})"))
}

/// The `Map.db2` ID of the map whose `Directory` is `directory` (`stormwindjail` is 34).
pub(crate) fn read_map_id(data_root: &Path, directory: &str) -> Result<u32, String> {
    let id = find_map_field(data_root, "Directory", directory, "ID")?;
    id.parse()
        .map_err(|error| format!("Map.csv: invalid ID {id:?} for {directory}: {error}"))
}

/// The non-empty `value_column` of the first `Map.csv` row whose `key_column` equals `key`,
/// ignoring ASCII case.
fn find_map_field(
    data_root: &Path,
    key_column: &str,
    key: &str,
    value_column: &str,
) -> Result<String, String> {
    use game_engine_core::csv_util::header_index;

    let path = data_root.join("db2/12.1.0.69933/Map.csv");
    let read_error = |error: &dyn std::fmt::Display| format!("Read {}: {error}", path.display());
    let file = fs::File::open(&path).map_err(|error| read_error(&error))?;
    let mut reader = csv::Reader::from_reader(file);
    let headers: Vec<String> = reader
        .headers()
        .map_err(|error| read_error(&error))?
        .iter()
        .map(str::to_owned)
        .collect();
    if headers.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    let key_index = header_index(&headers, key_column, &path)?;
    let value_index = header_index(&headers, value_column, &path)?;
    for record in reader.records() {
        let fields = record.map_err(|error| read_error(&error))?;
        if !fields
            .get(key_index)
            .is_some_and(|field| field.eq_ignore_ascii_case(key))
        {
            continue;
        }
        return fields
            .get(value_index)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                format!(
                    "{}: {key_column} {key} has no {value_column}",
                    path.display()
                )
            });
    }
    Err(format!("{}: no {key_column} {key}", path.display()))
}

/// The quest giver dialog message, or the message back (`QuestChannel`).
fn quest_message(
    message: ProtocolMessage,
) -> Result<Result<QuestMessage, ProtocolMessage>, String> {
    Ok(Ok(if message.is::<QuestGiverQuestList>() {
        QuestMessage::List(decode(message)?)
    } else if message.is::<QuestGiverQuestDetails>() {
        QuestMessage::Details(decode(message)?)
    } else if message.is::<QuestGiverRequestItems>() {
        QuestMessage::Progress(decode(message)?)
    } else if message.is::<QuestGiverOfferReward>() {
        QuestMessage::Reward(decode(message)?)
    } else if message.is::<QuestGiverQuestComplete>() {
        QuestMessage::Complete(decode(message)?)
    } else if message.is::<QuestFailed>() {
        QuestMessage::Failed(decode(message)?)
    } else {
        return Ok(Err(message));
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    use shared::components::{CharacterAppearance, EquipmentAppearance};
    use shared::protocol::{CharacterListEntry, TransferAbortReason};

    #[test]
    fn startup_name_must_match_authenticated_roster_without_fallback() {
        let response = LoginResponse {
            success: true,
            token: "new-token".into(),
            characters: vec![CharacterListEntry {
                character_id: 42,
                name: "Alessio".into(),
                level: 10,
                race: 1,
                class: 2,
                appearance: CharacterAppearance::default(),
                equipment_appearance: EquipmentAppearance::default(),
            }],
            error: None,
        };
        assert!(validate_startup_name(&response, Some("aLeSsIo")).is_ok());
        assert!(validate_startup_name(&response, None).is_ok());
        assert_eq!(
            validate_startup_name(&response, Some("Missing")),
            Err("Character 'Missing' not found in authenticated roster".into())
        );
    }

    #[test]
    fn failed_login_does_not_validate_startup_name() {
        let response = LoginResponse {
            success: false,
            token: String::new(),
            characters: Vec::new(),
            error: Some("Invalid credentials".into()),
        };
        assert!(validate_startup_name(&response, Some("Missing")).is_ok());
    }

    #[test]
    fn full_instance_error_does_not_require_map_catalog() {
        let account = Account::new(PathBuf::from("/nonexistent-map-catalog"));
        let error = account
            .format_transfer_error(TransferAborted {
                map_id: 34,
                reason: TransferAbortReason::MaxPlayers,
            })
            .unwrap();
        assert_eq!(error, "Transfer Aborted: instance is full");
    }

    #[test]
    fn heroic_transfer_error_uses_aborted_map_name() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let account = Account::new(data_root);
        let error = account
            .format_transfer_error(TransferAborted {
                map_id: 34,
                reason: TransferAbortReason::Difficulty(2),
            })
            .unwrap();
        assert_eq!(
            error,
            "Heroic difficulty mode is not available for Stormwind Stockade."
        );
    }

    #[test]
    fn transfer_abort_map_name_comes_from_map_csv_id() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        assert_eq!(
            read_transfer_map_name(&data_root, 34).unwrap(),
            "Stormwind Stockade"
        );
        assert_eq!(
            read_transfer_map_name(&data_root, 33).unwrap(),
            "Shadowfang Keep"
        );
        assert!(read_transfer_map_name(&data_root, u32::MAX).is_err());
    }

    #[test]
    fn map_id_comes_from_the_map_directory() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        assert_eq!(read_map_id(&data_root, "stormwindjail").unwrap(), 34);
        assert_eq!(read_map_id(&data_root, "azeroth").unwrap(), 0);
        assert!(read_map_id(&data_root, "no_such_map").is_err());
    }
}

fn is_loot_message(message: &ProtocolMessage) -> bool {
    message.is::<CorpseLootable>()
        || message.is::<LootResponse>()
        || message.is::<LootSlotRemoved>()
        || message.is::<LootClosed>()
        || message.is::<LootFailed>()
}

fn receive_loot_message(message: ProtocolMessage) -> Result<LootMessage, String> {
    if message.is::<CorpseLootable>() {
        return Ok(LootMessage::Lootable(decode(message)?));
    }
    if message.is::<LootResponse>() {
        return Ok(LootMessage::Opened(decode(message)?));
    }
    if message.is::<LootSlotRemoved>() {
        return Ok(LootMessage::Removed(decode(message)?));
    }
    if message.is::<LootClosed>() {
        return Ok(LootMessage::Closed(decode(message)?));
    }
    Ok(LootMessage::Failed(decode(message)?))
}

fn decode<M: game_engine_network::WireMessage>(message: ProtocolMessage) -> Result<M, String> {
    message.downcast::<M>().map_err(|_| {
        format!(
            "Invalid protocol message type {}",
            std::any::type_name::<M>()
        )
    })
}

fn auction_message(
    message: ProtocolMessage,
) -> Result<Result<AuctionReply, ProtocolMessage>, String> {
    let reply = if message.is::<AuctionHouseOpened>() {
        AuctionReply::Opened(decode(message)?)
    } else if message.is::<AuctionBrowseResults>() {
        AuctionReply::Browse(decode(message)?)
    } else if message.is::<AuctionSearchResults>() {
        AuctionReply::Search(decode(message)?)
    } else if message.is::<AuctionInventorySnapshot>() {
        AuctionReply::Inventory(decode(message)?)
    } else if message.is::<OwnedAuctionListResponse>() {
        AuctionReply::Owned(decode(message)?)
    } else if message.is::<BidAuctionListResponse>() {
        AuctionReply::Bids(decode(message)?)
    } else if message.is::<AuctionOperationResponse>() {
        AuctionReply::Operation(decode(message)?)
    } else {
        return Ok(Err(message));
    };
    Ok(Ok(reply))
}
